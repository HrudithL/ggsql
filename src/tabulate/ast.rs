//! Typed AST nodes for TABULATE statements.
//!
//! These types are parallel to the VISUALISE AST in `src/plot/`; they must
//! not be merged with or added to the existing visualise-side AST enums.

/// A complete TABULATE statement parsed from a ggsql query.
#[derive(Debug, Clone)]
pub struct TabulateStmt {
    /// Column selection: empty means `*` (all columns from source).
    pub columns: Vec<String>,
    /// `FROM <source>` override; None when the preceding SELECT provides data.
    pub from_source: Option<String>,
    /// Zero or more FORMAT clauses.
    pub format_clauses: Vec<FormatClause>,
    /// Single LABEL clause (collapsed across the at-most-one allowed clause).
    pub label: Option<LabelClause>,
    /// Zero or more SCALE clauses.
    pub scale_clauses: Vec<ScaleClause>,
}

/// A `SCALE <aesthetic> [FROM (min, max)] TO <palette>|(c1, c2, ...) [VIA <id>]
///   SETTING target => <col>|(col, ...)` clause.
#[derive(Debug, Clone)]
pub struct ScaleClause {
    /// Currently always `background`.
    pub aesthetic: String,
    /// Explicit numeric domain (min, max) from `FROM (min, max)`. When
    /// `None`, the domain is inferred from the target columns' data.
    pub domain: Option<(f64, f64)>,
    /// Output colour spec from `TO ...`.
    pub palette: ScalePalette,
    /// Optional transform from `VIA <id>` (only `log10` is recognised).
    pub transform: Option<String>,
    /// Target columns the scale applies to (from `SETTING target => ...`).
    pub target_cols: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ScalePalette {
    /// Explicit list of colour stops as parsed string literals (e.g.
    /// `('white', 'darkgreen')` or `('#f7fbff', '#08306b')`).
    Stops(Vec<String>),
    /// Named built-in palette (`viridis`, `RdYlGn`, ...).
    Named(String),
}

/// A `LABEL key => '...', ...` clause.
#[derive(Debug, Clone, Default)]
pub struct LabelClause {
    /// `title => '...'`
    pub title: Option<String>,
    /// `subtitle => '...'`
    pub subtitle: Option<String>,
    /// `caption => '...'` (rendered as gt's sourcenote).
    pub caption: Option<String>,
    /// All other `<id> => '...'` pairs (column or spanner labels, in source order).
    pub renames: Vec<(String, String)>,
}

/// A `FORMAT [SPAN|STUB] col [, col] [AS id] [SETTING ...] [RENAMING ...]` clause.
#[derive(Debug, Clone)]
pub struct FormatClause {
    /// Modifier keyword (`SPAN`, `STUB`, or neither).
    pub mode: FormatMode,
    /// Columns this clause targets.
    pub columns: Vec<String>,
    /// Spanner id from `AS <id>` (only relevant for `SPAN`).
    pub span_id: Option<String>,
    /// Key → value settings from `SETTING key => value, ...`.
    pub settings: Vec<FormatSetting>,
    /// Rename rules from `RENAMING lhs => rhs, ...`.
    pub renamings: Vec<FormatRenaming>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormatMode {
    /// Default: apply column-level settings directly.
    None,
    /// `FORMAT SPAN ...` — group columns under a spanner.
    Span,
    /// `FORMAT STUB ...` — designate stub / row-label column.
    Stub,
}

/// A single `key => value` pair from `SETTING`.
#[derive(Debug, Clone)]
pub struct FormatSetting {
    pub key: String,
    pub value: SettingValue,
}

#[derive(Debug, Clone)]
pub enum SettingValue {
    String(String),
    Number(f64),
    Bool(bool),
}

/// A single renaming rule from `RENAMING`.
#[derive(Debug, Clone)]
pub struct FormatRenaming {
    pub lhs: RenamingLhs,
    pub rhs: String,
}

#[derive(Debug, Clone)]
pub enum RenamingLhs {
    /// `*` — all values (formatter).
    Wildcard,
    /// `null` — NA / missing.
    Null,
    /// `0` — zero.
    Zero,
    /// Exact numeric literal.
    Number(f64),
    /// Exact string literal.
    Literal(String),
    /// Bare identifier (column name or custom lhs).
    Identifier(String),
}
