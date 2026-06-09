//! Fixture-diff harness for the TABULATE implementation.
//!
//! For each directory under `tests/fixtures/` containing a complete fixture
//! (query.ggsql + expected.html + data.parquet + meta.toml), this test:
//!   1. Reads the query and the expected HTML.
//!   2. Parses the query, executes it against the data, renders HTML.
//!   3. Normalizes both sides via `ggsql::tabulate::test_normalize::normalize_html`.
//!   4. Asserts equality.

use std::fs;
use std::path::PathBuf;

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("fixtures")
}

fn list_fixtures() -> Vec<PathBuf> {
    let root = fixtures_root();
    let mut out = Vec::new();
    if !root.exists() {
        return out;
    }
    for entry in fs::read_dir(&root).expect("read fixtures dir") {
        let p = entry.unwrap().path();
        if p.is_dir()
            && p.join("query.ggsql").exists()
            && p.join("expected.html").exists()
            && p.join("meta.toml").exists()
        {
            out.push(p);
        }
    }
    out.sort();
    out
}

/// Run a single named fixture end-to-end: parse -> execute -> render -> normalize -> diff.
fn run_fixture(name: &str) {
    let fixture_dir = fixtures_root().join(name);
    let query = fs::read_to_string(fixture_dir.join("query.ggsql"))
        .unwrap_or_else(|e| panic!("cannot read query.ggsql for {}: {}", name, e));
    let expected_raw = fs::read_to_string(fixture_dir.join("expected.html"))
        .unwrap_or_else(|e| panic!("cannot read expected.html for {}: {}", name, e));
    let data_path = fixture_dir.join("data.parquet");

    let table_ir = ggsql::tabulate::execute::execute(&query, &data_path)
        .unwrap_or_else(|e| panic!("execute failed for {}: {}", name, e));
    let rendered = ggsql::tabulate::html::render(&table_ir);

    let got = ggsql::tabulate::test_normalize::normalize_html(&rendered);
    let want = ggsql::tabulate::test_normalize::normalize_html(&expected_raw);

    if got != want {
        eprintln!("=== FIXTURE {} MISMATCH ===", name);
        eprintln!("--- expected (normalized, first 2000) ---");
        eprintln!("{}", &want[..want.len().min(2000)]);
        eprintln!("--- got (normalized, first 2000) ---");
        eprintln!("{}", &got[..got.len().min(2000)]);
        panic!("fixture {} HTML does not match expected", name);
    }
}

#[test]
fn fixtures_are_well_formed() {
    let fixtures = list_fixtures();
    if fixtures.is_empty() {
        eprintln!(
            "no fixtures present (expected under {:?}). run `make sync-fixtures` or \
             `make fixtures-capture` on the host.",
            fixtures_root()
        );
        return;
    }
    for f in &fixtures {
        assert!(
            f.join("query.ggsql").exists(),
            "missing query.ggsql in {:?}",
            f
        );
        assert!(
            f.join("expected.html").exists(),
            "missing expected.html in {:?}",
            f
        );
        assert!(f.join("meta.toml").exists(), "missing meta.toml in {:?}", f);
        let html = fs::read_to_string(f.join("expected.html")).unwrap();
        assert!(
            html.contains("<table"),
            "expected.html lacks <table in {:?}",
            f
        );
    }
}

// ============================================================================
// Phase 1 fixture tests
// ============================================================================

#[test]
fn fixture_01_minimal_table_all_columns() {
    run_fixture("01_minimal_table_all_columns");
}

#[test]
fn fixture_02_column_selection_and_reordering() {
    run_fixture("02_column_selection_and_reordering");
}

#[test]
fn fixture_09_hide_a_column() {
    run_fixture("09_hide_a_column");
}

// ============================================================================
// Phase 2 fixture tests
// ============================================================================

#[test]
fn fixture_03_stub_from_a_row_label_column() {
    run_fixture("03_stub_from_a_row_label_column");
}

#[test]
fn fixture_04_header_with_title_and_subtitle() {
    run_fixture("04_header_with_title_and_subtitle");
}

#[test]
fn fixture_05_header_source_note_caption_column_labels() {
    run_fixture("05_header_source_note_caption_column_labels");
}

/// Render phase-1 fixtures to a viewable HTML page at
/// `target/tabulate_demo.html`. Ignored by default; run with
/// `cargo test --test tabulate_fixtures emit_demo -- --include-ignored --nocapture`.
#[test]
#[ignore = "demo output, not a correctness check"]
fn emit_demo() {
    let names = [
        "01_minimal_table_all_columns",
        "02_column_selection_and_reordering",
        "09_hide_a_column",
    ];
    let mut sections = String::new();
    for name in names {
        let dir = fixtures_root().join(name);
        let query = fs::read_to_string(dir.join("query.ggsql")).unwrap();
        let ir = ggsql::tabulate::execute::execute(&query, &dir.join("data.parquet")).unwrap();
        let html = ggsql::tabulate::html::render(&ir);
        sections.push_str(&format!(
            "<section><h2>{}</h2>\
             <pre style='background:#eee;padding:.5rem;border-radius:4px'>{}</pre>\
             {}</section><hr>",
            name,
            query.trim(),
            html
        ));
    }
    let page = format!(
        "<!doctype html><meta charset=utf-8><title>TABULATE demo</title>\
         <style>body{{font-family:system-ui;margin:2rem;max-width:1200px}}\
         h1{{color:#333}}h2{{color:#555;font-size:1rem;margin-top:2rem}}\
         hr{{margin:2rem 0;border:none;border-top:1px solid #ccc}}</style>\
         <h1>TABULATE &mdash; rendered by ggsql</h1>\
         <p>Pipeline: tree-sitter parse &rarr; DuckDB &rarr; Arrow &rarr; gt 1.3 HTML.</p>\
         {sections}"
    );
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target")
        .join("tabulate_demo_ggsql.html");
    fs::write(&out, page).unwrap();
    println!("wrote {}", out.display());
}

/// Smoke test: normalize expected.html to itself (proves the harness links).
#[test]
#[ignore = "superseded by per-fixture tests above"]
fn render_matches_expected() {
    let fixtures = list_fixtures();
    assert!(!fixtures.is_empty(), "no fixtures to render");
    for f in &fixtures {
        let expected = fs::read_to_string(f.join("expected.html")).unwrap();
        let _ = ggsql::tabulate::test_normalize::normalize_html(&expected);
        eprintln!("TODO: render {}", f.file_name().unwrap().to_string_lossy());
    }
}
