//! Tree-sitter → [`TabulateStmt`] lowering for the TABULATE sub-language.

use crate::parser::SourceTree;
use crate::tabulate::ast::{
    FormatClause, FormatMode, FormatRenaming, FormatSetting, RenamingLhs, SettingValue,
    TabulateStmt,
};
use crate::{GgsqlError, Result};

/// Parse every `tabulate_statement` in the query and return the first one.
///
/// Phase 1 supports exactly one TABULATE per query; future phases may
/// extend this.
pub fn parse_tabulate(source: &SourceTree<'_>) -> Result<TabulateStmt> {
    let root = source.root();
    let stmt_nodes = source.find_nodes(&root, "(tabulate_statement) @s");
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

    Ok(TabulateStmt {
        columns,
        from_source,
        format_clauses,
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

    // --- column names (all identifier children before AS / SETTING / RENAMING) ---
    // Walk direct children of format_clause, collecting identifiers that appear
    // before any setting/renaming block.
    let mut columns: Vec<String> = Vec::new();
    let mut span_id: Option<String> = None;

    {
        // Use the raw tree-sitter children walk
        let mut cursor = node.walk();
        let children = node.children(&mut cursor);
        let mut after_as = false;
        for child in children {
            let kind = child.kind();
            match kind {
                "format_keyword" | "format_mode" => continue,
                "identifier" => {
                    if after_as {
                        span_id = Some(source.get_text(&child));
                        after_as = false;
                    } else {
                        columns.push(source.get_text(&child));
                    }
                }
                "," => continue,
                "AS" | "as" => {
                    // case-insensitive AS handled via grammar token
                    after_as = true;
                }
                _ if kind.eq_ignore_ascii_case("as") => {
                    after_as = true;
                }
                "format_setting_block" | "format_renaming_block" => break,
                _ => {}
            }
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
                let unquoted = t.trim_matches('\'').to_string();
                return Ok(SettingValue::String(unquoted));
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
                rhs = Some(t.trim_matches('\'').to_string());
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
                    RenamingLhs::Literal(t.trim_matches('\'').to_string())
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
