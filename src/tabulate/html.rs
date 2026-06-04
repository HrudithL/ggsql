//! HTML writer for the TABULATE table IR.
//!
//! Phase 0: emits a `<style>` block plus a stub `<table>` so the harness has
//! something to diff against. Real rendering lands in phase 1+.

use super::GT_DEFAULT_CSS;

/// Render a placeholder table. Replaced by real rendering in phase 1.
pub fn render_placeholder(title: &str) -> String {
    format!(
        "<div><style>{css}</style>\n<table class=\"gt_table\"><caption>{t}</caption></table></div>",
        css = GT_DEFAULT_CSS,
        t = html_escape(title)
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
