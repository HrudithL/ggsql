//! Execution pipeline: parse TABULATE query → run SQL → produce an in-memory
//! table representation ready for HTML rendering.
//!
//! Two entry points:
//! - [`execute`] — fixture-test harness, runs against a parquet file directly.
//! - [`execute_with_reader`] — CLI / library use, runs against any
//!   [`crate::reader::Reader`].
//!
//! Both funnel into [`build_table_ir`], which owns the column-resolution,
//! formatting, and IR construction logic.

use crate::parser::{tabulate as tab_parser, SourceTree};
use crate::reader::Reader;
use crate::tabulate::ast::{
    FacetSetting, FacetValue, FormatMode, LabelClause, SettingValue, TabulateStmt,
};
use crate::{GgsqlError, Result};
use arrow::array::{Array, ArrayRef, Float64Array, StringArray};
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;
#[cfg(feature = "duckdb")]
use duckdb::{params, Connection};
#[cfg(feature = "parquet")]
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::collections::HashMap;
#[cfg(feature = "parquet")]
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

// ============================================================================
// Column alignment
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColAlign {
    Left,
    Right,
    Center,
}

impl ColAlign {
    pub fn gt_class(self) -> &'static str {
        match self {
            ColAlign::Left => "gt_left",
            ColAlign::Right => "gt_right",
            ColAlign::Center => "gt_center",
        }
    }

    /// Whether to add `font-variant-numeric: tabular-nums` to header/cell style.
    pub fn tabular_nums(self) -> bool {
        self == ColAlign::Right
    }
}

// ============================================================================
// Table IR
// ============================================================================

/// In-memory representation of a rendered TABULATE table, ready for HTML output.
pub struct TableIr {
    /// Column descriptors in display order (hidden columns excluded).
    pub columns: Vec<ColMeta>,
    /// Row data: one entry per row, values in same order as `columns`.
    pub rows: Vec<Vec<String>>,
    /// Per-cell background colour from `SCALE background`. Same shape as
    /// [`Self::rows`]. `None` for cells that no `SCALE` applies to. A
    /// `Some(hex)` value is an uppercase 6-digit colour (`#RRGGBB`); the
    /// HTML writer derives the contrasting foreground colour via
    /// [`super::scale::ideal_fg`].
    pub cell_bg: Vec<Vec<Option<String>>>,
    /// Per-cell text colour from `SCALE foreground`. Same shape as
    /// [`Self::rows`]. `None` for cells with no `SCALE foreground`. A
    /// `Some(hex)` value sets the cell's CSS `color` property.
    pub cell_fg: Vec<Vec<Option<String>>>,
    /// Per-cell font-size from `SCALE size`. Same shape as
    /// [`Self::rows`]. `None` for cells with no `SCALE size`. A
    /// `Some(css)` value (e.g. `"14.5px"`) sets the cell's CSS
    /// `font-size` property.
    pub cell_size: Vec<Vec<Option<String>>>,
    /// Per-cell opacity from `SCALE opacity` (range 0..=1). Same shape
    /// as [`Self::rows`]. `None` for cells with no `SCALE opacity`.
    /// When both a background colour and an opacity are set, the
    /// renderer emits `background-color: rgba(r, g, b, a)`.
    pub cell_opacity: Vec<Vec<Option<f32>>>,
    /// Per-cell style overrides from `HIGHLIGHT` clauses. Same shape as
    /// [`Self::rows`]. Empty `CellStyle` (all `None`) means no highlight
    /// applies. When multiple `HIGHLIGHT`s target the same cell, the
    /// later clause's settings override the earlier (gt's last-writer
    /// semantics).
    pub cell_style: Vec<Vec<CellStyle>>,
    /// Row-groups from a `FACET` clause, in display order. Empty when
    /// the query has no `FACET`; in that case the renderer walks
    /// [`Self::rows`] sequentially.
    pub groups: Vec<RowGroup>,
    /// Table title (gt's `tab_header(title=)`).
    pub title: Option<String>,
    /// Table subtitle (gt's `tab_header(subtitle=)`).
    pub subtitle: Option<String>,
    /// Source-note caption (gt's `tab_source_note()`).
    pub caption: Option<String>,
    /// Index into `columns` of the stub column, if any.
    pub stub_col: Option<usize>,
    /// Header tree forest. Leaves point into `columns` by index; interior
    /// nodes are spanners introduced by `FORMAT SPAN ... AS <id>`. When no
    /// spanners apply, this is a flat sequence of one [`HeaderNode::Column`]
    /// per entry in `columns`.
    pub header_forest: Vec<HeaderNode>,
}

#[derive(Debug, Clone)]
pub struct ColMeta {
    /// Column name (used as `id=` attribute and `headers=` reference).
    pub name: String,
    /// Display label shown in `<th>` (defaults to `name`).
    pub label: String,
    /// True when [`Self::label`] came from a user-supplied `LABEL`
    /// clause; false when it is the default (the column name). The
    /// HTML writer runs user-supplied labels through markup
    /// processing (`^N` superscript, `_N` subscript, smart-text
    /// substitutions) and default labels through plain HTML escaping.
    pub label_is_user: bool,
    /// Alignment.
    pub align: ColAlign,
    /// Explicit column width from `FORMAT <col> SETTING width => '<css>'`.
    /// When any column carries a width, the renderer emits a `<colgroup>`
    /// and switches the table style to `table-layout: fixed`.
    pub width: Option<String>,
    /// True when this column's cell values are already HTML (currently
    /// only set by the scientific-notation formatter `{:num %.Ne}`) and
    /// must not be HTML-escaped by the renderer.
    pub raw_html: bool,
}

/// Per-cell style overrides from `HIGHLIGHT` clauses.
///
/// Each `Option<String>` carries the resolved CSS value (uppercase hex
/// for colours, raw token for `face`). `None` means "not set by any
/// HIGHLIGHT matching this cell".
#[derive(Debug, Clone, Default)]
pub struct CellStyle {
    /// `background => '<css colour>'` → uppercase `#RRGGBB`.
    pub background: Option<String>,
    /// `color => '<css colour>'` → uppercase `#RRGGBB`.
    pub color: Option<String>,
    /// `face => 'bold' | 'italic' | 'normal'` (rendered verbatim).
    pub face: Option<String>,
    /// `size => '<css length>'` (rendered verbatim into `font-size`).
    pub size: Option<String>,
    /// `transform => 'uppercase' | 'lowercase' | 'capitalize' | 'none'`
    /// (rendered verbatim into `text-transform`).
    pub transform: Option<String>,
    /// `decoration => 'underline' | 'line-through' | 'overline' | 'none'`
    /// (rendered verbatim into `text-decoration`).
    pub decoration: Option<String>,
}

impl CellStyle {
    pub fn is_empty(&self) -> bool {
        self.background.is_none()
            && self.color.is_none()
            && self.face.is_none()
            && self.size.is_none()
            && self.transform.is_none()
            && self.decoration.is_none()
    }
}

/// A contiguous row-group in a `FACET`-ed table.
#[derive(Debug, Clone)]
pub struct RowGroup {
    /// Group label rendered in the `<tr class="gt_group_heading_row">`
    /// row before the group's body rows.
    pub name: String,
    /// Indices into [`TableIr::rows`] for the body rows belonging to
    /// this group, in display order.
    pub row_indices: Vec<usize>,
    /// Zero or more summary rows emitted after the body rows (when
    /// `side = 'bottom'`, the default) or before them (`side = 'top'`,
    /// not yet exercised by a fixture).
    pub summary_rows: Vec<SummaryRow>,
    /// Side of the group the summary rows attach to. `"bottom"` (default)
    /// places summaries after the body rows; `"top"` before.
    pub summary_side: String,
}

/// A single summary row computed by an aggregate function over a group.
#[derive(Debug, Clone)]
pub struct SummaryRow {
    /// Text rendered in the stub `<th>` (e.g. `"sum"`, `"Min"`).
    pub label: String,
    /// One entry per column in [`TableIr::columns`]. `None` means render
    /// as the placeholder `—` (em-dash); `Some(text)` is the formatted
    /// aggregate value.
    pub cells: Vec<Option<String>>,
}

/// Node in the header tree. Spanners group child nodes (which may be
/// columns or further spanners) under a parent label.
#[derive(Debug, Clone)]
pub enum HeaderNode {
    /// Leaf node: an actual data column, identified by its index into
    /// [`TableIr::columns`].
    Column { col_idx: usize },
    /// Spanner cell grouping one or more children.
    Spanner {
        /// HTML `id=` attribute (the bareword from `AS <id>`).
        id: String,
        /// Display text rendered inside the spanner cell.
        label: String,
        /// True when [`Self::label`] came from a user-supplied
        /// `LABEL <span_id> => '...'` clause; false when it is the
        /// default (the bareword span ID).
        label_is_user: bool,
        /// Child nodes in display order.
        children: Vec<HeaderNode>,
    },
}

impl HeaderNode {
    /// Number of leaf columns under this node (1 for a column).
    pub fn leaf_count(&self) -> usize {
        match self {
            HeaderNode::Column { .. } => 1,
            HeaderNode::Spanner { children, .. } => children.iter().map(|c| c.leaf_count()).sum(),
        }
    }

    /// Height of the subtree: 0 for a column, 1 + max(child heights) for a
    /// spanner.
    pub fn height(&self) -> usize {
        match self {
            HeaderNode::Column { .. } => 0,
            HeaderNode::Spanner { children, .. } => {
                1 + children.iter().map(|c| c.height()).max().unwrap_or(0)
            }
        }
    }
}

// ============================================================================
// Main entry point
// ============================================================================

/// Parse `query`, execute against `data_path`, apply TABULATE transforms, and
/// return the table IR.
///
/// Fixture-test entry point: registers the parquet file as a DuckDB view
/// and runs the assembled SQL through DuckDB. Only compiled when the
/// `duckdb` feature is enabled (the test harness needs it; the wasm
/// build, which uses `sqlite` only, does not).
#[cfg(feature = "duckdb")]
pub fn execute(query: &str, data_path: &Path) -> Result<TableIr> {
    // 1. Parse the TABULATE statement.
    let source = SourceTree::new(query)?;
    source.validate()?;

    let tab_stmt = tab_parser::parse_tabulate(&source)?;

    // 2. Read original Arrow schema from parquet (before DuckDB normalization)
    //    to preserve dictionary / temporal type info for alignment decisions.
    let orig_schema = read_parquet_schema(data_path)?;

    // 3. Create DuckDB and register the parquet under the correct table name.
    let conn = Connection::open_in_memory()
        .map_err(|e| GgsqlError::ReaderError(format!("DuckDB open failed: {}", e)))?;
    #[cfg(debug_assertions)]
    conn.execute("SET disabled_optimizers TO 'common_subplan'", params![])
        .map_err(|e| GgsqlError::ReaderError(format!("DuckDB SET failed: {}", e)))?;

    let table_name = determine_table_name(&source, &tab_stmt);
    let parquet_path = data_path
        .to_str()
        .ok_or_else(|| GgsqlError::ReaderError("Invalid parquet path".to_string()))?;

    conn.execute(
        &format!(
            "CREATE VIEW {} AS SELECT * FROM read_parquet('{}')",
            table_name, parquet_path
        ),
        params![],
    )
    .map_err(|e| GgsqlError::ReaderError(format!("DuckDB CREATE VIEW failed: {}", e)))?;

    // 4. Build and execute the SQL to retrieve data.
    let sql = build_sql(&source, &tab_stmt, &table_name);

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| GgsqlError::ReaderError(format!("DuckDB prepare failed: {}", e)))?;
    let arrow_result = stmt
        .query_arrow(params![])
        .map_err(|e| GgsqlError::ReaderError(format!("DuckDB query failed: {}", e)))?;
    let schema = arrow_result.get_schema();
    let batches: Vec<_> = arrow_result.collect();
    let combined = if batches.is_empty() {
        arrow::record_batch::RecordBatch::new_empty(schema.clone())
    } else {
        arrow::compute::concat_batches(&schema, &batches)
            .map_err(|e| GgsqlError::ReaderError(format!("concat_batches failed: {}", e)))?
    };

    build_table_ir(&tab_stmt, &combined, Some(&orig_schema))
}

/// Parse `query`, execute the SQL portion through `reader`, apply TABULATE
/// transforms, and return the table IR.
///
/// This is the entry point used by the `ggsql` CLI (and any library caller
/// that wants to render a TABULATE query through a generic `Reader`). It
/// mirrors `Reader::execute` in shape but produces a [`TableIr`] instead of
/// a `Spec`.
pub fn execute_with_reader(reader: &dyn Reader, query: &str) -> Result<TableIr> {
    let source = SourceTree::new(query)?;
    source.validate()?;

    let tab_stmt = tab_parser::parse_tabulate(&source)?;

    // Resolve a table name only for the standalone `TABULATE * FROM <src>`
    // case where there is no SQL portion to run.
    let table_name = determine_table_name(&source, &tab_stmt);
    let sql = build_sql(&source, &tab_stmt, &table_name);

    let df = reader.execute_sql(&sql)?;
    let batch = df.into_inner();

    build_table_ir(&tab_stmt, &batch, None)
}

/// Core IR construction: take a resolved TABULATE statement and a
/// `RecordBatch` of fetched data, apply column selection, alignment,
/// formatting, and hide rules, and emit a [`TableIr`].
///
/// `orig_schema` is an optional hint with more precise type information
/// than the post-execution batch (for parquet sources this carries
/// `Dictionary` types that DuckDB normalises away). Pass `None` when the
/// data came through a generic `Reader`.
fn build_table_ir(
    tab_stmt: &TabulateStmt,
    combined: &RecordBatch,
    orig_schema: Option<&Schema>,
) -> Result<TableIr> {
    // Determine which columns to show and their metadata.
    let requested_cols = &tab_stmt.columns; // empty == all
    let combined_schema = combined.schema();
    let schema_names: Vec<&str> = combined_schema
        .fields()
        .iter()
        .map(|f| f.name().as_str())
        // Synthetic HIGHLIGHT predicate columns are not user-visible.
        .filter(|n| !n.starts_with(HL_COL_PREFIX))
        .collect();

    let display_cols: Vec<&str> = if requested_cols.is_empty() {
        schema_names.clone()
    } else {
        requested_cols
            .iter()
            .map(|c| {
                schema_names
                    .iter()
                    .find(|&&n| n.eq_ignore_ascii_case(c))
                    .copied()
                    .unwrap_or(c.as_str())
            })
            .collect()
    };

    // Hidden columns from FORMAT ... SETTING hide => true
    let hidden: std::collections::HashSet<String> = tab_stmt
        .format_clauses
        .iter()
        .filter(|fc| fc.mode == FormatMode::None)
        .filter(|fc| {
            fc.settings.iter().any(|s| {
                s.key.eq_ignore_ascii_case("hide") && matches!(&s.value, SettingValue::Bool(true))
            })
        })
        .flat_map(|fc| fc.columns.iter().cloned())
        .collect();

    // Stub column from `FORMAT STUB <col> [AS <span_id>]` (at most one).
    let stub_info: Option<(String, Option<String>)> = tab_stmt
        .format_clauses
        .iter()
        .find(|fc| fc.mode == FormatMode::Stub)
        .and_then(|fc| fc.columns.first().map(|c| (c.clone(), fc.span_id.clone())));

    let visible_cols: Vec<&str> = display_cols
        .iter()
        .copied()
        .filter(|c| !hidden.iter().any(|h| h.eq_ignore_ascii_case(c)))
        // The FACET group column drives row partitioning; hide it from
        // the rendered body (it appears as the group heading text).
        .filter(|c| {
            tab_stmt
                .facet
                .as_ref()
                .is_none_or(|f| !f.group_col.eq_ignore_ascii_case(c))
        })
        .collect();

    // When FACET is set but no `FORMAT STUB` is declared, gt's
    // `tab_row_group` still emits an empty stub column on the left edge
    // of the table. We synthesize one as the first ColMeta below.
    let synthetic_stub = tab_stmt.facet.is_some() && stub_info.is_none();

    // gt promotes the stub column to position 0 in the rendered header so it
    // sits left of the spanner block. Mirror that here.
    let visible_cols: Vec<&str> = if let Some((stub_name, _)) = &stub_info {
        if let Some(pos) = visible_cols
            .iter()
            .position(|c| stub_name.eq_ignore_ascii_case(c))
        {
            if pos != 0 {
                let mut reordered = Vec::with_capacity(visible_cols.len());
                reordered.push(visible_cols[pos]);
                for (i, c) in visible_cols.iter().enumerate() {
                    if i != pos {
                        reordered.push(*c);
                    }
                }
                reordered
            } else {
                visible_cols
            }
        } else {
            visible_cols
        }
    } else {
        visible_cols
    };

    // Build a label-rename lookup keyed by lowercase column name AND by
    // lowercase span_id (so `LABEL model_head => '...'` reaches both the
    // stub column and any spanner sharing that id).
    let mut label_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(lc) = &tab_stmt.label {
        for (k, v) in &lc.renames {
            label_map.insert(k.to_ascii_lowercase(), v.clone());
        }
    }

    // Resolve `FORMAT *` (wildcard) to the full set of visible column
    // names. Non-wildcard column lists pass through unchanged.
    let expand_cols = |cols: &[String]| -> Vec<String> {
        if cols.iter().any(|c| c == "*") {
            visible_cols.iter().map(|s| s.to_string()).collect()
        } else {
            cols.to_vec()
        }
    };

    // Per-column width / align overrides from `FORMAT <col> SETTING ...`.
    let mut width_overrides: HashMap<String, String> = HashMap::new();
    let mut align_overrides: HashMap<String, ColAlign> = HashMap::new();
    for fc in &tab_stmt.format_clauses {
        if fc.mode != FormatMode::None {
            continue;
        }
        for s in &fc.settings {
            if s.key.eq_ignore_ascii_case("width") {
                if let SettingValue::String(v) = &s.value {
                    for col in &expand_cols(&fc.columns) {
                        width_overrides.insert(col.to_ascii_lowercase(), v.clone());
                    }
                }
            } else if s.key.eq_ignore_ascii_case("align") {
                if let SettingValue::String(v) = &s.value {
                    let align = match v.to_ascii_lowercase().as_str() {
                        "left" => Some(ColAlign::Left),
                        "right" => Some(ColAlign::Right),
                        "center" => Some(ColAlign::Center),
                        _ => None,
                    };
                    if let Some(a) = align {
                        for col in &expand_cols(&fc.columns) {
                            align_overrides.insert(col.to_ascii_lowercase(), a);
                        }
                    }
                }
            }
        }
    }

    // Build column metadata with alignment and resolved display label.
    let mut columns: Vec<ColMeta> = visible_cols
        .iter()
        .map(|col_name| {
            let auto_align = determine_alignment(col_name, orig_schema, combined);
            let align = align_overrides
                .get(&col_name.to_ascii_lowercase())
                .copied()
                .unwrap_or(auto_align);
            let is_stub = stub_info
                .as_ref()
                .map(|(n, _)| n.eq_ignore_ascii_case(col_name))
                .unwrap_or(false);
            // Stub columns without an explicit label render with empty header
            // text (gt's default for `rowname_col`); regular columns default
            // to the column name.
            let mut label = if is_stub {
                String::new()
            } else {
                col_name.to_string()
            };
            let mut label_is_user = false;
            if let Some(s) = label_map.get(&col_name.to_ascii_lowercase()) {
                label = s.clone();
                label_is_user = true;
            }
            if let Some((stub_name, Some(span_id))) = &stub_info {
                if stub_name.eq_ignore_ascii_case(col_name) {
                    if let Some(s) = label_map.get(&span_id.to_ascii_lowercase()) {
                        label = s.clone();
                        label_is_user = true;
                    }
                }
            }
            let width = width_overrides.get(&col_name.to_ascii_lowercase()).cloned();
            ColMeta {
                name: col_name.to_string(),
                label,
                label_is_user,
                align,
                width,
                raw_html: false,
            }
        })
        .collect();

    // Remember the stub column index. The stub's alignment tracks its
    // underlying data type (gt aligns numeric stubs right, string stubs
    // left) — see `determine_alignment` above.
    let mut stub_col_idx: Option<usize> = None;
    if let Some((stub_name, _)) = &stub_info {
        if let Some(idx) = columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(stub_name))
        {
            stub_col_idx = Some(idx);
        }
    }

    // Insert a synthetic empty stub column for FACET-without-FORMAT-STUB.
    if synthetic_stub {
        columns.insert(
            0,
            ColMeta {
                name: String::new(),
                label: String::new(),
                label_is_user: false,
                align: ColAlign::Left,
                width: None,
                raw_html: false,
            },
        );
        stub_col_idx = Some(0);
    }

    // Collect per-column format-override RHS strings, locales, and direct
    // value substitutions from `FORMAT … SETTING locale => '...'` and
    // `RENAMING <lhs> => '<rhs>'`. The wildcard LHS feeds the cell
    // formatter; `null`, `0`, numeric, and string literals populate
    // dedicated substitution maps consulted before the formatter (spec
    // precedence: literal > null > 0 > `*`).
    use crate::tabulate::ast::RenamingLhs;
    use crate::tabulate::format::{build_format, CellFmt};
    let mut format_overrides: HashMap<String, String> = HashMap::new();
    let mut locale_overrides: HashMap<String, String> = HashMap::new();
    let mut null_subst: HashMap<String, String> = HashMap::new();
    let mut zero_subst: HashMap<String, String> = HashMap::new();
    let mut numeric_substs: HashMap<String, Vec<(f64, String)>> = HashMap::new();
    let mut literal_substs: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for fc in &tab_stmt.format_clauses {
        if fc.mode != FormatMode::None {
            continue;
        }
        for s in &fc.settings {
            if s.key.eq_ignore_ascii_case("locale") {
                if let SettingValue::String(v) = &s.value {
                    for col in &expand_cols(&fc.columns) {
                        locale_overrides.insert(col.to_ascii_lowercase(), v.clone());
                    }
                }
            }
        }
        for r in &fc.renamings {
            match &r.lhs {
                RenamingLhs::Wildcard => {
                    for col in &expand_cols(&fc.columns) {
                        format_overrides.insert(col.to_ascii_lowercase(), r.rhs.clone());
                    }
                }
                RenamingLhs::Null => {
                    let v = smart_text(&r.rhs);
                    for col in &expand_cols(&fc.columns) {
                        null_subst.insert(col.to_ascii_lowercase(), v.clone());
                    }
                }
                RenamingLhs::Zero => {
                    let v = smart_text(&r.rhs);
                    for col in &expand_cols(&fc.columns) {
                        zero_subst.insert(col.to_ascii_lowercase(), v.clone());
                    }
                }
                RenamingLhs::Number(n) => {
                    let v = smart_text(&r.rhs);
                    for col in &expand_cols(&fc.columns) {
                        numeric_substs
                            .entry(col.to_ascii_lowercase())
                            .or_default()
                            .push((*n, v.clone()));
                    }
                }
                RenamingLhs::Literal(lit) => {
                    let v = smart_text(&r.rhs);
                    for col in &expand_cols(&fc.columns) {
                        literal_substs
                            .entry(col.to_ascii_lowercase())
                            .or_default()
                            .push((lit.clone(), v.clone()));
                    }
                }
                RenamingLhs::Identifier(_) => {}
            }
        }
    }

    // 7. Build per-column formatters. Numeric columns without an explicit
    //    override fall back to gt's auto formatter; columns with a
    //    `{:num ...}` or `{:time ...}` override use the dispatched
    //    formatter from `format::build_format`. Time formatters set the
    //    column alignment to `right` to match gt's auto-detection for
    //    temporal data, and numeric scientific (`{:num %.Ne}`) flips the
    //    column's `raw_html` so the renderer keeps the `<sup>` tags.
    enum ColFmt {
        Auto(Box<dyn Fn(Option<f64>) -> String>),
        Num(crate::tabulate::format::NumFn),
        Time(crate::tabulate::format::TimeFn),
        Str(crate::tabulate::format::StringFn),
    }
    let mut formatters: HashMap<String, ColFmt> = HashMap::new();
    #[allow(clippy::needless_range_loop)]
    for cm_idx in 0..columns.len() {
        let name = columns[cm_idx].name.clone();
        let lower = name.to_ascii_lowercase();
        let col_data = match combined.schema().index_of(&name) {
            Ok(i) => combined.column(i).clone(),
            Err(_) => continue,
        };

        if let Some(rhs) = format_overrides.get(&lower) {
            let loc = locale_overrides.get(&lower).map(|s| s.as_str());
            if let Some((fmt, raw_html)) = build_format(rhs, loc) {
                match fmt {
                    CellFmt::Numeric(f) => {
                        if raw_html {
                            columns[cm_idx].raw_html = true;
                        }
                        formatters.insert(name, ColFmt::Num(f));
                        continue;
                    }
                    CellFmt::Time(f) => {
                        // Date/time columns render right-aligned in gt.
                        columns[cm_idx].align = ColAlign::Right;
                        formatters.insert(name, ColFmt::Time(f));
                        continue;
                    }
                    CellFmt::Str(f) => {
                        formatters.insert(name, ColFmt::Str(f));
                        continue;
                    }
                }
            }
        }
        // No override (or unparsable): use the auto float formatter when
        // the underlying array is a Float64Array, otherwise fall through
        // to the default `format_cell` path.
        if let Some(fa) = col_data.as_any().downcast_ref::<Float64Array>() {
            let values: Vec<Option<f64>> = (0..fa.len())
                .map(|i| {
                    if fa.is_null(i) {
                        None
                    } else {
                        Some(fa.value(i))
                    }
                })
                .collect();
            formatters.insert(name, ColFmt::Auto(build_float_formatter(&values)));
        }
    }

    // Build rows.
    let nrows = combined.num_rows();
    let mut rows: Vec<Vec<String>> = (0..nrows)
        .map(|row_idx| {
            visible_cols
                .iter()
                .map(|col_name| {
                    let idx = combined.schema().index_of(col_name).unwrap_or(usize::MAX);
                    if idx == usize::MAX {
                        return "NA".to_string();
                    }
                    let col = combined.column(idx);
                    let col_lower = col_name.to_ascii_lowercase();

                    // Precedence: literal > null > 0 > * / auto.
                    if !col.is_null(row_idx) {
                        if let Some(subs) = literal_substs.get(&col_lower) {
                            if let Some(sa) = col.as_any().downcast_ref::<StringArray>() {
                                let val = sa.value(row_idx);
                                if let Some((_, sub)) = subs.iter().find(|(k, _)| k == val) {
                                    return sub.clone();
                                }
                            }
                        }
                        if let Some(subs) = numeric_substs.get(&col_lower) {
                            if let Some(v) = numeric_to_f64(col, row_idx) {
                                if let Some((_, sub)) = subs.iter().find(|(k, _)| {
                                    (*k - v).abs() <= f64::EPSILON * v.abs().max(1.0)
                                }) {
                                    return sub.clone();
                                }
                            }
                        }
                    }
                    if col.is_null(row_idx) {
                        if let Some(s) = null_subst.get(&col_lower) {
                            return s.clone();
                        }
                    } else if let Some(s) = zero_subst.get(&col_lower) {
                        if let Some(v) = numeric_to_f64(col, row_idx) {
                            if v == 0.0 {
                                return s.clone();
                            }
                        }
                    }

                    match formatters.get(*col_name) {
                        Some(ColFmt::Num(f)) => {
                            if col.is_null(row_idx) {
                                f(None)
                            } else if let Some(v) = numeric_to_f64(col, row_idx) {
                                f(Some(v))
                            } else {
                                format_cell(col, row_idx, None)
                            }
                        }
                        Some(ColFmt::Auto(f)) => format_cell(col, row_idx, Some(f.as_ref())),
                        Some(ColFmt::Time(f)) => {
                            if col.is_null(row_idx) {
                                f(None)
                            } else if let Some(sa) = col.as_any().downcast_ref::<StringArray>() {
                                f(Some(sa.value(row_idx)))
                            } else {
                                // Date32 / Date64 / Time* / Timestamp* — let
                                // Arrow render to its canonical ISO form
                                // (`YYYY-MM-DD`, `HH:MM:SS`,
                                // `YYYY-MM-DD HH:MM:SS`), then re-parse in
                                // the time formatter.
                                match arrow::util::display::array_value_to_string(col, row_idx) {
                                    Ok(s) => f(Some(&s)),
                                    Err(_) => format_cell(col, row_idx, None),
                                }
                            }
                        }
                        Some(ColFmt::Str(f)) => {
                            if col.is_null(row_idx) {
                                f(None)
                            } else if let Some(sa) = col.as_any().downcast_ref::<StringArray>() {
                                f(Some(sa.value(row_idx)))
                            } else {
                                // Non-string types (e.g. dictionary-encoded
                                // strings) — fall back to Arrow's display
                                // and pipe that through the transform.
                                match arrow::util::display::array_value_to_string(col, row_idx) {
                                    Ok(s) => f(Some(&s)),
                                    Err(_) => format_cell(col, row_idx, None),
                                }
                            }
                        }
                        None => format_cell(col, row_idx, None),
                    }
                })
                .collect()
        })
        .collect();

    // Prepend an empty cell for the synthetic stub column when applicable.
    if synthetic_stub {
        for row in rows.iter_mut() {
            row.insert(0, String::new());
        }
    }

    let (title, subtitle, caption) = match &tab_stmt.label {
        Some(LabelClause {
            title,
            subtitle,
            caption,
            ..
        }) => (title.clone(), subtitle.clone(), caption.clone()),
        None => (None, None, None),
    };

    let header_forest = build_header_forest(tab_stmt, &columns, &label_map);

    // Validate spanner-ID uniqueness against column names and other
    // spanners (B8 / Phase 6.3 of POLISHING_PLAN).
    {
        let visible: std::collections::HashSet<String> = columns
            .iter()
            .map(|c| c.name.to_ascii_lowercase())
            .collect();
        let mut seen_spans: std::collections::HashSet<String> = std::collections::HashSet::new();
        for fc in &tab_stmt.format_clauses {
            if fc.mode != FormatMode::Span {
                continue;
            }
            let Some(span_id) = &fc.span_id else {
                continue;
            };
            let key = span_id.to_ascii_lowercase();
            if visible.contains(&key) {
                return Err(GgsqlError::ParseError(format!(
                    "spanner ID '{}' collides with column '{}'",
                    span_id, span_id
                )));
            }
            if !seen_spans.insert(key) {
                return Err(GgsqlError::ParseError(format!(
                    "duplicate spanner ID '{}'",
                    span_id
                )));
            }
        }
    }

    let (cell_bg, cell_fg, cell_size, cell_opacity) =
        build_cell_scale(tab_stmt, &columns, combined);
    let cell_style = build_cell_style(tab_stmt, &columns, combined);

    // Compute row groups + summary rows from the FACET clause.
    let groups = build_row_groups(tab_stmt, &columns, stub_col_idx, combined)?;

    Ok(TableIr {
        columns,
        rows,
        cell_bg,
        cell_fg,
        cell_size,
        cell_opacity,
        cell_style,
        groups,
        title,
        subtitle,
        caption,
        stub_col: stub_col_idx,
        header_forest,
    })
}

/// Build the header tree from `FORMAT SPAN <cols> AS <id>` clauses, starting
/// from a flat sequence of columns and folding each span over the current
/// forest. Children of a `FORMAT SPAN` are matched by name (case-insensitive)
/// against either column names or previously-introduced spanner ids.
///
/// Spans whose children are not contiguous in the current frontier produce a
/// best-effort grouping at the position of the first match; non-matching
/// children are silently dropped. Phases 6-11 may need to revisit this if a
/// fixture requires reordering.
fn build_header_forest(
    tab_stmt: &TabulateStmt,
    columns: &[ColMeta],
    label_map: &std::collections::HashMap<String, String>,
) -> Vec<HeaderNode> {
    // Start with a flat forest of columns in display order.
    let mut nodes: Vec<HeaderNode> = (0..columns.len())
        .map(|i| HeaderNode::Column { col_idx: i })
        .collect();

    // The id by which a node can be referenced in a later FORMAT SPAN: a
    // column's name or a spanner's bareword id.
    fn node_id<'a>(node: &'a HeaderNode, columns: &'a [ColMeta]) -> &'a str {
        match node {
            HeaderNode::Column { col_idx } => columns[*col_idx].name.as_str(),
            HeaderNode::Spanner { id, .. } => id.as_str(),
        }
    }

    for fc in &tab_stmt.format_clauses {
        if fc.mode != FormatMode::Span {
            continue;
        }
        let Some(span_id) = &fc.span_id else {
            continue;
        };
        // Collect indices in `nodes` that match any of the listed child ids.
        let matched: Vec<usize> = (0..nodes.len())
            .filter(|&i| {
                let nid = node_id(&nodes[i], columns);
                fc.columns.iter().any(|c| c.eq_ignore_ascii_case(nid))
            })
            .collect();
        if matched.is_empty() {
            continue;
        }
        // Reorder matched children to match the FORMAT SPAN listing order.
        let mut children: Vec<HeaderNode> = Vec::with_capacity(matched.len());
        for want in &fc.columns {
            if let Some(&i) = matched
                .iter()
                .find(|&&i| node_id(&nodes[i], columns).eq_ignore_ascii_case(want))
            {
                children.push(nodes[i].clone());
            }
        }
        let (label, label_is_user) = match label_map.get(&span_id.to_ascii_lowercase()) {
            Some(s) => (s.clone(), true),
            None => (span_id.clone(), false),
        };
        let spanner = HeaderNode::Spanner {
            id: span_id.clone(),
            label,
            label_is_user,
            children,
        };
        // Remove matched entries and insert the new spanner at the first
        // matched position to preserve original layout order.
        let insert_at = *matched.first().unwrap();
        let matched_set: std::collections::HashSet<usize> = matched.iter().copied().collect();
        let mut new_nodes: Vec<HeaderNode> = Vec::with_capacity(nodes.len() - matched.len() + 1);
        for (i, n) in nodes.into_iter().enumerate() {
            if matched_set.contains(&i) {
                continue;
            }
            new_nodes.push(n);
        }
        let insert_pos = insert_at.min(new_nodes.len());
        new_nodes.insert(insert_pos, spanner);
        nodes = new_nodes;
    }

    nodes
}

/// Compute the per-cell SCALE matrices for any `SCALE <aesthetic>`
/// clauses in `tab_stmt`. Returns four parallel matrices the same shape
/// as `TableIr::rows`:
///
/// * background colour (`SCALE background` → uppercase `#RRGGBB`)
/// * foreground / text colour (`SCALE foreground` → uppercase `#RRGGBB`)
/// * font-size CSS string (`SCALE size` → e.g. `"14.50px"`)
/// * opacity 0..=1 (`SCALE opacity` → f32)
///
/// When multiple SCALE clauses target the same cell + aesthetic the
/// later clause wins (matching gt's last-writer-wins semantics).
#[allow(clippy::type_complexity)]
fn build_cell_scale(
    tab_stmt: &TabulateStmt,
    columns: &[ColMeta],
    combined: &RecordBatch,
) -> (
    Vec<Vec<Option<String>>>,
    Vec<Vec<Option<String>>>,
    Vec<Vec<Option<String>>>,
    Vec<Vec<Option<f32>>>,
) {
    use crate::tabulate::scale::{map_value, resolve_stops};
    let nrows = combined.num_rows();
    let ncols = columns.len();
    let mut bg: Vec<Vec<Option<String>>> = (0..nrows).map(|_| vec![None; ncols]).collect();
    let mut fg: Vec<Vec<Option<String>>> = (0..nrows).map(|_| vec![None; ncols]).collect();
    let mut sz: Vec<Vec<Option<String>>> = (0..nrows).map(|_| vec![None; ncols]).collect();
    let mut op: Vec<Vec<Option<f32>>> = (0..nrows).map(|_| vec![None; ncols]).collect();

    for sc in &tab_stmt.scale_clauses {
        let aesthetic = sc.aesthetic.to_ascii_lowercase();
        let stops = resolve_stops(&sc.palette);
        if stops.is_empty() && !matches!(aesthetic.as_str(), "size" | "opacity") {
            continue;
        }
        let transform = sc.transform.as_deref();

        for target in &sc.target_cols {
            let col_idx = match columns
                .iter()
                .position(|c| c.name.eq_ignore_ascii_case(target))
            {
                Some(i) => i,
                None => continue,
            };
            let data_idx = match combined.schema().index_of(&columns[col_idx].name) {
                Ok(i) => i,
                Err(_) => continue,
            };
            let arr = combined.column(data_idx);

            // Resolve the domain: explicit `FROM (min, max)` or inferred
            // from the column's min/max (skipping nulls / non-finite).
            let domain = if let Some(d) = sc.domain {
                d
            } else {
                let mut lo = f64::INFINITY;
                let mut hi = f64::NEG_INFINITY;
                for r in 0..nrows {
                    if let Some(v) = numeric_to_f64(arr, r) {
                        if v.is_finite() {
                            lo = lo.min(v);
                            hi = hi.max(v);
                        }
                    }
                }
                if lo.is_finite() && hi.is_finite() {
                    (lo, hi)
                } else {
                    continue;
                }
            };

            match aesthetic.as_str() {
                "background" => {
                    for (row_idx, row_cells) in bg.iter_mut().enumerate().take(nrows) {
                        let v = numeric_to_f64(arr, row_idx);
                        row_cells[col_idx] = Some(map_value(v, domain, &stops, transform));
                    }
                }
                "foreground" | "color" => {
                    for (row_idx, row_cells) in fg.iter_mut().enumerate().take(nrows) {
                        let v = numeric_to_f64(arr, row_idx);
                        row_cells[col_idx] = Some(map_value(v, domain, &stops, transform));
                    }
                }
                "size" => {
                    // For `size`, the TO stops are CSS length strings
                    // (e.g. `'12px'`, `'28px'`). Parse to numeric px and
                    // interpolate linearly between adjacent stops via
                    // the normalized t = (v - lo) / (hi - lo).
                    let px_stops: Vec<f32> = match &sc.palette {
                        crate::tabulate::ast::ScalePalette::Stops(s) => {
                            s.iter().map(|s| parse_css_length_px(s)).collect()
                        }
                        crate::tabulate::ast::ScalePalette::Named(_) => continue,
                    };
                    if px_stops.len() < 2 {
                        continue;
                    }
                    for (row_idx, row_cells) in sz.iter_mut().enumerate().take(nrows) {
                        let Some(v) = numeric_to_f64(arr, row_idx) else {
                            continue;
                        };
                        if !v.is_finite() {
                            continue;
                        }
                        let t = scale_t(v, domain.0, domain.1, transform);
                        let interp = interp_f32(&px_stops, t);
                        row_cells[col_idx] = Some(format!("{:.2}px", interp));
                    }
                }
                "opacity" => {
                    // Opacity stops are numbers in [0, 1]. The grammar
                    // currently parses them as strings; try f32.
                    let num_stops: Vec<f32> = match &sc.palette {
                        crate::tabulate::ast::ScalePalette::Stops(s) => {
                            s.iter().filter_map(|x| x.parse::<f32>().ok()).collect()
                        }
                        crate::tabulate::ast::ScalePalette::Named(_) => continue,
                    };
                    if num_stops.len() < 2 {
                        continue;
                    }
                    for (row_idx, row_cells) in op.iter_mut().enumerate().take(nrows) {
                        let Some(v) = numeric_to_f64(arr, row_idx) else {
                            continue;
                        };
                        if !v.is_finite() {
                            continue;
                        }
                        let t = scale_t(v, domain.0, domain.1, transform);
                        row_cells[col_idx] = Some(interp_f32(&num_stops, t).clamp(0.0, 1.0));
                    }
                }
                _ => {}
            }
        }
    }
    (bg, fg, sz, op)
}

/// Parse a CSS length string into pixels. Recognises `'12px'`, `'1.5em'`
/// (16px), `'small'` / `'medium'` / `'large'` keywords. Falls back to
/// 16.0 on parse failure.
fn parse_css_length_px(s: &str) -> f32 {
    let s = s.trim();
    let kw = s.to_ascii_lowercase();
    match kw.as_str() {
        "xx-small" => return 9.0,
        "x-small" => return 10.0,
        "small" => return 13.0,
        "medium" => return 16.0,
        "large" => return 18.0,
        "x-large" => return 24.0,
        "xx-large" => return 32.0,
        _ => {}
    }
    if let Some(num) = s.strip_suffix("px") {
        return num.trim().parse::<f32>().unwrap_or(16.0);
    }
    if let Some(num) = s.strip_suffix("em") {
        return num.trim().parse::<f32>().unwrap_or(1.0) * 16.0;
    }
    if let Some(num) = s.strip_suffix("rem") {
        return num.trim().parse::<f32>().unwrap_or(1.0) * 16.0;
    }
    s.parse::<f32>().unwrap_or(16.0)
}

/// Normalize `v` into [0, 1] across `[lo, hi]`, optionally log10-warped
/// to match `SCALE … VIA log10`.
fn scale_t(v: f64, lo: f64, hi: f64, transform: Option<&str>) -> f32 {
    let (v_t, lo_t, hi_t) = match transform {
        Some(t) if t.eq_ignore_ascii_case("log10") => {
            let logz = |x: f64| if x <= 0.0 { 0.0 } else { x.log10() };
            (logz(v), logz(lo), logz(hi))
        }
        _ => (v, lo, hi),
    };
    if hi_t <= lo_t {
        return 0.0;
    }
    (((v_t - lo_t) / (hi_t - lo_t)).clamp(0.0, 1.0)) as f32
}

/// Piecewise-linear interpolation between adjacent stops at parameter
/// `t` in [0, 1].
fn interp_f32(stops: &[f32], t: f32) -> f32 {
    debug_assert!(!stops.is_empty());
    if stops.len() == 1 {
        return stops[0];
    }
    let t = t.clamp(0.0, 1.0);
    let n = stops.len() - 1;
    let seg_f = t * n as f32;
    let seg = (seg_f.floor() as usize).min(n - 1);
    let sub_t = seg_f - seg as f32;
    stops[seg] * (1.0 - sub_t) + stops[seg + 1] * sub_t
}

/// Build the per-cell highlight-style matrix from `HIGHLIGHT` clauses.
///
/// Each highlight contributes a boolean predicate column (named
/// `__hl_<N>_match`) added in [`build_sql`]. For every row where the
/// predicate is true, the highlight's `SETTING` values are applied to
/// every cell in the highlight's column list. Later highlights override
/// earlier ones on conflict — matching gt's `tab_style` semantics
/// observed in fixture 28.
fn build_cell_style(
    tab_stmt: &TabulateStmt,
    columns: &[ColMeta],
    combined: &RecordBatch,
) -> Vec<Vec<CellStyle>> {
    let nrows = combined.num_rows();
    let ncols = columns.len();
    let mut out: Vec<Vec<CellStyle>> = (0..nrows)
        .map(|_| (0..ncols).map(|_| CellStyle::default()).collect())
        .collect();

    let schema = combined.schema();
    for (hl_idx, hl) in tab_stmt.highlight_clauses.iter().enumerate() {
        let pred_name = format!("{}{}__match", HL_COL_PREFIX, hl_idx);
        let pred_col_idx = match schema.index_of(&pred_name) {
            Ok(i) => i,
            Err(_) => continue,
        };
        let pred_arr = combined.column(pred_col_idx);
        let pred_bool = pred_arr
            .as_any()
            .downcast_ref::<arrow::array::BooleanArray>();

        let target_col_idxs: Vec<usize> = hl
            .columns
            .iter()
            .filter_map(|c| {
                columns
                    .iter()
                    .position(|cm| cm.name.eq_ignore_ascii_case(c))
            })
            .collect();
        if target_col_idxs.is_empty() {
            continue;
        }

        // Pre-resolve each SETTING into the CSS-ready value once.
        let mut bg: Option<String> = None;
        let mut color: Option<String> = None;
        let mut face: Option<String> = None;
        let mut size: Option<String> = None;
        let mut transform: Option<String> = None;
        let mut decoration: Option<String> = None;
        for s in &hl.settings {
            let val = match &s.value {
                crate::tabulate::ast::SettingValue::String(v) => v.clone(),
                crate::tabulate::ast::SettingValue::Number(n) => n.to_string(),
                crate::tabulate::ast::SettingValue::Bool(b) => b.to_string(),
            };
            match s.key.to_ascii_lowercase().as_str() {
                "background" => bg = Some(crate::tabulate::scale::parse_to_hex_upper(&val)),
                "color" => color = Some(crate::tabulate::scale::parse_to_hex_upper(&val)),
                "face" => face = Some(val),
                "size" => size = Some(val),
                "transform" => transform = Some(val),
                "decoration" => decoration = Some(val),
                _ => {}
            }
        }

        for (row_idx, row_cells) in out.iter_mut().enumerate().take(nrows) {
            let matches = match pred_bool {
                Some(b) => !b.is_null(row_idx) && b.value(row_idx),
                None => false,
            };
            if !matches {
                continue;
            }
            for &col_idx in &target_col_idxs {
                let cs = &mut row_cells[col_idx];
                if let Some(b) = &bg {
                    cs.background = Some(b.clone());
                }
                if let Some(c) = &color {
                    cs.color = Some(c.clone());
                }
                if let Some(f) = &face {
                    cs.face = Some(f.clone());
                }
                if let Some(s) = &size {
                    cs.size = Some(s.clone());
                }
                if let Some(t) = &transform {
                    cs.transform = Some(t.clone());
                }
                if let Some(d) = &decoration {
                    cs.decoration = Some(d.clone());
                }
            }
        }
    }
    out
}

/// Resolved view of a [`FacetClause`]'s `SETTING` pairs, normalised into
/// the fields the IR builder actually consumes.
struct FacetView {
    target_cols: Vec<String>,
    aggregates: Vec<String>,
    labels: Vec<String>,
    side: String,
    /// Optional summary-cell format template (e.g. `'{:num %\'.2f}'`).
    /// When set, summary values are rendered via this format instead of
    /// the default `formatC(format="f", digits=K)` per-column max-K.
    fmt: Option<String>,
    /// Optional restriction on which group values get summary rows.
    /// `None` -> every group receives summaries. `Some(list)` ->
    /// only the listed groups (matched case-insensitively against the
    /// rendered group-name).
    groups_filter: Option<Vec<String>>,
}

impl FacetView {
    fn from_settings(settings: &[FacetSetting]) -> Self {
        let mut view = FacetView {
            target_cols: Vec::new(),
            aggregates: Vec::new(),
            labels: Vec::new(),
            side: "bottom".to_string(),
            fmt: None,
            groups_filter: None,
        };
        for s in settings {
            match s.key.to_ascii_lowercase().as_str() {
                "target" => {
                    view.target_cols = match &s.value {
                        FacetValue::IdentList(v) => v.clone(),
                        FacetValue::Identifier(v) => vec![v.clone()],
                        FacetValue::String(v) => vec![v.clone()],
                        FacetValue::StrList(v) => v.clone(),
                        _ => Vec::new(),
                    }
                }
                "aggregate" => {
                    view.aggregates = match &s.value {
                        FacetValue::StrList(v) => v.clone(),
                        FacetValue::IdentList(v) => v.clone(),
                        FacetValue::String(v) => vec![v.clone()],
                        FacetValue::Identifier(v) => vec![v.clone()],
                        _ => Vec::new(),
                    }
                }
                "label" => {
                    view.labels = match &s.value {
                        FacetValue::StrList(v) => v.clone(),
                        FacetValue::IdentList(v) => v.clone(),
                        FacetValue::String(v) => vec![v.clone()],
                        FacetValue::Identifier(v) => vec![v.clone()],
                        _ => Vec::new(),
                    }
                }
                "side" => {
                    view.side = match &s.value {
                        FacetValue::String(v) => v.clone(),
                        FacetValue::Identifier(v) => v.clone(),
                        _ => view.side,
                    }
                }
                "fmt" => {
                    view.fmt = match &s.value {
                        FacetValue::String(v) => Some(v.clone()),
                        _ => None,
                    }
                }
                "groups" => {
                    view.groups_filter = match &s.value {
                        FacetValue::StrList(v) => Some(v.clone()),
                        FacetValue::IdentList(v) => Some(v.clone()),
                        FacetValue::String(v) => Some(vec![v.clone()]),
                        FacetValue::Identifier(v) => Some(vec![v.clone()]),
                        _ => None,
                    }
                }
                _ => {}
            }
        }
        view
    }
}

/// Apply an aggregate function to a numeric slice. Returns `None` for an
/// empty slice or an unrecognised function name. `sd` is the sample
/// standard deviation (divisor `n-1`).
fn compute_aggregate(values: &[f64], name: &str) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let n = values.len() as f64;
    match name.to_ascii_lowercase().as_str() {
        "sum" => Some(values.iter().sum()),
        "min" => values.iter().copied().reduce(f64::min),
        "max" => values.iter().copied().reduce(f64::max),
        "avg" => Some(values.iter().sum::<f64>() / n),
        "median" => {
            let mut v: Vec<f64> = values.to_vec();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let m = v.len();
            if m % 2 == 1 {
                Some(v[m / 2])
            } else {
                Some((v[m / 2 - 1] + v[m / 2]) / 2.0)
            }
        }
        "sd" | "stdev" | "stddev" => {
            if values.len() < 2 {
                return Some(0.0);
            }
            let mean = values.iter().sum::<f64>() / n;
            let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
            Some(var.sqrt())
        }
        _ => None,
    }
}

/// Format a summary-row aggregate value to a fixed number of decimal
/// places without thousands separators. gt's `tab_summary_rows()` default
/// rendering picks a per-column decimal count equal to the maximum
/// decimals needed by any value in that column (computed by
/// [`per_column_summary_nsmall`]) and prints each value to that fixed
/// width with `formatC(format = "f", digits = nsmall)`. Thousands
/// separators are intentionally omitted; the captured fixture for
/// example 34 has values up to 6,236 rendered as `6236.00`.
fn format_summary_value(v: f64, nsmall: usize) -> String {
    if !v.is_finite() {
        return "NA".to_string();
    }
    format!("{:.*}", nsmall, v)
}

/// Number of decimals needed to losslessly print `v` at up to
/// [`MAX_SUMMARY_DECIMALS`] places (R's default for `format()` /
/// `formatC()` is 7 significant figures; we cap at 6 trailing decimals,
/// which is more than enough for any summary value we have seen).
fn summary_decimals_for(v: f64) -> usize {
    const MAX: usize = 6;
    if !v.is_finite() {
        return 0;
    }
    for k in 0..=MAX {
        let scaled = v * 10f64.powi(k as i32);
        if (scaled - scaled.round()).abs() < 1e-6 * scaled.abs().max(1.0) {
            return k;
        }
    }
    MAX
}

/// Partition rows into [`RowGroup`]s using the FACET clause's group
/// column. Groups appear in discovery order (the order in which their
/// first row is encountered) and each group collects all rows whose
/// group-column value equals the group name.
fn build_row_groups(
    tab_stmt: &TabulateStmt,
    columns: &[ColMeta],
    stub_col_idx: Option<usize>,
    combined: &RecordBatch,
) -> Result<Vec<RowGroup>> {
    let Some(facet) = &tab_stmt.facet else {
        return Ok(Vec::new());
    };
    let nrows = combined.num_rows();
    let schema = combined.schema();
    let group_idx = match schema.index_of(&facet.group_col) {
        Ok(i) => i,
        Err(_) => return Ok(Vec::new()),
    };
    let group_arr = combined.column(group_idx);

    let group_values: Vec<String> = (0..nrows)
        .map(|i| format_cell(group_arr, i, None))
        .collect();

    let mut order: Vec<String> = Vec::new();
    let mut row_indices_by: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, v) in group_values.iter().enumerate() {
        if !row_indices_by.contains_key(v) {
            order.push(v.clone());
        }
        row_indices_by.entry(v.clone()).or_default().push(i);
    }

    let view = FacetView::from_settings(&facet.settings);

    // Validate `groups => [...]`: every named group must exist in the
    // data. Unknown names are a hard error.
    if let Some(filter) = &view.groups_filter {
        for g in filter {
            if !order.iter().any(|name| name.eq_ignore_ascii_case(g)) {
                return Err(GgsqlError::ParseError(format!(
                    "FACET groups: '{}' is not a value of grouping column '{}'",
                    g, facet.group_col
                )));
            }
        }
    }

    // Map target column names → column index in `columns` for cell placement.
    let target_idxs: Vec<(String, usize)> = view
        .target_cols
        .iter()
        .filter_map(|t| {
            columns
                .iter()
                .position(|c| c.name.eq_ignore_ascii_case(t))
                .map(|i| (t.clone(), i))
        })
        .collect();

    // Map each target column to its arrow data column for aggregate compute.
    let target_data: Vec<(usize, ArrayRef)> = view
        .target_cols
        .iter()
        .filter_map(|t| {
            let col_idx = columns
                .iter()
                .position(|c| c.name.eq_ignore_ascii_case(t))?;
            let data_idx = schema.index_of(t).ok()?;
            Some((col_idx, combined.column(data_idx).clone()))
        })
        .collect();

    // gt's `tab_summary_rows()` default rendering formats every summary
    // cell in a target column to the same number of decimal places: the
    // maximum needed by any aggregate value in that column across all
    // groups. Precompute that here so each row renders consistently.
    let mut nsmall_by_col: HashMap<usize, usize> = HashMap::new();
    for (col_idx, arr) in &target_data {
        let mut max_nsmall = 0usize;
        for group_name in &order {
            let Some(rows) = row_indices_by.get(group_name) else {
                continue;
            };
            let vals: Vec<f64> = rows
                .iter()
                .filter_map(|&r| numeric_to_f64(arr, r))
                .collect();
            for agg in &view.aggregates {
                if let Some(v) = compute_aggregate(&vals, agg) {
                    max_nsmall = max_nsmall.max(summary_decimals_for(v));
                }
            }
        }
        nsmall_by_col.insert(*col_idx, max_nsmall);
    }

    // When the FACET `SETTING fmt => '<template>'` is provided, build a
    // numeric formatter from it and apply it to every summary cell
    // (mirrors gt's `summary_rows(fmt = ~ fmt_number(...))`).
    let summary_fmt: Option<crate::tabulate::format::NumFn> = view.fmt.as_deref().and_then(|t| {
        crate::tabulate::format::build_format(t, None).and_then(|(f, _)| match f {
            crate::tabulate::format::CellFmt::Numeric(nf) => Some(nf),
            _ => None,
        })
    });

    let rg: Vec<RowGroup> = order
        .into_iter()
        .map(|name| {
            let row_indices = row_indices_by.remove(&name).unwrap();

            // `groups => [...]` restricts which group values get summary
            // rows. When set and this group's name is not in the list,
            // emit an empty summary_rows vec.
            let group_in_filter = match &view.groups_filter {
                Some(filter) => filter.iter().any(|g| g.eq_ignore_ascii_case(&name)),
                None => true,
            };

            let summary_rows: Vec<SummaryRow> = if !group_in_filter {
                Vec::new()
            } else {
                view.aggregates
                    .iter()
                    .enumerate()
                    .map(|(agg_idx, agg)| {
                        // Per-target-column aggregate values keyed by column index.
                        let mut values_by_col: HashMap<usize, Option<f64>> = HashMap::new();
                        for (col_idx, arr) in &target_data {
                            let vals: Vec<f64> = row_indices
                                .iter()
                                .filter_map(|&r| numeric_to_f64(arr, r))
                                .collect();
                            values_by_col.insert(*col_idx, compute_aggregate(&vals, agg));
                        }

                        let cells: Vec<Option<String>> = (0..columns.len())
                            .map(|col_idx| {
                                if Some(col_idx) == stub_col_idx {
                                    return None; // stub holds the label, set below
                                }
                                if !target_idxs.iter().any(|(_, i)| *i == col_idx) {
                                    return None; // non-target → em-dash
                                }
                                values_by_col.get(&col_idx).copied().flatten().map(|v| {
                                    if let Some(ref f) = summary_fmt {
                                        f(Some(v))
                                    } else {
                                        let nsmall = *nsmall_by_col.get(&col_idx).unwrap_or(&0);
                                        format_summary_value(v, nsmall)
                                    }
                                })
                            })
                            .collect();

                        let label = if agg_idx < view.labels.len() {
                            view.labels[agg_idx].clone()
                        } else {
                            agg.clone()
                        };

                        SummaryRow { label, cells }
                    })
                    .collect()
            };

            RowGroup {
                name,
                row_indices,
                summary_rows,
                summary_side: view.side.clone(),
            }
        })
        .collect();
    Ok(rg)
}

// ============================================================================
// Helpers
// ============================================================================

/// Read the Apache Arrow schema embedded in a Parquet file without loading data.
#[cfg(feature = "parquet")]
fn read_parquet_schema(path: &Path) -> Result<Arc<Schema>> {
    let file = File::open(path).map_err(|e| {
        GgsqlError::ReaderError(format!("Cannot open parquet '{}': {}", path.display(), e))
    })?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| GgsqlError::ReaderError(format!("Parquet read failed: {}", e)))?;
    Ok(builder.schema().clone())
}

/// Pick a DuckDB table name to register the parquet view as. When the
/// query has a SQL portion, use the first table referenced in its `FROM`
/// (the actual underlying table — the TABULATE `FROM` clause may name a
/// CTE that we'd never want to override). Otherwise use the TABULATE
/// `FROM` clause, falling back to `__data__`.
fn determine_table_name(source: &SourceTree<'_>, tab_stmt: &TabulateStmt) -> String {
    if let Some(name) = tab_parser::extract_sql_from_table(source) {
        return name;
    }
    if let Some(ref s) = tab_stmt.from_source {
        return s.clone();
    }
    "__data__".to_string()
}

/// Build the SQL to execute. If there is a SQL portion, use it unchanged.
/// Otherwise build `SELECT * FROM <source>`.
///
/// Special case: a SQL portion that ends in a bare `WITH … AS (…)` with no
/// trailing `SELECT`/`FROM` body is treated as CTE definitions only, and we
/// append `SELECT * FROM <table_name>` so DuckDB has something to execute.
/// This lets a query say "define CTEs, then TABULATE … FROM <cte>".
///
/// When the TABULATE statement contains `HIGHLIGHT … FILTER …` clauses, the
/// returned SQL wraps the base query in a subquery and appends one boolean
/// projection column per highlight (`__hl_<N>_match`). The IR builder reads
/// those columns to populate the per-cell style matrix and then strips them
/// from the visible column list.
fn build_sql(source: &SourceTree<'_>, tab_stmt: &TabulateStmt, table_name: &str) -> String {
    let root = source.root();
    let sql_nodes = source.find_nodes(&root, "(sql_portion) @s");
    // (with_prefix, body): when the SQL portion is a CTE-only query, the
    // body is the synthetic `SELECT * FROM <table>` we append; otherwise
    // the body is the whole SQL portion and `with_prefix` is empty.
    let (with_prefix, body) = if let Some(sql_node) = sql_nodes.into_iter().next() {
        let text = source
            .get_text(&sql_node)
            .trim()
            .trim_end_matches(';')
            .trim()
            .to_string();
        if sql_portion_is_cte_only(&sql_node) {
            (text, format!("SELECT * FROM {}", table_name))
        } else {
            (String::new(), text)
        }
    } else {
        (String::new(), format!("SELECT * FROM {}", table_name))
    };
    if tab_stmt.highlight_clauses.is_empty() {
        if with_prefix.is_empty() {
            return body;
        }
        return format!("{} {}", with_prefix, body);
    }
    let mut select = String::from("SELECT __t.*");
    for (i, hl) in tab_stmt.highlight_clauses.iter().enumerate() {
        select.push_str(&format!(
            ", ({}) AS {}{}__match",
            hl.filter, HL_COL_PREFIX, i
        ));
    }
    select.push_str(&format!(" FROM ({}) __t", body));
    if with_prefix.is_empty() {
        select
    } else {
        format!("{} {}", with_prefix, select)
    }
}

/// True when the `sql_portion`'s last `sql_statement` is a `with_statement`
/// that has no trailing `select_statement`/`from_statement` body — i.e. the
/// user wrote only CTE definitions and expects TABULATE to drive the final
/// SELECT.
fn sql_portion_is_cte_only(sql_portion: &tree_sitter::Node<'_>) -> bool {
    let last_stmt = (0..sql_portion.named_child_count())
        .rev()
        .filter_map(|i| sql_portion.named_child(i as u32))
        .find(|n| n.kind() == "sql_statement");
    let Some(stmt) = last_stmt else { return false };
    let with = (0..stmt.named_child_count())
        .filter_map(|i| stmt.named_child(i as u32))
        .find(|n| n.kind() == "with_statement");
    let Some(with) = with else { return false };
    !(0..with.named_child_count())
        .filter_map(|i| with.named_child(i as u32))
        .any(|n| matches!(n.kind(), "select_statement" | "from_statement"))
}

/// Prefix for synthetic boolean projection columns added by `build_sql` when
/// the query has `HIGHLIGHT` clauses.
pub(crate) const HL_COL_PREFIX: &str = "__hl_";

/// Determine alignment for a column given the original Arrow schema (if
/// available — e.g. when reading directly from parquet) and the actual data
/// column from the query result.
fn determine_alignment(
    col_name: &str,
    orig_schema: Option<&Schema>,
    batch: &arrow::record_batch::RecordBatch,
) -> ColAlign {
    // Check original Arrow schema type (more precise than the post-execution
    // schema; e.g. parquet preserves Dictionary that DuckDB collapses).
    if let Some(orig_schema) = orig_schema {
        if let Ok(orig_idx) = orig_schema.index_of(col_name) {
            let orig_type = orig_schema.field(orig_idx).data_type();
            match orig_type {
                DataType::Dictionary(_, _) => return ColAlign::Center,
                DataType::Float16
                | DataType::Float32
                | DataType::Float64
                | DataType::Int8
                | DataType::Int16
                | DataType::Int32
                | DataType::Int64
                | DataType::UInt8
                | DataType::UInt16
                | DataType::UInt32
                | DataType::UInt64
                | DataType::Decimal128(_, _)
                | DataType::Decimal256(_, _) => return ColAlign::Right,
                DataType::Date32 | DataType::Date64 => return ColAlign::Right,
                DataType::Time32(_) | DataType::Time64(_) => return ColAlign::Right,
                DataType::Timestamp(_, _) => return ColAlign::Right,
                _ => {}
            }
        }
    }

    // Check the query result type.
    let batch_schema = batch.schema();
    if let Ok(idx) = batch_schema.index_of(col_name) {
        let dt = batch_schema.field(idx).data_type();
        match dt {
            DataType::Float16
            | DataType::Float32
            | DataType::Float64
            | DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::Int64
            | DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32
            | DataType::UInt64
            | DataType::Decimal128(_, _)
            | DataType::Decimal256(_, _) => return ColAlign::Right,
            DataType::Date32 | DataType::Date64 => return ColAlign::Right,
            DataType::Time32(_) | DataType::Time64(_) => return ColAlign::Right,
            DataType::Timestamp(_, _) => return ColAlign::Right,
            _ => {}
        }

        // For string columns: detect date/time by pattern matching on values.
        if matches!(dt, DataType::Utf8 | DataType::LargeUtf8) {
            if let Some(align) = detect_string_alignment(batch.column(idx)) {
                return align;
            }
        }
    }

    ColAlign::Left
}

/// Inspect a string column's values to detect date/time-like patterns.
/// Returns Some(ColAlign::Right) if all non-null values look like dates/times,
/// None otherwise.
fn detect_string_alignment(col: &ArrayRef) -> Option<ColAlign> {
    let sa = col.as_any().downcast_ref::<StringArray>()?;
    let mut seen_any = false;
    for i in 0..sa.len() {
        if sa.is_null(i) {
            continue;
        }
        let v = sa.value(i);
        seen_any = true;
        if !is_date_or_time_string(v) {
            return None;
        }
    }
    if seen_any {
        Some(ColAlign::Right)
    } else {
        None
    }
}

/// Check if a string value looks like a date, time, or datetime.
fn is_date_or_time_string(s: &str) -> bool {
    let bytes = s.as_bytes();
    let n = bytes.len();
    // ISO date: YYYY-MM-DD (10 chars)
    if n == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(|b| b.is_ascii_digit())
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[8..].iter().all(|b| b.is_ascii_digit())
    {
        return true;
    }
    // Time HH:MM or HH:MM:SS
    if (n == 5 || n == 8)
        && bytes[2] == b':'
        && bytes[..2].iter().all(|b| b.is_ascii_digit())
        && bytes[3..5].iter().all(|b| b.is_ascii_digit())
    {
        return true;
    }
    // Datetime YYYY-MM-DD HH:MM or YYYY-MM-DD HH:MM:SS
    if n >= 16
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b' '
        && bytes[13] == b':'
        && bytes[..4].iter().all(|b| b.is_ascii_digit())
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[8..10].iter().all(|b| b.is_ascii_digit())
        && bytes[11..13].iter().all(|b| b.is_ascii_digit())
        && bytes[14..16].iter().all(|b| b.is_ascii_digit())
    {
        return true;
    }
    false
}

// ============================================================================
// Numeric formatting
// ============================================================================

/// Build a formatter function for a float column that matches gt's `fmt_auto()`.
///
/// Rules (derived from gt 1.3 / R's format() behavior):
/// 1. All integer-valued → format as integer (no decimals).
/// 2. Otherwise use 3 decimal places.
/// 3. If fixed width of the largest value (with 3 dp) > 9 chars → scientific.
fn build_float_formatter(values: &[Option<f64>]) -> Box<dyn Fn(Option<f64>) -> String> {
    let non_null: Vec<f64> = values
        .iter()
        .filter_map(|v| *v)
        .filter(|v| v.is_finite())
        .map(|v| round_to_sig_figs(v, 7))
        .collect();

    if non_null.is_empty() {
        return Box::new(|v: Option<f64>| match v {
            None => "NA".to_string(),
            Some(x) => format!("{}", x),
        });
    }

    // Check all integer-valued.
    let all_integer = non_null
        .iter()
        .all(|v| (v - v.round()).abs() < 1e-9 * v.abs().max(1.0));

    let max_abs = non_null.iter().map(|v| v.abs()).fold(0f64, f64::max);

    if all_integer {
        // gt's R-side default rendering of a numeric vector falls back to
        // `format()`, which picks scientific notation when the shortest
        // representation of the largest value is shorter as `X.XXXe+YY`
        // than as fixed. R's threshold for an *integer*-valued column
        // (e.g. `c(650000, 1.4e9)`) is around `1e6`, *provided* each
        // value is clean to ~4 significant figures — that's the regime
        // where scientific buys you space without losing information.
        // Columns like SP500 daily volume (`4378680000`) carry 7+ sig
        // figs and need the full integer string; gt renders them
        // verbatim. We approximate gt by gating scientific notation on
        // "all values round to 4 sf without change".
        let scientific_friendly = max_abs >= 1e6
            && non_null.iter().all(|v| {
                let r = round_to_sig_figs(*v, 4);
                (r - v).abs() <= 1e-6 * v.abs().max(1.0)
            });
        if scientific_friendly {
            return Box::new(|v: Option<f64>| match v {
                None => "NA".to_string(),
                Some(x) if x.is_nan() => "NA".to_string(),
                Some(x) => format_scientific_3dp(x),
            });
        }
        return Box::new(|v: Option<f64>| match v {
            None => "NA".to_string(),
            Some(x) if x.is_nan() => "NA".to_string(),
            Some(x) => format!("{}", x.round() as i64),
        });
    }

    // Determine: scientific or fixed?
    let fixed_width_of_max = format!("{:.3}", max_abs).len();
    let use_scientific = fixed_width_of_max > 9;

    // Pick the minimum decimal-place count (1..=3) that represents every
    // value losslessly. gt's auto formatter behaves similarly: 12.0 and
    // 12.345 in the same column → 3 dp, but 4.27 and 4.75 → 2 dp.
    let dp = if use_scientific {
        3
    } else {
        (1..=3)
            .find(|&d| {
                let scale = 10f64.powi(d);
                non_null.iter().all(|v| {
                    let rounded = (v * scale).round() / scale;
                    (rounded - v).abs() <= 1e-9 * v.abs().max(1.0)
                })
            })
            .unwrap_or(3) as usize
    };

    Box::new(move |v: Option<f64>| match v {
        None => "NA".to_string(),
        Some(x) if x.is_nan() => "NA".to_string(),
        Some(x) if use_scientific => format_scientific_3dp(x),
        Some(x) => format!("{:.*}", dp, round_to_sig_figs(x, 7)),
    })
}

/// Round `x` to `sig` significant figures, matching R's `signif()` for
/// positive `sig` and finite, non-zero `x`. Used by the auto formatter
/// before it picks a uniform decimal-place count so columns like SP500
/// `adj_close` (where the raw doubles carry trailing 1-bit noise such as
/// `2044.8101`) round to gt's displayed precision (`2044.81`).
fn round_to_sig_figs(x: f64, sig: u32) -> f64 {
    if x == 0.0 || !x.is_finite() || sig == 0 {
        return x;
    }
    let d = x.abs().log10().floor() as i32 + 1;
    let power = sig as i32 - d;
    let factor = 10f64.powi(power);
    (x * factor).round() / factor
}

/// Format a float in scientific notation with 3 mantissa decimal places,
/// matching R/gt's format: `X.XXXe+YY` or `X.XXXe-YY`.
fn format_scientific_3dp(v: f64) -> String {
    if v == 0.0 {
        return "0.000e+00".to_string();
    }
    let abs_v = v.abs();
    let exp = abs_v.log10().floor() as i32;
    let mantissa = v / 10f64.powi(exp);
    if exp >= 0 {
        format!("{:.3}e+{:02}", mantissa, exp)
    } else {
        format!("{:.3}e-{:02}", mantissa, exp.unsigned_abs())
    }
}

/// Format a single cell value as a string.
fn format_cell(
    col: &ArrayRef,
    row: usize,
    formatter: Option<&dyn Fn(Option<f64>) -> String>,
) -> String {
    if col.is_null(row) {
        return "NA".to_string();
    }

    // Numeric column with custom formatter: convert through f64.
    if let Some(fmt) = formatter {
        if let Some(v) = numeric_to_f64(col, row) {
            return fmt(Some(v));
        }
    }

    // String column.
    if let Some(sa) = col.as_any().downcast_ref::<StringArray>() {
        return if sa.is_null(row) {
            "NA".to_string()
        } else {
            sa.value(row).to_string()
        };
    }

    // Fallback: use Arrow's display.
    arrow::util::display::array_value_to_string(col, row).unwrap_or_else(|_| "NA".to_string())
}

/// Try to read `(col, row)` as an `f64` regardless of integer/float width.
/// Returns `None` for non-numeric types or null cells.
fn numeric_to_f64(col: &ArrayRef, row: usize) -> Option<f64> {
    use arrow::array::{
        Float32Array, Int16Array, Int32Array, Int64Array, Int8Array, UInt16Array, UInt32Array,
        UInt64Array, UInt8Array,
    };
    if col.is_null(row) {
        return None;
    }
    let any = col.as_any();
    if let Some(a) = any.downcast_ref::<Float64Array>() {
        return Some(a.value(row));
    }
    if let Some(a) = any.downcast_ref::<Float32Array>() {
        return Some(a.value(row) as f64);
    }
    if let Some(a) = any.downcast_ref::<Int64Array>() {
        return Some(a.value(row) as f64);
    }
    if let Some(a) = any.downcast_ref::<Int32Array>() {
        return Some(a.value(row) as f64);
    }
    if let Some(a) = any.downcast_ref::<Int16Array>() {
        return Some(a.value(row) as f64);
    }
    if let Some(a) = any.downcast_ref::<Int8Array>() {
        return Some(a.value(row) as f64);
    }
    if let Some(a) = any.downcast_ref::<UInt64Array>() {
        return Some(a.value(row) as f64);
    }
    if let Some(a) = any.downcast_ref::<UInt32Array>() {
        return Some(a.value(row) as f64);
    }
    if let Some(a) = any.downcast_ref::<UInt16Array>() {
        return Some(a.value(row) as f64);
    }
    if let Some(a) = any.downcast_ref::<UInt8Array>() {
        return Some(a.value(row) as f64);
    }
    None
}

// `{:num <printf>}` and `{:time <strftime>}` formatters live in
// `super::format`; this module dispatches through `format::build_format`.

/// gt's `sub_missing()` (and friends) run their replacement text through
/// the same processor used for markdown labels, which collapses `---` to
/// an em-dash, `--` to an en-dash, and `...` to a horizontal ellipsis. We
/// apply the same conversion to the RHS of `RENAMING null|0|'literal' =>
/// '...'` so substitution output matches gt byte-for-byte.
fn smart_text(s: &str) -> String {
    s.replace("---", "\u{2014}")
        .replace("--", "\u{2013}")
        .replace("...", "\u{2026}")
}
