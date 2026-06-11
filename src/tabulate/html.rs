//! HTML writer for TABULATE — produces output matching `gt::as_raw_html()`.
//!
//! gt 1.3+ inlines all styles via `style=` attributes; the vendored CSS file
//! is essentially empty. We emit the same inline-style HTML structure.

use super::GT_DEFAULT_CSS;
use crate::tabulate::execute::{CellStyle, ColAlign, ColMeta, HeaderNode, SummaryRow, TableIr};
use uuid::Uuid;

// ============================================================================
// Style constants (verbatim from gt 1.3.0 HTML output)
// ============================================================================

const DIV_STYLE: &str = concat!(
    "padding-left:0px;padding-right:0px;padding-top:10px;",
    "padding-bottom:10px;overflow-x:auto;overflow-y:auto;width:auto;height:auto;"
);

const TABLE_STYLE: &str = concat!(
    "-webkit-font-smoothing: antialiased; ",
    "-moz-osx-font-smoothing: grayscale; ",
    "font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, ",
    "'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji'; ",
    "display: table; border-collapse: collapse; line-height: normal; ",
    "margin-left: auto; margin-right: auto; color: #333333; font-size: 16px; ",
    "font-weight: normal; font-style: normal; background-color: #FFFFFF; ",
    "width: auto; border-top-style: solid; border-top-width: 2px; ",
    "border-top-color: #A8A8A8; border-right-style: none; border-right-width: 2px; ",
    "border-right-color: #D3D3D3; border-bottom-style: solid; border-bottom-width: 2px; ",
    "border-bottom-color: #A8A8A8; border-left-style: none; border-left-width: 2px; ",
    "border-left-color: #D3D3D3;"
);

// Same as TABLE_STYLE but with `width: auto;` swapped for the
// fixed-layout pair gt emits when any column carries an explicit width.
const TABLE_STYLE_FIXED: &str = concat!(
    "-webkit-font-smoothing: antialiased; ",
    "-moz-osx-font-smoothing: grayscale; ",
    "font-family: system-ui, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif, ",
    "'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji'; ",
    "display: table; border-collapse: collapse; line-height: normal; ",
    "margin-left: auto; margin-right: auto; color: #333333; font-size: 16px; ",
    "font-weight: normal; font-style: normal; background-color: #FFFFFF; ",
    "border-top-style: solid; border-top-width: 2px; ",
    "border-top-color: #A8A8A8; border-right-style: none; border-right-width: 2px; ",
    "border-right-color: #D3D3D3; border-bottom-style: solid; border-bottom-width: 2px; ",
    "border-bottom-color: #A8A8A8; border-left-style: none; border-left-width: 2px; ",
    "border-left-color: #D3D3D3; table-layout: fixed; width: 0px;"
);

const THEAD_STYLE: &str = "border-style: none;";

const THEAD_TR_STYLE: &str = concat!(
    "border-style: none; border-top-style: solid; ",
    "border-top-width: 2px; border-top-color: #D3D3D3; border-bottom-style: solid; ",
    "border-bottom-width: 2px; border-bottom-color: #D3D3D3; border-left-style: none; ",
    "border-left-width: 1px; border-left-color: #D3D3D3; border-right-style: none; ",
    "border-right-width: 1px; border-right-color: #D3D3D3;"
);

// Spanner header rows: same border setup as THEAD_TR_STYLE but the bottom
// border is rendered `hidden` so the spanner cell's own bottom border
// (from `<div class="gt_column_spanner">`) is the visible separator.
const SPANNER_TR_STYLE: &str = concat!(
    "border-style: none; border-top-style: solid; ",
    "border-top-width: 2px; border-top-color: #D3D3D3; ",
    "border-bottom-width: 2px; border-bottom-color: #D3D3D3; border-left-style: none; ",
    "border-left-width: 1px; border-left-color: #D3D3D3; border-right-style: none; ",
    "border-right-width: 1px; border-right-color: #D3D3D3; border-bottom-style: hidden;"
);

// Spanner-cell base style: shared between spanner cells and the empty
// placeholder cells that sit above a column whose row was lifted.
const SPANNER_OUTER_BASE_PREFIX: &str = concat!(
    "border-style: none; color: #333333; ",
    "background-color: #FFFFFF; font-size: 100%; font-weight: normal; ",
    "text-transform: inherit; padding-top: 0; padding-bottom: 0; "
);

// `gt_column_spanner` div style — the bottom border that visually marks the
// spanner cell sits on the inner div, not the outer `<th>`.
const SPANNER_DIV_STYLE: &str = concat!(
    "border-bottom-style: solid; border-bottom-width: 2px; ",
    "border-bottom-color: #D3D3D3; vertical-align: bottom; padding-top: 5px; ",
    "padding-bottom: 5px; overflow-x: hidden; display: inline-block; width: 100%;"
);

const TH_BASE_STYLE: &str = concat!(
    "border-style: none; color: #333333; ",
    "background-color: #FFFFFF; font-size: 100%; font-weight: normal; ",
    "text-transform: inherit; border-left-style: none; border-left-width: 1px; ",
    "border-left-color: #D3D3D3; border-right-style: none; border-right-width: 1px; ",
    "border-right-color: #D3D3D3; vertical-align: bottom; padding-top: 5px; ",
    "padding-bottom: 6px; padding-left: 5px; padding-right: 5px; ",
    "overflow-x: hidden;"
);

const TBODY_STYLE: &str = concat!(
    "border-style: none; border-top-style: solid; ",
    "border-top-width: 2px; border-top-color: #D3D3D3; border-bottom-style: solid; ",
    "border-bottom-width: 2px; border-bottom-color: #D3D3D3;"
);

const TR_STYLE: &str = "border-style: none;";

const TD_BASE_STYLE: &str = concat!(
    "border-style: none; padding-top: 8px; ",
    "padding-bottom: 8px; padding-left: 5px; padding-right: 5px; margin: 10px; ",
    "border-top-style: solid; border-top-width: 1px; border-top-color: #D3D3D3; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "border-right-style: none; border-right-width: 1px; border-right-color: #D3D3D3; ",
    "vertical-align: middle; overflow-x: hidden;"
);

// Heading row container (one for each heading line). Title and subtitle each
// live in their own `<tr class="gt_heading">`.
const HEADING_TR_STYLE: &str = concat!(
    "border-style: none; background-color: #FFFFFF; ",
    "text-align: center; border-bottom-color: #FFFFFF; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "border-right-style: none; border-right-width: 1px; border-right-color: #D3D3D3;"
);

const TITLE_TD_STYLE: &str = concat!(
    "border-style: none; color: #333333; font-size: 125%; ",
    "padding-top: 4px; padding-bottom: 4px; padding-left: 5px; padding-right: 5px; ",
    "border-bottom-width: 0; background-color: #FFFFFF; text-align: center; ",
    "border-bottom-color: #FFFFFF; border-left-style: none; border-left-width: 1px; ",
    "border-left-color: #D3D3D3; border-right-style: none; border-right-width: 1px; ",
    "border-right-color: #D3D3D3; font-weight: normal;"
);

const SUBTITLE_TD_STYLE: &str = concat!(
    "border-style: none; color: #333333; font-size: 85%; ",
    "padding-top: 3px; padding-bottom: 5px; padding-left: 5px; padding-right: 5px; ",
    "border-top-color: #FFFFFF; border-top-width: 0; background-color: #FFFFFF; ",
    "text-align: center; border-left-style: none; border-left-width: 1px; ",
    "border-left-color: #D3D3D3; border-right-style: none; border-right-width: 1px; ",
    "border-right-color: #D3D3D3; border-bottom-style: solid; border-bottom-width: 2px; ",
    "border-bottom-color: #D3D3D3; font-weight: normal;"
);

// Title without subtitle: gets the bottom border directly.
const TITLE_ONLY_TD_STYLE: &str = concat!(
    "border-style: none; color: #333333; font-size: 125%; ",
    "padding-top: 4px; padding-bottom: 4px; padding-left: 5px; padding-right: 5px; ",
    "background-color: #FFFFFF; text-align: center; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "border-right-style: none; border-right-width: 1px; border-right-color: #D3D3D3; ",
    "border-bottom-style: solid; border-bottom-width: 2px; border-bottom-color: #D3D3D3; ",
    "font-weight: normal;"
);

// Stub column body cell (`<th>` inside `<tbody>`). Same base as `<td>` plus
// gt's stub-specific overrides (right-border, font-weight initial, etc.).
// The trailing `text-align` (and `font-variant-numeric` for right-aligned
// numeric stubs) is appended dynamically per row in `render_tr`.
const STUB_TH_STYLE: &str = concat!(
    "border-style: none; padding-top: 8px; padding-bottom: 8px; margin: 10px; ",
    "border-top-style: solid; border-top-width: 1px; border-top-color: #D3D3D3; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "vertical-align: middle; overflow-x: hidden; ",
    "color: #333333; background-color: #FFFFFF; font-size: 100%; ",
    "font-weight: initial; text-transform: inherit; ",
    "border-right-style: solid; border-right-width: 2px; border-right-color: #D3D3D3; ",
    "padding-left: 5px; padding-right: 5px;"
);

// FACET row-group support. gt emits a heading row before each group's body
// rows, and the first body row of every group gets a heavier top border
// (`border-top-width: 2px;`) plus the `gt_row_group_first` class. The
// "first-of-group" variants below omit the regular `border-top-width: 1px;`
// declaration so the renderer can append `border-top-width: 2px;` at the
// end (matching gt's emit order byte-for-byte).
const GROUP_HEADING_TH_STYLE: &str = concat!(
    "border-style: none; padding-top: 8px; padding-bottom: 8px; ",
    "padding-left: 5px; padding-right: 5px; ",
    "color: #333333; background-color: #FFFFFF; font-size: 100%; ",
    "font-weight: initial; text-transform: inherit; ",
    "border-top-style: solid; border-top-width: 2px; border-top-color: #D3D3D3; ",
    "border-bottom-style: solid; border-bottom-width: 2px; border-bottom-color: #D3D3D3; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "border-right-style: none; border-right-width: 1px; border-right-color: #D3D3D3; ",
    "vertical-align: middle; text-align: left;"
);

const TD_BASE_STYLE_FIRST_OF_GROUP: &str = concat!(
    "border-style: none; padding-top: 8px; ",
    "padding-bottom: 8px; padding-left: 5px; padding-right: 5px; margin: 10px; ",
    "border-top-style: solid; border-top-color: #D3D3D3; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "border-right-style: none; border-right-width: 1px; border-right-color: #D3D3D3; ",
    "vertical-align: middle; overflow-x: hidden;"
);

const STUB_TH_STYLE_FIRST_OF_GROUP: &str = concat!(
    "border-style: none; padding-top: 8px; padding-bottom: 8px; margin: 10px; ",
    "border-top-style: solid; border-top-color: #D3D3D3; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "vertical-align: middle; overflow-x: hidden; ",
    "color: #333333; background-color: #FFFFFF; font-size: 100%; ",
    "font-weight: initial; text-transform: inherit; ",
    "border-right-style: solid; border-right-width: 2px; border-right-color: #D3D3D3; ",
    "padding-left: 5px; padding-right: 5px;"
);

// Summary-row cell base styles — one variant per (first × last) combination
// because gt reorders CSS declarations across the four cases. Each constant
// stops just before the trailing `text-align: <align>;` so the renderer can
// append the column's alignment (and `font-variant-numeric: tabular-nums;`
// for numeric columns, then `border-top-width: 2px;` for FIRST variants).

const SUMMARY_STUB_TH_STYLE_MIDDLE: &str = concat!(
    "border-style: none; margin: 10px; ",
    "border-top-style: solid; border-top-width: 1px; border-top-color: #D3D3D3; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "vertical-align: middle; overflow-x: hidden; ",
    "font-size: 100%; font-weight: initial; ",
    "border-right-style: solid; border-right-width: 2px; border-right-color: #D3D3D3; ",
    "color: #333333; background-color: #FFFFFF; text-transform: inherit; ",
    "padding-top: 8px; padding-bottom: 8px; padding-left: 5px; padding-right: 5px;"
);

const SUMMARY_STUB_TH_STYLE_FIRST: &str = concat!(
    "border-style: none; margin: 10px; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "vertical-align: middle; overflow-x: hidden; ",
    "font-size: 100%; font-weight: initial; ",
    "border-right-style: solid; border-right-width: 2px; border-right-color: #D3D3D3; ",
    "color: #333333; background-color: #FFFFFF; text-transform: inherit; ",
    "padding-top: 8px; padding-bottom: 8px; padding-left: 5px; padding-right: 5px; ",
    "border-top-style: solid; border-top-color: #D3D3D3;"
);

const SUMMARY_STUB_TH_STYLE_LAST: &str = concat!(
    "border-style: none; margin: 10px; ",
    "border-top-style: solid; border-top-width: 1px; border-top-color: #D3D3D3; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "vertical-align: middle; overflow-x: hidden; ",
    "font-size: 100%; font-weight: initial; ",
    "border-right-style: solid; border-right-width: 2px; border-right-color: #D3D3D3; ",
    "color: #333333; background-color: #FFFFFF; text-transform: inherit; ",
    "padding-top: 8px; padding-bottom: 8px; padding-left: 5px; padding-right: 5px; ",
    "border-bottom-style: solid; border-bottom-width: 2px; border-bottom-color: #D3D3D3;"
);

const SUMMARY_STUB_TH_STYLE_FIRST_LAST: &str = concat!(
    "border-style: none; margin: 10px; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "vertical-align: middle; overflow-x: hidden; ",
    "font-size: 100%; font-weight: initial; ",
    "border-right-style: solid; border-right-width: 2px; border-right-color: #D3D3D3; ",
    "color: #333333; background-color: #FFFFFF; text-transform: inherit; ",
    "border-top-style: solid; border-top-color: #D3D3D3; ",
    "padding-top: 8px; padding-bottom: 8px; padding-left: 5px; padding-right: 5px; ",
    "border-bottom-style: solid; border-bottom-width: 2px; border-bottom-color: #D3D3D3;"
);

const SUMMARY_TD_STYLE_MIDDLE: &str = concat!(
    "border-style: none; margin: 10px; ",
    "border-top-style: solid; border-top-width: 1px; border-top-color: #D3D3D3; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "border-right-style: none; border-right-width: 1px; border-right-color: #D3D3D3; ",
    "vertical-align: middle; overflow-x: hidden; ",
    "color: #333333; background-color: #FFFFFF; text-transform: inherit; ",
    "padding-top: 8px; padding-bottom: 8px; padding-left: 5px; padding-right: 5px;"
);

const SUMMARY_TD_STYLE_FIRST: &str = concat!(
    "border-style: none; margin: 10px; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "border-right-style: none; border-right-width: 1px; border-right-color: #D3D3D3; ",
    "vertical-align: middle; overflow-x: hidden; ",
    "color: #333333; background-color: #FFFFFF; text-transform: inherit; ",
    "padding-top: 8px; padding-bottom: 8px; padding-left: 5px; padding-right: 5px; ",
    "border-top-style: solid; border-top-color: #D3D3D3;"
);

const SUMMARY_TD_STYLE_LAST: &str = concat!(
    "border-style: none; margin: 10px; ",
    "border-top-style: solid; border-top-width: 1px; border-top-color: #D3D3D3; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "border-right-style: none; border-right-width: 1px; border-right-color: #D3D3D3; ",
    "vertical-align: middle; overflow-x: hidden; ",
    "color: #333333; background-color: #FFFFFF; text-transform: inherit; ",
    "padding-top: 8px; padding-bottom: 8px; padding-left: 5px; padding-right: 5px; ",
    "border-bottom-style: solid; border-bottom-width: 2px; border-bottom-color: #D3D3D3;"
);

const SUMMARY_TD_STYLE_FIRST_LAST: &str = concat!(
    "border-style: none; margin: 10px; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "border-right-style: none; border-right-width: 1px; border-right-color: #D3D3D3; ",
    "vertical-align: middle; overflow-x: hidden; ",
    "color: #333333; background-color: #FFFFFF; text-transform: inherit; ",
    "border-top-style: solid; border-top-color: #D3D3D3; ",
    "padding-top: 8px; padding-bottom: 8px; padding-left: 5px; padding-right: 5px; ",
    "border-bottom-style: solid; border-bottom-width: 2px; border-bottom-color: #D3D3D3;"
);

const TFOOT_STYLE: &str = "border-style: none;";

const SOURCENOTES_TR_STYLE: &str = concat!(
    "border-style: none; color: #333333; background-color: #FFFFFF; ",
    "border-bottom-style: none; border-bottom-width: 2px; border-bottom-color: #D3D3D3; ",
    "border-left-style: none; border-left-width: 2px; border-left-color: #D3D3D3; ",
    "border-right-style: none; border-right-width: 2px; border-right-color: #D3D3D3;"
);

const SOURCENOTE_TD_STYLE: &str = concat!(
    "border-style: none; font-size: 90%; ",
    "padding-top: 4px; padding-bottom: 4px; padding-left: 5px; padding-right: 5px;"
);

// ============================================================================
// Public render function
// ============================================================================

/// Render the table IR as a gt-compatible HTML string.
pub fn render(table: &TableIr) -> String {
    let id = generate_id();
    let mut out = String::with_capacity(4096);
    let has_widths = table.columns.iter().any(|c| c.width.is_some());

    out.push_str(&format!(
        "<div id=\"{}\" style=\"{}\">\n  \n  ",
        id, DIV_STYLE
    ));

    // Emit CSS only if it contains actual rules.
    let css = GT_DEFAULT_CSS.trim();
    let has_rules = css.lines().any(|l| {
        let t = l.trim();
        !t.is_empty() && !t.starts_with("/*") && !t.starts_with('*') && !t.starts_with("//")
    });
    if has_rules {
        out.push_str(&format!("<style>{}</style>\n  ", css));
    }

    out.push_str(&format!(
        "<table class=\"gt_table\" \
         data-quarto-disable-processing=\"false\" \
         data-quarto-bootstrap=\"false\" \
         style=\"{}\"{} bgcolor=\"#FFFFFF\">\n",
        if has_widths {
            TABLE_STYLE_FIXED
        } else {
            TABLE_STYLE
        },
        if has_widths { " width=\"0\"" } else { "" },
    ));

    // colgroup — gt emits this whenever any column has an explicit width.
    if has_widths {
        out.push_str("  <colgroup>\n");
        for col in &table.columns {
            match &col.width {
                Some(w) => out.push_str(&format!("    <col style=\"width:{};\">\n", w)),
                None => out.push_str("    <col>\n"),
            }
        }
        out.push_str("  </colgroup>\n");
    }

    // thead
    out.push_str(&format!("  <thead style=\"{}\">\n", THEAD_STYLE));

    let ncols = table.columns.len();
    if let Some(title) = &table.title {
        let td_style = if table.subtitle.is_some() {
            TITLE_TD_STYLE
        } else {
            TITLE_ONLY_TD_STYLE
        };
        let cls = if table.subtitle.is_some() {
            "gt_heading gt_title gt_font_normal"
        } else {
            "gt_heading gt_title gt_font_normal gt_bottom_border"
        };
        out.push_str(&format!(
            "    <tr class=\"gt_heading\" style=\"{}\" bgcolor=\"#FFFFFF\" align=\"center\">\n      \
             <td colspan=\"{}\" class=\"{}\" style=\"{}\" bgcolor=\"#FFFFFF\" align=\"center\">{}</td>\n    </tr>\n",
            HEADING_TR_STYLE,
            ncols,
            cls,
            td_style,
            html_escape(title),
        ));
    }
    if let Some(subtitle) = &table.subtitle {
        out.push_str(&format!(
            "    <tr class=\"gt_heading\" style=\"{}\" bgcolor=\"#FFFFFF\" align=\"center\">\n      \
             <td colspan=\"{}\" class=\"gt_heading gt_subtitle gt_font_normal gt_bottom_border\" \
             style=\"{}\" bgcolor=\"#FFFFFF\" align=\"center\">{}</td>\n    </tr>\n",
            HEADING_TR_STYLE,
            ncols,
            SUBTITLE_TD_STYLE,
            html_escape(subtitle),
        ));
    }

    out.push_str(&render_header(table));
    out.push_str("  </thead>\n");

    // tbody
    out.push_str(&format!(
        "  <tbody class=\"gt_table_body\" style=\"{}\">\n",
        TBODY_STYLE
    ));
    if table.groups.is_empty() {
        for (row_idx, row) in table.rows.iter().enumerate() {
            let bg_row = table.cell_bg.get(row_idx).map(|v| v.as_slice());
            let style_row = table.cell_style.get(row_idx).map(|v| v.as_slice());
            out.push_str(&render_tr(
                row,
                &table.columns,
                table.stub_col,
                row_idx,
                bg_row,
                style_row,
                None,
                false,
                row_idx + 1,
            ));
        }
    } else {
        let mut global_row = 0usize;
        for group in &table.groups {
            // Group heading row.
            out.push_str(&format!(
                "    <tr class=\"gt_group_heading_row\" style=\"{}\">\n      \
                 <th colspan=\"{}\" class=\"gt_group_heading\" scope=\"colgroup\" \
                 id=\"{}\" style=\"{}\" bgcolor=\"#FFFFFF\" valign=\"middle\" \
                 align=\"left\">{}</th>\n    </tr>\n",
                TR_STYLE,
                ncols,
                group.name,
                GROUP_HEADING_TH_STYLE,
                html_escape(&group.name),
            ));

            // Summary rows for `side = 'top'` precede body rows.
            if group.summary_side.eq_ignore_ascii_case("top") {
                for (n, sr) in group.summary_rows.iter().enumerate() {
                    out.push_str(&render_summary_tr(
                        sr,
                        &table.columns,
                        table.stub_col,
                        &group.name,
                        n + 1,
                        n == 0,
                        n + 1 == group.summary_rows.len(),
                    ));
                }
            }

            for (j, &row_idx) in group.row_indices.iter().enumerate() {
                global_row += 1;
                let row = &table.rows[row_idx];
                let bg_row = table.cell_bg.get(row_idx).map(|v| v.as_slice());
                let style_row = table.cell_style.get(row_idx).map(|v| v.as_slice());
                out.push_str(&render_tr(
                    row,
                    &table.columns,
                    table.stub_col,
                    row_idx,
                    bg_row,
                    style_row,
                    Some(group.name.as_str()),
                    j == 0,
                    global_row,
                ));
            }

            if !group.summary_side.eq_ignore_ascii_case("top") {
                for (n, sr) in group.summary_rows.iter().enumerate() {
                    out.push_str(&render_summary_tr(
                        sr,
                        &table.columns,
                        table.stub_col,
                        &group.name,
                        n + 1,
                        n == 0,
                        n + 1 == group.summary_rows.len(),
                    ));
                }
            }
        }
    }
    out.push_str("  </tbody>\n");

    // tfoot (sourcenote / caption)
    if let Some(caption) = &table.caption {
        out.push_str(&format!(
            "  <tfoot style=\"{}\">\n    <tr class=\"gt_sourcenotes\" style=\"{}\" bgcolor=\"#FFFFFF\">\n      \
             <td class=\"gt_sourcenote\" colspan=\"{}\" style=\"{}\">{}</td>\n    </tr>\n  </tfoot>\n",
            TFOOT_STYLE,
            SOURCENOTES_TR_STYLE,
            ncols,
            SOURCENOTE_TD_STYLE,
            html_escape(caption),
        ));
    }

    out.push_str("  </table></div>");

    out
}

// ============================================================================
// Helpers
// ============================================================================

fn generate_id() -> String {
    let u = Uuid::new_v4().to_string().replace('-', "");
    u[..10].to_string()
}

fn render_th(col: &ColMeta, is_stub: bool, rowspan: usize) -> String {
    // gt always renders the stub column heading as left-aligned regardless
    // of the underlying data type (the body cells follow the data alignment
    // — see `render_tr`).
    let align = if is_stub { ColAlign::Left } else { col.align };
    let align_str = match align {
        ColAlign::Left => "left",
        ColAlign::Right => "right",
        ColAlign::Center => "center",
    };
    let gt_class = align.gt_class();
    let mut style = format!("{} text-align: {};", TH_BASE_STYLE, align_str);
    if align.tabular_nums() {
        style.push_str(" font-variant-numeric: tabular-nums;");
    }
    // Stub columns use the special `a::stub` id slot in gt's HTML.
    let id = if is_stub {
        "a::stub"
    } else {
        col.name.as_str()
    };
    format!(
        "      <th class=\"gt_col_heading gt_columns_bottom_border {}\" \
         rowspan=\"{}\" colspan=\"1\" scope=\"col\" id=\"{}\" \
         style=\"{}\" bgcolor=\"#FFFFFF\" valign=\"bottom\" align=\"{}\">{}</th>\n",
        gt_class,
        rowspan,
        id,
        style,
        align_str,
        html_escape(&col.label)
    )
}

/// One cell in a header row, returned by the forest walk.
struct HeaderCell {
    /// Rendered `<th>...</th>` string, less the leftmost / rightmost padding
    /// fix-up. For cells that are NOT spanner-class, this is the final
    /// markup; for spanner-class cells, this carries placeholder tokens
    /// `__LRPAD__` that get substituted to the leftmost/rightmost/interior
    /// padding suffix.
    markup: String,
    /// Whether this cell uses `gt_column_spanner_outer` (and therefore needs
    /// the leftmost / rightmost padding adjustment).
    is_spanner_outer: bool,
}

/// Render the `<thead>` rows. Handles both flat (no spanners) and nested
/// (spanner forest) tables.
fn render_header(table: &TableIr) -> String {
    let max_height = table
        .header_forest
        .iter()
        .map(|n| n.height())
        .max()
        .unwrap_or(0);
    let total_rows = max_height + 1;
    let mut out = String::new();

    for row in 0..total_rows {
        // Walk the forest collecting cells for this row.
        let mut cells: Vec<HeaderCell> = Vec::new();
        for node in &table.header_forest {
            collect_header_cells(
                node,
                row,
                0,
                max_height,
                &table.columns,
                table.stub_col,
                &mut cells,
            );
        }
        // Apply leftmost / rightmost padding suffix to spanner-outer cells.
        let n = cells.len();
        let last_idx = n.saturating_sub(1);
        for (i, cell) in cells.iter_mut().enumerate() {
            if !cell.is_spanner_outer {
                continue;
            }
            let is_first = i == 0;
            let is_last = i == last_idx;
            let suffix = if is_last {
                // Rightmost takes precedence when a cell is also first.
                "padding-left: 4px; text-align: center; padding-right: 0;"
            } else if is_first {
                "padding-right: 4px; text-align: center; padding-left: 0;"
            } else {
                "padding-left: 4px; padding-right: 4px; text-align: center;"
            };
            cell.markup = cell.markup.replace("__LRPAD__", suffix);
        }
        // Row TR style: column row uses the standard thead tr; spanner rows
        // use the hidden-bottom-border variant.
        let tr_style = if row == max_height {
            THEAD_TR_STYLE
        } else {
            SPANNER_TR_STYLE
        };
        let tr_class = if row == max_height {
            "gt_col_headings"
        } else {
            "gt_col_headings gt_spanner_row"
        };
        out.push_str(&format!(
            "    <tr class=\"{}\" style=\"{}\">\n",
            tr_class, tr_style
        ));
        for cell in cells {
            out.push_str(&cell.markup);
        }
        out.push_str("    </tr>\n");
    }
    out
}

/// Recursively gather cells for `row` from a header subtree.
///
/// `current_depth` is 0 for a top-level forest entry and incremented when
/// recursing into a spanner's children.
fn collect_header_cells(
    node: &HeaderNode,
    row: usize,
    current_depth: usize,
    max_height: usize,
    columns: &[ColMeta],
    stub_col: Option<usize>,
    out: &mut Vec<HeaderCell>,
) {
    match node {
        HeaderNode::Column { col_idx } => {
            let col = &columns[*col_idx];
            let is_stub = Some(*col_idx) == stub_col;
            // Top-level columns get lifted into the lowest spanner row so they
            // can rowspan into the column row alongside their spanner siblings.
            // Nested columns sit on the column row.
            let c_level = if current_depth == 0 && max_height > 0 {
                max_height - 1
            } else {
                max_height
            };
            if row < c_level {
                // Empty placeholder slot. Class matches a spanner cell so
                // gt's CSS treats it uniformly.
                let markup = format!(
                    "      <th class=\"gt_center gt_columns_top_border gt_column_spanner_outer\" \
                     rowspan=\"1\" colspan=\"1\" scope=\"col\" \
                     style=\"{}__LRPAD__\" bgcolor=\"#FFFFFF\" align=\"center\"></th>\n",
                    SPANNER_OUTER_BASE_PREFIX
                );
                out.push(HeaderCell {
                    markup,
                    is_spanner_outer: true,
                });
            } else if row == c_level {
                let rowspan = max_height + 1 - row;
                out.push(HeaderCell {
                    markup: render_th(col, is_stub, rowspan),
                    is_spanner_outer: false,
                });
            }
            // row > c_level → covered by rowspan from earlier; skip.
        }
        HeaderNode::Spanner {
            id: _,
            label,
            children,
        } => {
            let span_row = max_height - node.height();
            let leaf_count = node.leaf_count();
            if row < span_row {
                // Placeholder covering all leaves of this spanner. Not
                // observed in fixtures 6/7/8 (top-level spanners always sit at
                // row 0); kept for completeness.
                let markup = format!(
                    "      <th class=\"gt_center gt_columns_top_border gt_column_spanner_outer\" \
                     rowspan=\"1\" colspan=\"{}\" scope=\"col\" \
                     style=\"{}__LRPAD__\" bgcolor=\"#FFFFFF\" align=\"center\"></th>\n",
                    leaf_count, SPANNER_OUTER_BASE_PREFIX
                );
                out.push(HeaderCell {
                    markup,
                    is_spanner_outer: true,
                });
            } else if row == span_row {
                let markup = format!(
                    "      <th class=\"gt_center gt_columns_top_border gt_column_spanner_outer\" \
                     rowspan=\"1\" colspan=\"{}\" scope=\"colgroup\" id=\"{}\" \
                     style=\"{}__LRPAD__\" bgcolor=\"#FFFFFF\" align=\"center\">\n        \
                     <div class=\"gt_column_spanner\" style=\"{}\">{}</div>\n      </th>\n",
                    leaf_count,
                    html_escape(label),
                    SPANNER_OUTER_BASE_PREFIX,
                    SPANNER_DIV_STYLE,
                    html_escape(label),
                );
                out.push(HeaderCell {
                    markup,
                    is_spanner_outer: true,
                });
            } else {
                for child in children {
                    collect_header_cells(
                        child,
                        row,
                        current_depth + 1,
                        max_height,
                        columns,
                        stub_col,
                        out,
                    );
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_tr(
    row: &[String],
    columns: &[ColMeta],
    stub_col: Option<usize>,
    row_idx: usize,
    bg_row: Option<&[Option<String>]>,
    style_row: Option<&[CellStyle]>,
    group_name: Option<&str>,
    is_first_in_group: bool,
    global_row_number: usize,
) -> String {
    let _ = row_idx; // row_idx is only used to position cell_bg/style; the
                     // visible stub id derives from global_row_number so
                     // FACET tables get a monotonically increasing counter.
    let mut out = String::new();
    let tr_class = if is_first_in_group {
        " class=\"gt_row_group_first\""
    } else {
        ""
    };
    out.push_str(&format!("    <tr{} style=\"{}\">", tr_class, TR_STYLE));
    // Stub id for this row (used as the cell id of the stub <th> and as the
    // first token of every sibling cell's `headers` attribute).
    let stub_id = stub_col.map(|_| format!("stub_1_{}", global_row_number));
    for (i, (value, col)) in row.iter().zip(columns.iter()).enumerate() {
        let bg = bg_row.and_then(|r| r.get(i)).and_then(|o| o.as_deref());
        let hl = style_row.and_then(|s| s.get(i));
        let cell = if Some(i) == stub_col {
            // Stub <th> in tbody. Alignment / numeric styling tracks the
            // column's resolved alignment (numeric stubs render right).
            let align = col.align;
            let align_str = match align {
                ColAlign::Left => "left",
                ColAlign::Right => "right",
                ColAlign::Center => "center",
            };
            let gt_class = align.gt_class();
            let base = if is_first_in_group {
                STUB_TH_STYLE_FIRST_OF_GROUP
            } else {
                STUB_TH_STYLE
            };
            let mut style = format!("{} text-align: {};", base, align_str);
            if align.tabular_nums() {
                style.push_str(" font-variant-numeric: tabular-nums;");
            }
            if is_first_in_group {
                style.push_str(" border-top-width: 2px;");
            }
            format!(
                "<th id=\"{}\" scope=\"row\" class=\"gt_row {} gt_stub\" style=\"{}\" \
                 valign=\"middle\" bgcolor=\"#FFFFFF\" align=\"{}\">{}</th>",
                stub_id.as_deref().unwrap_or(""),
                gt_class,
                style,
                align_str,
                html_escape(value)
            )
        } else {
            let align = col.align;
            let align_str = match align {
                ColAlign::Left => "left",
                ColAlign::Right => "right",
                ColAlign::Center => "center",
            };
            let gt_class = align.gt_class();
            let base = if is_first_in_group {
                TD_BASE_STYLE_FIRST_OF_GROUP
            } else {
                TD_BASE_STYLE
            };
            let mut style = format!("{} text-align: {};", base, align_str);
            if align.tabular_nums() {
                style.push_str(" font-variant-numeric: tabular-nums;");
            }
            if is_first_in_group {
                style.push_str(" border-top-width: 2px;");
            }
            // Merge SCALE background with HIGHLIGHT overrides. HIGHLIGHT
            // background wins over SCALE; HIGHLIGHT color and face append
            // their own declarations after any scale-derived foreground.
            let hl_bg = hl.and_then(|h| h.background.as_deref());
            let hl_color = hl.and_then(|h| h.color.as_deref());
            let hl_face = hl.and_then(|h| h.face.as_deref());
            let effective_bg = hl_bg.or(bg);
            let (style_with_bg, bgcolor_attr) = match effective_bg {
                Some(hex) => {
                    let attr = format!(" bgcolor=\"{}\"", hex);
                    if hl_bg.is_some() {
                        // HIGHLIGHT background: do not synthesize a
                        // foreground colour (gt's tab_style only sets
                        // what was asked for).
                        (format!("{} background-color: {};", style, hex), attr)
                    } else {
                        let fg = crate::tabulate::scale::ideal_fg(hex);
                        (
                            format!("{} background-color: {}; color: {};", style, hex, fg),
                            attr,
                        )
                    }
                }
                None => (style, String::new()),
            };
            let mut style_final = style_with_bg;
            if let Some(c) = hl_color {
                style_final.push_str(&format!(" color: {};", c));
            }
            if let Some(f) = hl_face {
                let decl = match f.to_ascii_lowercase().as_str() {
                    "italic" | "oblique" => format!("font-style: {};", f),
                    _ => format!("font-weight: {};", f),
                };
                style_final.push(' ');
                style_final.push_str(&decl);
            }
            let headers = match (&group_name, &stub_id) {
                (Some(g), Some(sid)) => format!("{} {} {}", g, sid, col.name),
                (Some(g), None) => format!("{} {}", g, col.name),
                (None, Some(sid)) => format!("{} {}", sid, col.name),
                (None, None) => col.name.clone(),
            };
            let cell_text = if col.raw_html {
                value.clone()
            } else {
                html_escape(value)
            };
            format!(
                "<td headers=\"{}\" class=\"gt_row {}\" style=\"{}\"{} \
                 valign=\"middle\" align=\"{}\">{}</td>",
                headers, gt_class, style_final, bgcolor_attr, align_str, cell_text
            )
        };
        if i == 0 {
            out.push_str(&cell);
        } else {
            out.push('\n');
            out.push_str(&cell);
        }
    }
    out.push_str("</tr>\n");
    out
}

fn render_summary_tr(
    sr: &SummaryRow,
    columns: &[ColMeta],
    stub_col: Option<usize>,
    group_name: &str,
    summary_idx: usize,
    is_first: bool,
    is_last: bool,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("    <tr style=\"{}\">", TR_STYLE));

    let class_suffix = match (is_first, is_last) {
        (true, true) => " gt_first_summary_row thick gt_last_summary_row",
        (true, false) => " gt_first_summary_row thick",
        (false, true) => " gt_last_summary_row",
        (false, false) => "",
    };

    let stub_base = match (is_first, is_last) {
        (true, true) => SUMMARY_STUB_TH_STYLE_FIRST_LAST,
        (true, false) => SUMMARY_STUB_TH_STYLE_FIRST,
        (false, true) => SUMMARY_STUB_TH_STYLE_LAST,
        (false, false) => SUMMARY_STUB_TH_STYLE_MIDDLE,
    };
    let td_base = match (is_first, is_last) {
        (true, true) => SUMMARY_TD_STYLE_FIRST_LAST,
        (true, false) => SUMMARY_TD_STYLE_FIRST,
        (false, true) => SUMMARY_TD_STYLE_LAST,
        (false, false) => SUMMARY_TD_STYLE_MIDDLE,
    };

    let summary_stub_id = format!("summary_stub_{}_{}", group_name, summary_idx);

    for (i, (cell, col)) in sr.cells.iter().zip(columns.iter()).enumerate() {
        let is_stub = Some(i) == stub_col;
        let frag = if is_stub {
            // Summary stub renders the aggregate label, always left-aligned.
            let mut style = format!("{} text-align: left;", stub_base);
            if is_first {
                style.push_str(" border-top-width: 2px;");
            }
            format!(
                "<th id=\"{}\" scope=\"row\" class=\"gt_row gt_left gt_stub gt_summary_row{}\" \
                 style=\"{}\" valign=\"middle\" bgcolor=\"#FFFFFF\" align=\"left\">{}</th>",
                summary_stub_id,
                class_suffix,
                style,
                html_escape(&sr.label),
            )
        } else {
            let align = col.align;
            let align_str = match align {
                ColAlign::Left => "left",
                ColAlign::Right => "right",
                ColAlign::Center => "center",
            };
            let gt_class = align.gt_class();
            let mut style = format!("{} text-align: {};", td_base, align_str);
            if align.tabular_nums() {
                style.push_str(" font-variant-numeric: tabular-nums;");
            }
            if is_first {
                style.push_str(" border-top-width: 2px;");
            }
            let value = cell.as_deref().unwrap_or("\u{2014}");
            let cell_text = if col.raw_html {
                value.to_string()
            } else {
                html_escape(value)
            };
            format!(
                "<td headers=\"{} {} {}\" class=\"gt_row {} gt_summary_row{}\" style=\"{}\" \
                 valign=\"middle\" bgcolor=\"#FFFFFF\" align=\"{}\">{}</td>",
                group_name,
                summary_stub_id,
                col.name,
                gt_class,
                class_suffix,
                style,
                align_str,
                cell_text
            )
        };
        if i == 0 {
            out.push_str(&frag);
        } else {
            out.push('\n');
            out.push_str(&frag);
        }
    }

    out.push_str("</tr>\n");
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
