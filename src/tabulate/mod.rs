//! TABULATE clause implementation: parser → table IR → HTML writer.
//!
//! Ground truth is the captured `gt::as_raw_html()` output in
//! `tests/fixtures/*/expected.html`. See `/spec/GTSQL_AGENTBUILD_SPEC.md` and
//! `/spec/GTSQL_PLAN.md`.
//!
//! Status: bootstrap skeleton. Phases land per spec section 5.

pub mod ast;
pub mod execute;
pub mod format;
pub mod html;
pub mod test_normalize;

/// Default CSS, vendored from gt via `scripts/extract_gt_css.R`. Emitted
/// verbatim by the HTML writer. Must not be modified by the agent.
pub const GT_DEFAULT_CSS: &str = include_str!("gt_default.css");
