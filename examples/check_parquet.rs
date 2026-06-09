use duckdb::{params, Connection};
fn main() {
    let conn = Connection::open_in_memory().unwrap();
    #[cfg(debug_assertions)]
    conn.execute("SET disabled_optimizers TO 'common_subplan'", params![]).ok();
    let mut stmt = conn.prepare("DESCRIBE SELECT * FROM read_parquet('tests/fixtures/01_minimal_table_all_columns/data.parquet')").unwrap();
    let rows = stmt.query_map(params![], |row| {
        let col: String = row.get(0)?;
        let typ: String = row.get(1)?;
        Ok((col, typ))
    }).unwrap();
    for row in rows {
        let (col, typ) = row.unwrap();
        println!("{}: {}", col, typ);
    }
    // Also show actual values via Arrow
    let mut stmt2 = conn.prepare("SELECT * FROM read_parquet('tests/fixtures/01_minimal_table_all_columns/data.parquet')").unwrap();
    let arrow = stmt2.query_arrow(params![]).unwrap();
    let schema = arrow.get_schema();
    println!("\nArrow schema:");
    for field in schema.fields() {
        println!("  {}: {:?}", field.name(), field.data_type());
    }
}
