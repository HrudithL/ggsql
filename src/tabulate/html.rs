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
    out.push_str(&format!(
        "  <thead style=\"{}\">\n    <tr class=\"gt_col_headings\" style=\"{}\">\n",
        THEAD_STYLE, THEAD_TR_STYLE
    ));
    for col in &table.columns {
        out.push_str(&render_th(col.align, &col.name));
    }
    out.push_str("    </tr>\n  </thead>\n");

    // tbody
    out.push_str(&format!(
        "  <tbody class=\"gt_table_body\" style=\"{}\">\n",
        TBODY_STYLE
    ));
    for row in &table.rows {
        out.push_str(&render_tr(row, &table.columns));
    }
    out.push_str("  </tbody>\n  </table></div>");

    out
}

// ============================================================================
// Helpers
// ============================================================================

fn generate_id() -> String {
    let u = Uuid::new_v4().to_string().replace('-', "");
    u[..10].to_string()
}

fn render_th(align: ColAlign, label: &str) -> String {
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
    format!(
        "      <th class=\"gt_col_heading gt_columns_bottom_border {}\" \
         rowspan=\"1\" colspan=\"1\" scope=\"col\" id=\"{}\" \
         style=\"{}\" bgcolor=\"#FFFFFF\" valign=\"bottom\" align=\"{}\">{}</th>\n",
        gt_class,
        label,
        style,
        align_str,
        html_escape(label)
    )
}

fn render_tr(row: &[String], columns: &[ColMeta]) -> String {
    let mut out = String::new();
    out.push_str(&format!("    <tr style=\"{}\">", TR_STYLE));
    for (i, (value, col)) in row.iter().zip(columns.iter()).enumerate() {
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
        let td = format!(
            "<td headers=\"{}\" class=\"gt_row {}\" style=\"{}\" \
             valign=\"middle\" align=\"{}\">{}</td>",
            col.name,
            gt_class,
            style,
            align_str,
            html_escape(value)
        );
        if i == 0 {
            out.push_str(&td);
        } else {
            out.push('\n');
            out.push_str(&td);
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
