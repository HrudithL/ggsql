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
use crate::tabulate::ast::{FormatMode, LabelClause, SettingValue, TabulateStmt};
use crate::{GgsqlError, Result};
use arrow::array::{Array, ArrayRef, Float64Array, StringArray};
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;
use duckdb::{params, Connection};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::collections::HashMap;
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
    /// Alignment.
    pub align: ColAlign,
    /// Explicit column width from `FORMAT <col> SETTING width => '<css>'`.
    /// When any column carries a width, the renderer emits a `<colgroup>`
    /// and switches the table style to `table-layout: fixed`.
    pub width: Option<String>,
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
        .collect();

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
                    for col in &fc.columns {
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
                        for col in &fc.columns {
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
            if let Some(s) = label_map.get(&col_name.to_ascii_lowercase()) {
                label = s.clone();
            }
            if let Some((stub_name, Some(span_id))) = &stub_info {
                if stub_name.eq_ignore_ascii_case(col_name) {
                    if let Some(s) = label_map.get(&span_id.to_ascii_lowercase()) {
                        label = s.clone();
                    }
                }
            }
            let width = width_overrides.get(&col_name.to_ascii_lowercase()).cloned();
            ColMeta {
                name: col_name.to_string(),
                label,
                align,
                width,
            }
        })
        .collect();

    // Stub columns: force alignment to left and remember the index.
    let mut stub_col_idx: Option<usize> = None;
    if let Some((stub_name, _)) = &stub_info {
        if let Some(idx) = columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(stub_name))
        {
            columns[idx].align = ColAlign::Left;
            stub_col_idx = Some(idx);
        }
    }

    // Collect per-column number-format overrides from `RENAMING * => '...'`.
    // Only the wildcard LHS with a `{:num <spec>}` body is recognised here;
    // richer renaming support lands in later phases.
    use crate::tabulate::ast::RenamingLhs;
    let mut num_format_overrides: HashMap<String, String> = HashMap::new();
    for fc in &tab_stmt.format_clauses {
        if fc.mode != FormatMode::None {
            continue;
        }
        for r in &fc.renamings {
            if matches!(r.lhs, RenamingLhs::Wildcard) {
                for col in &fc.columns {
                    num_format_overrides.insert(col.to_ascii_lowercase(), r.rhs.clone());
                }
            }
        }
    }

    // 7. Format cell values.
    // Per-column formatters for numeric columns. If a `RENAMING * => '{:num ...}'`
    // override exists, use the spec-driven formatter; otherwise fall back to
    // gt's auto formatter.
    type Fmt = Box<dyn Fn(Option<f64>) -> String>;
    let formatters: HashMap<String, Fmt> = columns
        .iter()
        .filter_map(|cm| {
            let idx = combined.schema().index_of(&cm.name).ok()?;
            let col = combined.column(idx);
            let override_spec = num_format_overrides
                .get(&cm.name.to_ascii_lowercase())
                .cloned();
            if let Some(spec) = override_spec {
                if let Some(f) = build_num_format(&spec) {
                    return Some((cm.name.clone(), f));
                }
            }
            if let Some(fa) = col.as_any().downcast_ref::<Float64Array>() {
                let values: Vec<Option<f64>> = (0..fa.len())
                    .map(|i| {
                        if fa.is_null(i) {
                            None
                        } else {
                            Some(fa.value(i))
                        }
                    })
                    .collect();
                Some((cm.name.clone(), build_float_formatter(&values)))
            } else {
                None
            }
        })
        .collect();

    // Build rows.
    let nrows = combined.num_rows();
    let rows: Vec<Vec<String>> = (0..nrows)
        .map(|row_idx| {
            visible_cols
                .iter()
                .map(|col_name| {
                    let idx = combined.schema().index_of(col_name).unwrap_or(usize::MAX);
                    if idx == usize::MAX {
                        return "NA".to_string();
                    }
                    let col = combined.column(idx);
                    format_cell(col, row_idx, formatters.get(*col_name).map(|f| f.as_ref()))
                })
                .collect()
        })
        .collect();

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

    Ok(TableIr {
        columns,
        rows,
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
        let label = label_map
            .get(&span_id.to_ascii_lowercase())
            .cloned()
            .unwrap_or_else(|| span_id.clone());
        let spanner = HeaderNode::Spanner {
            id: span_id.clone(),
            label,
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

// ============================================================================
// Helpers
// ============================================================================

/// Read the Apache Arrow schema embedded in a Parquet file without loading data.
fn read_parquet_schema(path: &Path) -> Result<Arc<Schema>> {
    let file = File::open(path).map_err(|e| {
        GgsqlError::ReaderError(format!("Cannot open parquet '{}': {}", path.display(), e))
    })?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| GgsqlError::ReaderError(format!("Parquet read failed: {}", e)))?;
    Ok(builder.schema().clone())
}

/// Pick a DuckDB table name: prefer the TABULATE `FROM <source>`, then the
/// SQL portion's first table reference.
fn determine_table_name(source: &SourceTree<'_>, tab_stmt: &TabulateStmt) -> String {
    if let Some(ref s) = tab_stmt.from_source {
        return s.clone();
    }
    tab_parser::extract_sql_from_table(source).unwrap_or_else(|| "__data__".to_string())
}

/// Build the SQL to execute. If there is a SQL portion, use it unchanged.
/// Otherwise build `SELECT * FROM <source>`.
fn build_sql(source: &SourceTree<'_>, _tab_stmt: &TabulateStmt, table_name: &str) -> String {
    let root = source.root();
    // Check for a sql_portion node.
    let sql_nodes = source.find_nodes(&root, "(sql_portion) @s");
    if let Some(sql_node) = sql_nodes.into_iter().next() {
        return source.get_text(&sql_node);
    }
    // Standalone TABULATE — generate SELECT.
    format!("SELECT * FROM {}", table_name)
}

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

    if all_integer {
        return Box::new(|v: Option<f64>| match v {
            None => "NA".to_string(),
            Some(x) if x.is_nan() => "NA".to_string(),
            Some(x) => format!("{}", x.round() as i64),
        });
    }

    // Determine: scientific or fixed?
    let max_abs = non_null.iter().map(|v| v.abs()).fold(0f64, f64::max);

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
        Some(x) => format!("{:.*}", dp, x),
    })
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

// ============================================================================
// `{:num <printf>}` formatter (phase 2 subset)
// ============================================================================

/// Build a number formatter from a renaming RHS like `{:num %'d}` or
/// `{:num %.2f}`. Returns `None` if the spec is not a recognised
/// `{:num ...}` template; later phases extend the spec language.
fn build_num_format(rhs: &str) -> Option<Box<dyn Fn(Option<f64>) -> String>> {
    // Pattern: optional literal prefix, `{:num <spec>}`, optional literal suffix.
    let open = rhs.find("{:num ")?;
    let after = &rhs[open + "{:num ".len()..];
    let close = after.find('}')?;
    let body = after[..close].trim();
    let prefix = rhs[..open].to_string();
    let suffix = rhs[open + "{:num ".len() + close + 1..].to_string();

    // Phase 2 only supports integer formatting with the `'` (thousands) flag.
    // Richer printf parsing lands in phase 5.
    let spec = NumSpec::parse(body)?;
    Some(Box::new(move |v: Option<f64>| match v {
        None => "NA".to_string(),
        Some(x) if x.is_nan() => "NA".to_string(),
        Some(x) => format!("{}{}{}", prefix, spec.render(x), suffix),
    }))
}

struct NumSpec {
    /// Locale-aware thousands separator (`'` flag).
    thousands: bool,
    /// Conversion: `'d'`, `'f'`, etc.
    conv: char,
    /// Precision (digits after `.` for `f`); `None` if unspecified.
    precision: Option<u32>,
}

impl NumSpec {
    fn parse(body: &str) -> Option<Self> {
        let mut s = body.strip_prefix('%')?;
        let mut thousands = false;
        // flags
        loop {
            match s.chars().next()? {
                '\'' => {
                    thousands = true;
                    s = &s[1..];
                }
                '+' | '0' | '-' | ' ' | '#' => {
                    s = &s[s.chars().next()?.len_utf8()..];
                }
                _ => break,
            }
        }
        // width — ignore for now
        while s.chars().next()?.is_ascii_digit() {
            s = &s[1..];
        }
        // precision
        let mut precision: Option<u32> = None;
        if let Some(rest) = s.strip_prefix('.') {
            let mut n: u32 = 0;
            let mut consumed = 0;
            for c in rest.chars() {
                if let Some(d) = c.to_digit(10) {
                    n = n * 10 + d;
                    consumed += 1;
                } else {
                    break;
                }
            }
            precision = Some(n);
            s = &rest[consumed..];
        }
        let conv = s.chars().next()?;
        Some(NumSpec {
            thousands,
            conv,
            precision,
        })
    }

    fn render(&self, x: f64) -> String {
        match self.conv {
            'd' => {
                let n = x.round() as i64;
                if self.thousands {
                    insert_thousands(n)
                } else {
                    format!("{}", n)
                }
            }
            'f' => {
                let p = self.precision.unwrap_or(6) as usize;
                let s = format!("{:.*}", p, x);
                if self.thousands {
                    let (sign, rest) = if let Some(r) = s.strip_prefix('-') {
                        ("-", r)
                    } else {
                        ("", s.as_str())
                    };
                    let (int_part, frac_part) = match rest.find('.') {
                        Some(i) => (&rest[..i], &rest[i..]),
                        None => (rest, ""),
                    };
                    let int_n: i64 = int_part.parse().unwrap_or(0);
                    format!("{}{}{}", sign, insert_thousands(int_n), frac_part)
                } else {
                    s
                }
            }
            _ => format!("{}", x),
        }
    }
}

fn insert_thousands(n: i64) -> String {
    let neg = n < 0;
    let mut abs = if neg {
        (n as i128).unsigned_abs().to_string()
    } else {
        n.to_string()
    };
    let bytes = abs.as_bytes().to_vec();
    let len = bytes.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, b) in bytes.iter().enumerate() {
        let from_end = len - i;
        if i > 0 && from_end % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    abs.clear();
    if neg {
        format!("-{}", out)
    } else {
        out
    }
}
