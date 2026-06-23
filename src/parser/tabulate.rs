//! Tree-sitter → [`TabulateStmt`] lowering for the TABULATE sub-language.

use crate::parser::SourceTree;
use crate::tabulate::ast::{
    FacetClause, FacetSetting, FacetValue, FormatClause, FormatMode, FormatRenaming, FormatSetting,
    HighlightClause, HighlightSetting, LabelClause, RenamingLhs, ScaleClause, ScalePalette,
    SettingValue, TabulateStmt,
};
use crate::{GgsqlError, Result};

/// Parse every `tabulate_statement` in the query and return the first one.
///
/// Phase 1 supports exactly one TABULATE per query; future phases may
/// extend this.
pub fn parse_tabulate(source: &SourceTree<'_>) -> Result<TabulateStmt> {
    let root = source.root();
    let stmt_nodes = source.find_nodes(&root, "(tabulate_statement) @s");

    // VISUALISE and TABULATE are mutually exclusive in a single query.
    // Reject any query containing both with a clear error.
    let viz_nodes = source.find_nodes(&root, "(visualise_statement) @viz");
    if !viz_nodes.is_empty() && !stmt_nodes.is_empty() {
        return Err(GgsqlError::ParseError(
            "VISUALISE and TABULATE are mutually exclusive in a single \
             query; use separate queries for each output mode."
                .to_string(),
        ));
    }

    let stmt = stmt_nodes.into_iter().next().ok_or_else(|| {
        GgsqlError::ParseError("No TABULATE statement found in query".to_string())
    })?;

    // --- column list ---
    let col_list_nodes = source.find_nodes(&stmt, "(tab_col_list) @c");
    let columns: Vec<String> = if let Some(col_list) = col_list_nodes.into_iter().next() {
        let text = source.get_text(&col_list);
        if text.trim() == "*" {
            vec![]
        } else {
            // Find all identifier nodes directly under tab_col_list
            source
                .find_nodes(&col_list, "(identifier) @id")
                .into_iter()
                .map(|n| source.get_text(&n))
                .collect()
        }
    } else {
        vec![]
    };

    // --- FROM source ---
    let from_nodes = source.find_nodes(&stmt, "(tab_from_clause) @f");
    let from_source = from_nodes.into_iter().next().and_then(|from_node| {
        // field 'source' is an identifier
        let id_nodes = source.find_nodes(&from_node, "(identifier) @id");
        id_nodes.into_iter().next().map(|n| source.get_text(&n))
    });

    // --- FORMAT clauses ---
    let format_nodes = source.find_nodes(&stmt, "(format_clause) @fc");
    let format_clauses: Vec<FormatClause> = format_nodes
        .into_iter()
        .map(|fc_node| parse_format_clause(source, &fc_node))
        .collect::<Result<Vec<_>>>()?;

    // --- LABEL clause (at most one per TABULATE) ---
    let label_nodes = source.find_nodes(&stmt, "(label_clause) @lc");
    if label_nodes.len() > 1 {
        return Err(GgsqlError::ParseError(
            "TABULATE allows at most one LABEL clause".to_string(),
        ));
    }
    let label = label_nodes
        .into_iter()
        .next()
        .map(|n| parse_label_clause(source, &n))
        .transpose()?;

    // --- SCALE clauses ---
    let scale_nodes = source.find_nodes(&stmt, "(tab_scale_clause) @sc");
    let scale_clauses: Vec<ScaleClause> = scale_nodes
        .into_iter()
        .map(|n| parse_scale_clause(source, &n))
        .collect::<Result<Vec<_>>>()?;

    // --- HIGHLIGHT clauses ---
    let hl_nodes = source.find_nodes(&stmt, "(tab_highlight_clause) @hc");
    let highlight_clauses: Vec<HighlightClause> = hl_nodes
        .into_iter()
        .map(|n| parse_highlight_clause(source, &n))
        .collect::<Result<Vec<_>>>()?;

    // --- FACET clause (at most one) ---
    let facet_nodes = source.find_nodes(&stmt, "(tab_facet_clause) @fc");
    if facet_nodes.len() > 1 {
        return Err(GgsqlError::ParseError(
            "TABULATE allows at most one FACET clause".to_string(),
        ));
    }
    let facet = facet_nodes
        .into_iter()
        .next()
        .map(|n| parse_facet_clause(source, &n))
        .transpose()?;

    Ok(TabulateStmt {
        columns,
        from_source,
        format_clauses,
        label,
        scale_clauses,
        highlight_clauses,
        facet,
    })
}

fn parse_format_clause(
    source: &SourceTree<'_>,
    node: &tree_sitter::Node<'_>,
) -> Result<FormatClause> {
    // --- mode (SPAN / STUB / None) ---
    let mode = if source.find_nodes(node, "(format_mode) @m").is_empty() {
        FormatMode::None
    } else {
        let mode_nodes = source.find_nodes(node, "(format_mode) @m");
        let mode_text = mode_nodes
            .into_iter()
            .next()
            .map(|n| source.get_text(&n).to_uppercase())
            .unwrap_or_default();
        if mode_text == "SPAN" {
            FormatMode::Span
        } else if mode_text == "STUB" {
            FormatMode::Stub
        } else {
            FormatMode::None
        }
    };

    // --- column names + optional span_id ---
    // Columns are direct identifier children up to (but not including) the one
    // captured by the `span_id` field. Settings/renamings are in their own blocks.
    // `FORMAT *` (the wildcard branch in the grammar) is represented as a
    // single column "*"; the executor expands it to every visible column.
    let span_id_node = node.child_by_field_name("span_id");
    let span_id: Option<String> = span_id_node.map(|n| source.get_text(&n));

    let has_wildcard = !source.find_nodes(node, "(format_wildcard) @w").is_empty();

    let mut columns: Vec<String> = Vec::new();
    if has_wildcard {
        columns.push("*".to_string());
    } else {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != "identifier" {
                continue;
            }
            if let Some(ref sid) = span_id_node {
                if child.id() == sid.id() {
                    continue;
                }
            }
            columns.push(source.get_text(&child));
        }
    }

    // --- settings ---
    let setting_block_nodes = source.find_nodes(node, "(format_setting_block) @sb");
    let settings: Vec<FormatSetting> = if let Some(sb) = setting_block_nodes.into_iter().next() {
        source
            .find_nodes(&sb, "(format_setting_pair) @sp")
            .into_iter()
            .map(|pair| parse_setting_pair(source, &pair))
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![]
    };

    // --- renamings ---
    let renaming_block_nodes = source.find_nodes(node, "(format_renaming_block) @rb");
    let renamings: Vec<FormatRenaming> = if let Some(rb) = renaming_block_nodes.into_iter().next() {
        source
            .find_nodes(&rb, "(format_renaming_pair) @rp")
            .into_iter()
            .map(|pair| parse_renaming_pair(source, &pair))
            .collect::<Result<Vec<_>>>()?
    } else {
        vec![]
    };

    Ok(FormatClause {
        mode,
        columns,
        span_id,
        settings,
        renamings,
    })
}

fn parse_setting_pair(
    source: &SourceTree<'_>,
    node: &tree_sitter::Node<'_>,
) -> Result<FormatSetting> {
    let name_nodes = source.find_nodes(node, "(identifier) @n");
    let key = name_nodes
        .into_iter()
        .next()
        .map(|n| source.get_text(&n))
        .ok_or_else(|| GgsqlError::ParseError("Missing key in FORMAT SETTING".to_string()))?;

    if key.eq_ignore_ascii_case("units") {
        return Err(GgsqlError::ParseError(
            "FORMAT SETTING `units` was removed; put units in the LABEL text \
             instead, e.g. LABEL land_area => 'Land Area (km²)'"
                .to_string(),
        ));
    }

    let value = parse_setting_value(source, node)?;
    Ok(FormatSetting { key, value })
}

fn parse_setting_value(
    source: &SourceTree<'_>,
    node: &tree_sitter::Node<'_>,
) -> Result<SettingValue> {
    // Look for string, number, or boolean as children
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    for child in children {
        match child.kind() {
            "string" => {
                let t = source.get_text(&child);
                return Ok(SettingValue::String(unquote_string(&t)));
            }
            "number" => {
                let t = source.get_text(&child);
                let n: f64 = t
                    .parse()
                    .map_err(|_| GgsqlError::ParseError(format!("Invalid number '{}'", t)))?;
                return Ok(SettingValue::Number(n));
            }
            "boolean" => {
                let t = source.get_text(&child);
                return Ok(SettingValue::Bool(t == "true"));
            }
            _ => {}
        }
    }
    Err(GgsqlError::ParseError(
        "Missing value in FORMAT SETTING".to_string(),
    ))
}

fn parse_renaming_pair(
    source: &SourceTree<'_>,
    node: &tree_sitter::Node<'_>,
) -> Result<FormatRenaming> {
    let mut lhs: Option<RenamingLhs> = None;
    let mut rhs: Option<String> = None;
    let mut past_arrow = false;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let kind = child.kind();
        if kind == "=>" {
            past_arrow = true;
            continue;
        }
        if past_arrow {
            if kind == "string" {
                let t = source.get_text(&child);
                rhs = Some(unquote_string(&t));
            }
        } else {
            lhs = Some(match kind {
                "*" => RenamingLhs::Wildcard,
                "number" => {
                    let t = source.get_text(&child);
                    let n: f64 = t.parse().unwrap_or(0.0);
                    if (n - 0.0).abs() < f64::EPSILON {
                        RenamingLhs::Zero
                    } else {
                        RenamingLhs::Number(n)
                    }
                }
                "string" => {
                    let t = source.get_text(&child);
                    RenamingLhs::Literal(unquote_string(&t))
                }
                "identifier" => {
                    let t = source.get_text(&child);
                    if t.eq_ignore_ascii_case("null") {
                        RenamingLhs::Null
                    } else {
                        RenamingLhs::Identifier(t)
                    }
                }
                _ => continue,
            });
        }
    }

    Ok(FormatRenaming {
        lhs: lhs.unwrap_or(RenamingLhs::Wildcard),
        rhs: rhs.unwrap_or_default(),
    })
}

/// Extract the FROM table name from the SQL portion of the query (if any).
///
/// For queries like `SELECT ... FROM gtcars TABULATE ...`, returns `Some("gtcars")`.
pub fn extract_sql_from_table(source: &SourceTree<'_>) -> Option<String> {
    let root = source.root();
    // Find the first from_clause inside a sql_portion
    let sql_nodes = source.find_nodes(&root, "(sql_portion) @sql");
    let sql_node = sql_nodes.into_iter().next()?;

    // The table name is inside from_clause > table_ref > qualified_name > identifier
    let table_nodes = source.find_nodes(
        &sql_node,
        "(from_clause (table_ref table: (qualified_name (identifier) @t)))",
    );
    table_nodes.into_iter().next().map(|n| source.get_text(&n))
}

/// Unquote a single-quoted string literal as produced by the grammar.
///
/// Handles backslash escapes (`\n`, `\t`, `\r`, `\\`, `\'`, `\"`). Unknown
/// `\x` escapes are kept verbatim. There is no `''` doubled-quote escape;
/// embed a single quote with `\'`.
fn unquote_string(text: &str) -> String {
    let inner = text
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(text);
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some('\'') => out.push('\''),
                Some('"') => out.push('"'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_label_clause(
    source: &SourceTree<'_>,
    node: &tree_sitter::Node<'_>,
) -> Result<LabelClause> {
    let mut out = LabelClause::default();
    let assignments = source.find_nodes(node, "(label_assignment) @la");
    for la in assignments {
        // name (identifier inside label_type)
        let name_nodes = source.find_nodes(&la, "(identifier) @n");
        let name = name_nodes
            .into_iter()
            .next()
            .map(|n| source.get_text(&n))
            .ok_or_else(|| GgsqlError::ParseError("LABEL key missing".to_string()))?;

        // value (string or null_literal)
        let mut value: Option<String> = None;
        let mut cursor = la.walk();
        for child in la.children(&mut cursor) {
            match child.kind() {
                "string" => {
                    value = Some(unquote_string(&source.get_text(&child)));
                    break;
                }
                "null_literal" => {
                    value = None;
                    break;
                }
                _ => {}
            }
        }
        let Some(value) = value else { continue };
        match name.to_ascii_lowercase().as_str() {
            "title" => out.title = Some(value),
            "subtitle" => out.subtitle = Some(value),
            "caption" => out.caption = Some(value),
            _ => out.renames.push((name, value)),
        }
    }
    Ok(out)
}

fn parse_scale_clause(
    source: &SourceTree<'_>,
    node: &tree_sitter::Node<'_>,
) -> Result<ScaleClause> {
    // aesthetic: field `aesthetic` (identifier directly under the clause)
    let aesthetic = node
        .child_by_field_name("aesthetic")
        .map(|n| source.get_text(&n))
        .ok_or_else(|| GgsqlError::ParseError("SCALE missing aesthetic".to_string()))?;

    // domain from `(tab_scale_from)`: two numbers
    let domain = source
        .find_nodes(node, "(tab_scale_from) @f")
        .into_iter()
        .next()
        .and_then(|f| {
            let nums = source.find_nodes(&f, "(number) @n");
            if nums.len() >= 2 {
                let a: f64 = source.get_text(&nums[0]).parse().ok()?;
                let b: f64 = source.get_text(&nums[1]).parse().ok()?;
                Some((a, b))
            } else {
                None
            }
        });

    // palette from `(tab_scale_to)`: either `palette: identifier` (named)
    // or one-or-more `(string)` children (explicit stops).
    let palette = source
        .find_nodes(node, "(tab_scale_to) @t")
        .into_iter()
        .next()
        .map(|to_node| {
            if let Some(p) = to_node.child_by_field_name("palette") {
                ScalePalette::Named(source.get_text(&p))
            } else {
                let stops: Vec<String> = source
                    .find_nodes(&to_node, "(string) @s")
                    .into_iter()
                    .map(|n| unquote_string(&source.get_text(&n)))
                    .collect();
                ScalePalette::Stops(stops)
            }
        })
        .ok_or_else(|| GgsqlError::ParseError("SCALE missing TO clause".to_string()))?;

    // optional VIA <id>
    let transform = source
        .find_nodes(node, "(tab_scale_via) @v")
        .into_iter()
        .next()
        .and_then(|v| v.child_by_field_name("transform"))
        .map(|n| source.get_text(&n));

    // SETTING target => <col>|(col, col, ...)  — collect all identifiers
    // that follow `=>` inside the setting block.
    let target_cols = source
        .find_nodes(node, "(tab_scale_setting) @s")
        .into_iter()
        .next()
        .map(|s_node| -> Result<Vec<String>> {
            let key_node = s_node.child_by_field_name("key");
            let mut ids = Vec::new();
            let mut cursor = s_node.walk();
            let mut after_arrow = false;
            let mut saw_paren = false;
            for child in s_node.children(&mut cursor) {
                if child.kind() == "=>" {
                    after_arrow = true;
                    continue;
                }
                if !after_arrow {
                    continue;
                }
                if child.kind() == "(" {
                    saw_paren = true;
                }
                if child.kind() == "identifier" {
                    if let Some(k) = &key_node {
                        if child.id() == k.id() {
                            continue;
                        }
                    }
                    ids.push(source.get_text(&child));
                }
            }
            // C1: enforce parenthesized form even for a single column.
            if !ids.is_empty() && !saw_paren {
                return Err(GgsqlError::ParseError(
                    "SCALE target requires a parenthesized list, e.g. \
                     `target => (col)` or `target => (col1, col2)`"
                        .to_string(),
                ));
            }
            Ok(ids)
        })
        .transpose()?
        .unwrap_or_default();

    Ok(ScaleClause {
        aesthetic,
        domain,
        palette,
        transform,
        target_cols,
    })
}

fn parse_highlight_clause(
    source: &SourceTree<'_>,
    node: &tree_sitter::Node<'_>,
) -> Result<HighlightClause> {
    // Columns: the bare identifier children that appear before the
    // (filter_clause) — they are listed comma-separated right after
    // the HIGHLIGHT keyword.
    let filter_node = source
        .find_nodes(node, "(filter_clause) @fc")
        .into_iter()
        .next()
        .ok_or_else(|| GgsqlError::ParseError("HIGHLIGHT missing FILTER clause".to_string()))?;

    let mut columns: Vec<String> = Vec::new();
    {
        let filter_start = filter_node.start_byte();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != "identifier" {
                continue;
            }
            if child.start_byte() >= filter_start {
                break;
            }
            columns.push(source.get_text(&child));
        }
    }
    if columns.is_empty() {
        return Err(GgsqlError::ParseError(
            "HIGHLIGHT requires at least one column".to_string(),
        ));
    }

    // FILTER expression is captured raw and forwarded to the SQL backend.
    let filter = source
        .find_nodes(&filter_node, "(filter_expression) @fe")
        .into_iter()
        .next()
        .map(|n| source.get_text(&n).trim().to_string())
        .ok_or_else(|| GgsqlError::ParseError("HIGHLIGHT FILTER missing expression".to_string()))?;

    // SETTING <k> => <v>, ... (optional)
    let settings: Vec<HighlightSetting> = source
        .find_nodes(node, "(tab_highlight_setting) @hs")
        .into_iter()
        .next()
        .map(|hs| {
            source
                .find_nodes(&hs, "(tab_highlight_pair) @hp")
                .into_iter()
                .map(|pair| parse_highlight_pair(source, &pair))
                .collect::<Result<Vec<_>>>()
        })
        .unwrap_or_else(|| Ok(Vec::new()))?;

    Ok(HighlightClause {
        columns,
        filter,
        settings,
    })
}

fn parse_highlight_pair(
    source: &SourceTree<'_>,
    node: &tree_sitter::Node<'_>,
) -> Result<HighlightSetting> {
    let key = node
        .child_by_field_name("key")
        .map(|n| source.get_text(&n))
        .ok_or_else(|| GgsqlError::ParseError("HIGHLIGHT SETTING missing key".to_string()))?;
    let value = parse_setting_value(source, node)?;
    Ok(HighlightSetting { key, value })
}

fn parse_facet_clause(
    source: &SourceTree<'_>,
    node: &tree_sitter::Node<'_>,
) -> Result<FacetClause> {
    let group_col = node
        .child_by_field_name("group")
        .map(|n| source.get_text(&n))
        .ok_or_else(|| GgsqlError::ParseError("FACET missing group column".to_string()))?;

    let settings: Vec<FacetSetting> = source
        .find_nodes(node, "(tab_facet_setting_block) @b")
        .into_iter()
        .next()
        .map(|sb| {
            source
                .find_nodes(&sb, "(tab_facet_pair) @p")
                .into_iter()
                .map(|p| parse_facet_pair(source, &p))
                .collect::<Result<Vec<_>>>()
        })
        .unwrap_or_else(|| Ok(Vec::new()))?;

    Ok(FacetClause {
        group_col,
        settings,
    })
}

fn parse_facet_pair(source: &SourceTree<'_>, node: &tree_sitter::Node<'_>) -> Result<FacetSetting> {
    let key = node
        .child_by_field_name("key")
        .map(|n| source.get_text(&n))
        .ok_or_else(|| GgsqlError::ParseError("FACET SETTING missing key".to_string()))?;

    // The value is one of: string, number, boolean, identifier, ident list, str list.
    let value = node
        .child_by_field_name("value")
        .ok_or_else(|| GgsqlError::ParseError("FACET SETTING missing value".to_string()))?;

    let parsed = match value.kind() {
        "string" => FacetValue::String(unquote_string(&source.get_text(&value))),
        "number" => {
            let t = source.get_text(&value);
            FacetValue::Number(
                t.parse()
                    .map_err(|_| GgsqlError::ParseError(format!("Invalid number '{}'", t)))?,
            )
        }
        "boolean" => FacetValue::Bool(source.get_text(&value) == "true"),
        "identifier" => FacetValue::Identifier(source.get_text(&value)),
        "tab_facet_id_list" => {
            let ids: Vec<String> = source
                .find_nodes(&value, "(identifier) @id")
                .into_iter()
                .map(|n| source.get_text(&n))
                .collect();
            FacetValue::IdentList(ids)
        }
        "tab_facet_str_list" => {
            let strs: Vec<String> = source
                .find_nodes(&value, "(string) @s")
                .into_iter()
                .map(|n| unquote_string(&source.get_text(&n)))
                .collect();
            FacetValue::StrList(strs)
        }
        other => {
            return Err(GgsqlError::ParseError(format!(
                "Unsupported FACET value kind '{}'",
                other
            )))
        }
    };

    // Validate aggregate function names: 'mean' / 'average' are no longer
    // accepted (use 'avg', the SQL-canonical spelling).
    if key.eq_ignore_ascii_case("aggregate") {
        let names: Vec<&str> = match &parsed {
            FacetValue::String(v) => vec![v.as_str()],
            FacetValue::Identifier(v) => vec![v.as_str()],
            FacetValue::StrList(v) => v.iter().map(|s| s.as_str()).collect(),
            FacetValue::IdentList(v) => v.iter().map(|s| s.as_str()).collect(),
            _ => Vec::new(),
        };
        for n in names {
            let lc = n.to_ascii_lowercase();
            if lc == "mean" || lc == "average" {
                return Err(GgsqlError::ParseError(format!(
                    "FACET aggregate '{}' is not accepted; use 'avg' instead",
                    n,
                )));
            }
        }
    }

    // C1: `target` must be a parenthesized list, even for a single column.
    // The bareword form `target => <col>` is rejected.
    if key.eq_ignore_ascii_case("target")
        && matches!(parsed, FacetValue::Identifier(_) | FacetValue::String(_))
    {
        return Err(GgsqlError::ParseError(
            "FACET target requires a parenthesized list, e.g. `target => (col)` \
             or `target => (col1, col2)`"
                .to_string(),
        ));
    }
    Ok(FacetSetting { key, value: parsed })
}
