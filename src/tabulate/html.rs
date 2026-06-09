//! HTML writer for TABULATE — produces output matching `gt::as_raw_html()`.
//!
//! gt 1.3+ inlines all styles via `style=` attributes; the vendored CSS file
//! is essentially empty. We emit the same inline-style HTML structure.

use super::GT_DEFAULT_CSS;
use crate::tabulate::execute::{ColAlign, ColMeta, TableIr};
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

const THEAD_STYLE: &str = "border-style: none;";

const THEAD_TR_STYLE: &str = concat!(
    "border-style: none; border-top-style: solid; ",
    "border-top-width: 2px; border-top-color: #D3D3D3; border-bottom-style: solid; ",
    "border-bottom-width: 2px; border-bottom-color: #D3D3D3; border-left-style: none; ",
    "border-left-width: 1px; border-left-color: #D3D3D3; border-right-style: none; ",
    "border-right-width: 1px; border-right-color: #D3D3D3;"
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
const STUB_TH_STYLE: &str = concat!(
    "border-style: none; padding-top: 8px; padding-bottom: 8px; margin: 10px; ",
    "border-top-style: solid; border-top-width: 1px; border-top-color: #D3D3D3; ",
    "border-left-style: none; border-left-width: 1px; border-left-color: #D3D3D3; ",
    "vertical-align: middle; overflow-x: hidden; ",
    "color: #333333; background-color: #FFFFFF; font-size: 100%; ",
    "font-weight: initial; text-transform: inherit; ",
    "border-right-style: solid; border-right-width: 2px; border-right-color: #D3D3D3; ",
    "padding-left: 5px; padding-right: 5px; text-align: left;"
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
         style=\"{}\" bgcolor=\"#FFFFFF\">\n",
        TABLE_STYLE
    ));

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

    out.push_str(&format!(
        "    <tr class=\"gt_col_headings\" style=\"{}\">\n",
        THEAD_TR_STYLE
    ));
    for (i, col) in table.columns.iter().enumerate() {
        let is_stub = Some(i) == table.stub_col;
        out.push_str(&render_th(col, is_stub));
    }
    out.push_str("    </tr>\n  </thead>\n");

    // tbody
    out.push_str(&format!(
        "  <tbody class=\"gt_table_body\" style=\"{}\">\n",
        TBODY_STYLE
    ));
    for (row_idx, row) in table.rows.iter().enumerate() {
        out.push_str(&render_tr(row, &table.columns, table.stub_col, row_idx));
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

fn render_th(col: &ColMeta, is_stub: bool) -> String {
    let align_str = match col.align {
        ColAlign::Left => "left",
        ColAlign::Right => "right",
        ColAlign::Center => "center",
    };
    let gt_class = col.align.gt_class();
    let mut style = format!("{} text-align: {};", TH_BASE_STYLE, align_str);
    if col.align.tabular_nums() {
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
         rowspan=\"1\" colspan=\"1\" scope=\"col\" id=\"{}\" \
         style=\"{}\" bgcolor=\"#FFFFFF\" valign=\"bottom\" align=\"{}\">{}</th>\n",
        gt_class,
        id,
        style,
        align_str,
        html_escape(&col.label)
    )
}

fn render_tr(
    row: &[String],
    columns: &[ColMeta],
    stub_col: Option<usize>,
    row_idx: usize,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("    <tr style=\"{}\">", TR_STYLE));
    // Stub id for this row (used as the cell id of the stub <th> and as the
    // first token of every sibling cell's `headers` attribute).
    let stub_id = stub_col.map(|_| format!("stub_1_{}", row_idx + 1));
    for (i, (value, col)) in row.iter().zip(columns.iter()).enumerate() {
        let cell = if Some(i) == stub_col {
            // Stub <th> in tbody.
            format!(
                "<th id=\"{}\" scope=\"row\" class=\"gt_row gt_left gt_stub\" style=\"{}\" \
                 valign=\"middle\" bgcolor=\"#FFFFFF\" align=\"left\">{}</th>",
                stub_id.as_deref().unwrap_or(""),
                STUB_TH_STYLE,
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
            let mut style = format!("{} text-align: {};", TD_BASE_STYLE, align_str);
            if align.tabular_nums() {
                style.push_str(" font-variant-numeric: tabular-nums;");
            }
            let headers = match &stub_id {
                Some(sid) => format!("{} {}", sid, col.name),
                None => col.name.clone(),
            };
            format!(
                "<td headers=\"{}\" class=\"gt_row {}\" style=\"{}\" \
                 valign=\"middle\" align=\"{}\">{}</td>",
                headers,
                gt_class,
                style,
                align_str,
                html_escape(value)
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

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
