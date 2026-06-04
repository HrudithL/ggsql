//! Fixture-diff harness for the TABULATE implementation.
//!
//! For each directory under `tests/fixtures/` containing a complete fixture
//! (query.ggsql + expected.html + data.parquet + meta.toml), this test:
//!   1. Reads the query and the expected HTML.
//!   2. (Phase 1+) Parses the query, executes it against the data, renders
//!      HTML via `ggsql::tabulate::html`.
//!   3. Normalizes both sides via `ggsql::tabulate::test_normalize::normalize_html`.
//!   4. Asserts equality.
//!
//! Until phase 1 lands, the per-fixture tests are `#[ignore]` and the harness
//! only verifies that fixtures are well-formed.

use std::fs;
use std::path::{Path, PathBuf};

fn fixtures_root() -> PathBuf {
    // CARGO_MANIFEST_DIR for the ggsql crate is `<repo>/src`; fixtures live
    // at `<repo>/tests/fixtures` so the host-run capture script writes to
    // an obvious top-level location.
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

/// Phase 1+ replaces this with: parse → execute → render → normalize → diff.
/// For now it just normalizes expected.html to itself to prove the harness
/// links and runs.
#[test]
#[ignore = "enabled once the parser/executor/writer for TABULATE land in phase 1"]
fn render_matches_expected() {
    let fixtures = list_fixtures();
    assert!(!fixtures.is_empty(), "no fixtures to render");
    for f in &fixtures {
        let expected = fs::read_to_string(f.join("expected.html")).unwrap();
        let _ = ggsql::tabulate::test_normalize::normalize_html(&expected);
        // TODO(phase 1): render(query, data) and diff against expected.
        eprintln!("TODO: render {}", f.file_name().unwrap().to_string_lossy());
    }
}

#[allow(dead_code)]
fn render(_query: &str, _data: &Path) -> String {
    unimplemented!("phase 1+: parse TABULATE → execute → emit HTML")
}
