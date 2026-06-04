//! Execution pipeline: parse TABULATE query → run SQL against DuckDB →
//! produce an in-memory table representation ready for HTML rendering.
//!
//! This is the main entry point used by the fixture test harness.

use crate::parser::{tabulate as tab_parser, SourceTree};
use crate::tabulate::ast::{FormatMode, SettingValue, TabulateStmt};
use crate::{GgsqlError, Result};
use arrow::array::{Array, ArrayRef, Float64Array, StringArray};
use arrow::datatypes::{DataType, Schema};
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
}

#[derive(Debug, Clone)]
pub struct ColMeta {
    /// Column name (used as header label and `id=` attribute).
    pub name: String,
    /// Alignment.
    pub align: ColAlign,
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

    // 5. Determine which columns to show and their metadata.
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

    let visible_cols: Vec<&str> = display_cols
        .iter()
        .copied()
        .filter(|c| !hidden.iter().any(|h| h.eq_ignore_ascii_case(c)))
        .collect();

    // 6. Build column metadata with alignment.
    let columns: Vec<ColMeta> = visible_cols
        .iter()
        .map(|col_name| {
            let align = determine_alignment(col_name, &orig_schema, &combined);
            ColMeta {
                name: col_name.to_string(),
                align,
            }
        })
        .collect();

    // 7. Format cell values.
    // Build per-column formatters for numeric columns.
    type Fmt = Box<dyn Fn(Option<f64>) -> String>;
    let formatters: HashMap<String, Fmt> = columns
        .iter()
        .filter_map(|cm| {
            let idx = combined.schema().index_of(&cm.name).ok()?;
            let col = combined.column(idx);
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

    Ok(TableIr { columns, rows })
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

/// Determine alignment for a column given the original parquet Arrow schema
/// and the actual data column from DuckDB.
fn determine_alignment(
    col_name: &str,
    orig_schema: &Schema,
    batch: &arrow::record_batch::RecordBatch,
) -> ColAlign {
    // Check original parquet type (before DuckDB normalization).
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

    // Check DuckDB result type.
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

    // Determine: scientific or fixed (3 decimal places)?
    let max_abs = non_null.iter().map(|v| v.abs()).fold(0f64, f64::max);

    let fixed_width_of_max = format!("{:.3}", max_abs).len();
    let use_scientific = fixed_width_of_max > 9;

    Box::new(move |v: Option<f64>| match v {
        None => "NA".to_string(),
        Some(x) if x.is_nan() => "NA".to_string(),
        Some(x) if use_scientific => format_scientific_3dp(x),
        Some(x) => format!("{:.3}", x),
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

    // Float64 column with custom formatter.
    if let Some(fmt) = formatter {
        if let Some(fa) = col.as_any().downcast_ref::<Float64Array>() {
            let v = if fa.is_null(row) {
                None
            } else {
                Some(fa.value(row))
            };
            return fmt(v);
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
