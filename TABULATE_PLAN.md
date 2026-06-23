# TABULATE — implementation plan for ggsql

This document is the working implementation plan for the `TABULATE` clause
in this repo. The upstream spec in `/spec/` is **authoritative for syntax
and semantics**; this plan maps that spec onto concrete modules under
`src/tabulate/` and records implementation choices.

Read in this order:

1. [AGENTS.md](AGENTS.md) — agent rules, branches, stop conditions.
2. [/spec/GTSQL_AGENTBUILD_SPEC.md](../gtsql/GTSQL_AGENTBUILD_SPEC.md) —
   goal, oracle, normalization rule, phase order, pass gate, sandbox.
3. [/spec/GTSQL_PLAN.md](../gtsql/GTSQL_PLAN.md) — language reference.
4. [/spec/GTSQL_EXAMPLES.qmd](../gtsql/GTSQL_EXAMPLES.qmd) — example corpus
   (each example becomes a fixture).
5. This file — implementation surfaces and per-phase scope.

> **Fixture freshness.** The captured fixtures in `tests/fixtures/` were
> generated from an earlier spec revision. Their `{:num ...}` and
> `{:time ...}` bodies match this repo's canonical syntax — see §2 and
> §5 note 1 — and do not need re-capturing.

> **Printf body has no `%` introducer in this repo.** The upstream spec
> shows printf bodies with a leading `%` (e.g. `{:num %'d}`); this
> implementation rejects that form and accepts only the bare conversion
> spec (`{:num 'd}`, `{:num .3f}`, `{:num +.1f}`). See §5 note 1.

---

## 1. Top-level grammar

A query is `SELECT ... TABULATE ... <clauses>*`. The SQL preamble is
optional when `FROM` is given inside `TABULATE`. Six clauses can follow
`TABULATE`, each in any order:

- **VISUALISE and TABULATE are mutually exclusive in a single query.**
  The parser rejects any query containing both.

| Clause       | Repeatable | Subclauses               |
| ------------ | ---------- | ------------------------ |
| `FORMAT`     | yes        | `SETTING`, `RENAMING`    |
| `FACET`      | **once**   | `SETTING`                |
| `SCALE`      | yes        | `SETTING`, `FILTER`      |
| `HIGHLIGHT`  | yes        | `FILTER`, `SETTING`      |
| `LABEL`      | **once**   | —                        |

Grammar entry point: extend `tree-sitter-ggsql/grammar.js` `query` rule with
`repeat($.tabulate_statement)` alongside `repeat($.visualise_statement)`.

---

## 2. Syntax reference (fixture-accurate)

### Reserved barewords

The spec's \"Reserved barewords\" section lists every keyword that
requires quoting when used as a column name. In short:

- Top-level clause keywords: `TABULATE`, `FORMAT`, `FACET`, `SCALE`,
  `HIGHLIGHT`, `LABEL`.
- Subclause keywords: `SETTING`, `RENAMING`, `FILTER`, `FROM`.
- Inline keywords inside a `FORMAT`: `STUB`, `SPAN`, `AS`.
- Inline keywords inside `SCALE`: `TO`, `VIA`.
- `LABEL`-only reserved keys: `title`, `subtitle`, `caption`.

Inner `SETTING` keys (`target`, `aggregate`, `groups`, `side`, `label`,
`missing_text`, `width`, `align`, `hide`, `locale`, `face`, `color`,
`background`, `size`, `transform`, `decoration`, `foreground`,
`opacity`) are **not** reserved and may freely be used as column names
without quoting.

To use a reserved bareword as a column name, quote it with double quotes
(`"format"` for a column literally named `format`).

### 2.1 `TABULATE`

```
TABULATE [<col> [AS <new_name>], ...] [FROM <source>]
```

- Column list optional → all columns from the preceding `SELECT` in source order.
- `FROM <source>` optional when a `SELECT` precedes.
- `AS <new_name>` renames the **column itself** (a real SQL alias). Display
  labels are set in `LABEL`, never in `TABULATE`.

### 2.2 `FORMAT`

```
FORMAT [SPAN | STUB] <col> [, <col>...] [AS <id>]
  [SETTING <key> => <value>, ...]
  [RENAMING <lhs> => <string>, ...]
```

- `SPAN` — group columns or other spanners under a parent spanner.
  - `AS <id>` is **bareword** (no quotes). The identifier doubles as the
    default display label; override via `LABEL <id> => '<text>'`.
  - Spanner IDs and column names share a namespace; nesting is done by
    listing spanner IDs in a later `FORMAT SPAN ... AS <parent>`.
- `STUB` — designate column(s) as the row-label stub (gt `rowname_col`).
- Multiple columns may appear before `SETTING` / `RENAMING`; the subclauses
  apply to every listed column.

#### `FORMAT ... SETTING` keys

| Key       | Type    | gt mapping                          | Fixture |
| --------- | ------- | ----------------------------------- | ------- |
| `width`   | string  | `cols_width()` (e.g. `'150px'`)     | 10      |
| `align`   | string  | `cols_align()` (`left`/`center`/`right`/`auto`) | 10 |
| `hide`    | bool    | `cols_hide()`                       | 9       |
| `locale`  | string  | per-column locale for formatters    | 21      |

> To put a unit in a column header, set `LABEL <col> => 'Label (km²)'`.
> There is no `units` SETTING.

#### `FORMAT ... RENAMING` LHS

| LHS pattern  | Meaning                                | gt mapping        |
| ------------ | -------------------------------------- | ----------------- |
| `*`          | all values (formatter)                 | `fmt_*()`         |
| `null`       | NA / missing                           | `sub_missing()`   |
| `0`          | zero                                   | `sub_zero()`      |
| `'literal'`  | exact value match                      | `text_transform()`|

Precedence (highest first): literal > `null` > `0` > `*`. So
`RENAMING * => '{:num \'d}', 30 => 'Thirty'` formats everything but `30` as
an integer, and `30` becomes the literal string.

#### `FORMAT ... RENAMING` RHS — string interpolation

The RHS is a single-quoted string. Any text outside `{...}` is preserved as
a literal prefix/suffix. The two formatter mini-languages are
**`{:num ...}`** and **`{:time ...}`**, plus case transforms.

##### Numeric formatter `{:num <printf-body>}`

The body after `{:num ` is a `printf(3)` conversion specification
**without** the leading `%` (the implementation rejects a `%`
introducer; the surrounding `{:num ...}` already plays that role).
Examples used in the fixtures and examples corpus:

| Spec              | Means                                | Example call            |
| ----------------- | ------------------------------------ | ----------------------- |
| `{:num .3f}`      | 3 decimal places                     | `fmt_number`            |
| `{:num 'd}`       | integer with thousands separators    | `fmt_integer`           |
| `{:num '.2f}`     | float, 2 decimals, thousands seps    | `fmt_number(use_seps)`  |
| `{:num '.1f}`     | float, 1 decimal, thousands seps     | `fmt_number(use_seps)`  |
| `{:num .1f}%`     | percent, 1 decimal                   | `fmt_percent`           |
| `{:num .2e}`      | scientific notation, 2 decimals      | `fmt_scientific`        |
| `{:num +.1f}%`    | percent, forced sign, 1 decimal      | `fmt_percent(force_sign)`|
| `${:num '.2f}`    | currency — literal `$` prefix        | `fmt_currency("USD")`   |

Flags supported: `'` (locale-aware thousands separator), `+` (forced
sign), `0` (zero pad). Conversions supported: `d` (integer), `f` (fixed
float), `e` (scientific).

##### Percent semantics

A literal `%` suffix outside the `{...}` (e.g. `'{:num %.1f}%'`) triggers
gt's `fmt_percent()` behavior: the value is **multiplied by 100** before
formatting. Spec example 14: a proportion of `0.085` renders as `8.5%`.

##### Time formatter `{:time <strftime>}`

The body is a `strftime`-style format string. Per the spec the format
codes are standard strftime (`%d` for day, `%I` for 12-hour, `%H` for
24-hour, `%B` for month name, etc.).

**Implementation note (day/hour padding).** gt's named date styles
(`date_style = "day_month_year"`, `time_style = "h_m_p"`, ...) render
single-digit days and hours **unpadded** (“1 June 2026”, “3:45 PM”). The
spec examples use `%d` and `%I` in GTSQL queries while their R counterparts
call the named gt styles. The implementation must reproduce gt's output —
i.e. `%d` and `%I` inside `{:time ...}` render unpadded so the captured
HTML matches. Treat the strftime in `{:time ...}` as a gt-compatible
format layer, not a strict `strftime(3)` passthrough.

Per-column locale comes from `FORMAT ... SETTING locale => '...'`. There
is no global locale (spec open-question 6).

Examples from the corpus (spec syntax):

| Spec                                       | Example |
| ------------------------------------------ | ------- |
| `{:time %B %d, %Y}`                        | 16      |
| `{:time %d %B %Y}`                         | 17      |
| `{:time %I:%M %p}`                         | 17      |
| `{:time %A, %B %d, %Y at %I:%M %p}`        | 17      |
| `{:time %A %d %B %Y}` (`locale='fr'`)      | 21      |

##### Case transforms

| Spec        | Means       | Fixture |
| ----------- | ----------- | ------- |
| `{:Title}`  | title case  | 31      |
| `{:UPPER}`  | upper       | —       |
| `{:lower}`  | lower       | —       |
| `{}`        | as-is       | —       |

### 2.3 `FACET`

```
FACET [<group_col>]
  [SETTING target => <col> | (<col>, ...),
           aggregate => (<fn>, ...),
           groups => [<v>, ...],
           side => 'top' | 'bottom',
           label => '<l>' | ['<l>', ...],
           missing_text => '<s>']
```

- One `FACET` per query.
- `<group_col>` optional. Present → `gt(groupname_col = ...)`. Absent →
  table-wide summary rows only.
- `target` accepts a single bareword column or a parenthesized list. Mirrors
  `SCALE ... SETTING target =>`.
- `aggregate` functions: `'min'`, `'max'`, `'avg'`, `'median'`, `'sd'`, `'sum'`.
  (`'mean'` is rejected — use `'avg'`.)
- `label` is a single string for one aggregate, a list aligned to
  `aggregate` for multiple. Fixture 30/34 use `label => ['Min', 'Max', 'Avg']`.

### 2.4 `SCALE`

```
SCALE <aesthetic> [FROM (<min>, <max>)] TO (<v1>, <v2>) | TO <palette> [VIA <transform>]
  SETTING target => <col> | (<col>, ...)
  [FILTER <condition>]
```

- Aesthetics: `background`, `foreground`, `size`, `opacity`.
- `FROM` optional → auto-domain (fixture 23, 24).
- `TO ( ... )` for explicit values; `TO <palette>` for **bareword** palette
  names (fixtures use `viridis`, `RdYlGn`). Reuses the ggsql VISUALISE
  palette catalogue.
- `VIA <transform>`: `log10`, `sqrt`, `reverse` (fixture 25 uses `log10`).
- `target` is required. Single column or list.
- `FILTER` is optional row-restriction using the same SQL-like expression
  language as `HIGHLIGHT ... FILTER`.

### 2.5 `HIGHLIGHT`

```
HIGHLIGHT <col> [, <col>...]
  FILTER <condition>
  SETTING <style_key> => <value>, ...
```

- Multiple columns share one filter and one style block (fixtures 27, 28).
- Multiple `HIGHLIGHT`s per query (fixture 28: up-day and down-day).
- `SETTING` style keys: `face`, `color`, `background`, `size`, `transform`,
  `decoration`.

### 2.6 `LABEL`

```
LABEL
  title    => '<str>',
  subtitle => '<str>',
  caption  => '<str>',
  <id>     => '<str>', ...
```

- One `LABEL` per query.
- Reserved keys (`title`, `subtitle`, `caption`) only when **unquoted**.
  Quote a column literally named `title` as `"title" => '...'`.
- `<id>` may be either a column name or a spanner ID introduced by
  `FORMAT SPAN ... AS <id>` (fixture 8: `population`, `density`,
  `comparison`).

---

## 3. Implementation surfaces

| Concern              | Location                                           |
| -------------------- | -------------------------------------------------- |
| Grammar              | `tree-sitter-ggsql/grammar.js`                     |
| Grammar corpus       | `tree-sitter-ggsql/test/corpus/tabulate/*.txt`     |
| AST                  | `src/tabulate/ast.rs` (new) — keep separate from `src/plot/` |
| Parser lowering      | `src/parser/builder.rs` (new `build_tabulate_*` fns) |
| Table IR             | `src/tabulate/ir.rs` (resolved table after SQL + AST merge) |
| Cell formatters      | `src/tabulate/format.rs` (`{:num}`, `{:time}`, case transforms) |
| Scales / palettes    | reuse `src/plot/scale.rs` palette catalogue        |
| HTML writer          | `src/tabulate/html.rs`                             |
| Vendored gt CSS      | `src/tabulate/gt_default.css` (**immutable**)      |
| Normalizer           | `src/tabulate/test_normalize.rs` (**immutable**)   |
| Fixture-diff harness | `tests/tabulate_fixtures.rs`                       |
| Captured oracle      | `tests/fixtures/<NN>/{query.ggsql,data.parquet,expected.html,meta.toml}` (**immutable**) |

> AGENTS.md hard rules: never modify `tests/fixtures/*/expected.html`, never
> modify `src/tabulate/gt_default.css`, never weaken
> `src/tabulate/test_normalize.rs`. Adding an `allowed_diff` entry requires
> a justification line in `AGENT_LOG.md`.

---

## 4. Phase plan (mirrors spec §5)

Phase order is mandatory. One feature branch per phase
(`agent/tabulate-phase-<N>`), one PR per phase, human review/merge.

### Phase 0 — bootstrap (done)

- Devcontainer, Makefile, fixture-diff harness, normalizer, empty AST stubs,
  vendored CSS, 34 captured fixtures.

### Phase 1 — `TABULATE *`, column selection, `hide` (fixtures 1, 2, 9)

- Grammar: `tabulate_statement`, optional column list, optional `FROM`.
- AST: `TabulateStmt { columns: Vec<TabCol>, source: Option<Source> }`.
- IR: resolve column order; default cell rendering (no formatters yet).
- `FORMAT <col> SETTING hide => true` for fixture 9.
- HTML writer: emit header, body rows, default cell text via `Display`.

### Phase 2 — stub, labels (fixtures 3, 4, 5)

- `FORMAT STUB <col>` → `rowname_col` in IR; render as stub column.
- `LABEL title/subtitle/caption` → `<thead>` header block.
- `LABEL <col> => '<str>'` → relabel column header.

### Phase 3 — spanners, including nesting (fixtures 6, 7, 8)

- `FORMAT SPAN <ids> AS <id>` produces a `Spanner { id, children: Vec<Node> }`
  where each child is either a column or another spanner.
- Multi-level header rendering: walk spanner tree top-down, emit `<tr>` per
  level.
- `LABEL <span_id> => '<str>'` resolves through the spanner namespace.

### Phase 4 — width and align (fixture 10)

- `FORMAT <col> SETTING width => '...', align => '...'`.
- Emit `style="width:..; text-align:..."` on `<th>`/`<td>` per gt's HTML.

### Phase 5 — number / time formatter mini-language + per-column locale (fixtures 11–17, 21)

- `src/tabulate/format.rs`: tokenize `{:num <printf>}`, `{:time <strftime>}`,
  `{:Title}` etc. inside a single-quoted RHS string. Preserve literal
  prefix/suffix.
- Numeric: parse a real printf body with required `%` introducer.
  Implement flags `'` (thousands), `+` (forced sign), `0` (zero pad);
  precision; conversions `d`, `f`, `e`. Reject specs lacking `%` with a
  clear error.
- Trailing literal `%` outside the `{...}` triggers ×100 scaling. A
  literal `%%` should not scale.
- Currency is just a literal `$` (or other) prefix outside `{...}` — no
  special case.
- Time: render `%d`/`%I` unpadded to match gt's named-style output (see
  §2.2 implementation note). Apply per-column locale.

### Phase 6 — null / zero / direct value mapping (fixtures 18, 19, 20)

- `RENAMING null => '<s>'` → `sub_missing`.
- `RENAMING 0 => '<s>'` → `sub_zero`.
- `RENAMING '<v>' => '<s>'` → `text_transform` for exact match.
- Honor LHS precedence: literal > null > 0 > `*`.

### Phase 7 — `SCALE` continuous (fixtures 22, 23, 24, 25)

- `SCALE background FROM (min, max) TO (c1, c2)` → per-cell interpolation
  applied as inline `background` color.
- Auto-domain when `FROM` omitted (fixture 23).
- Named palettes (bareword): reuse `src/plot/scale.rs` catalogue
  (`viridis`, `RdYlGn`, etc.) — fixtures 22, 24.
- `VIA log10` transform on domain mapping (fixture 25).
- `target` may be a list — apply scale independently per column.

### Phase 8 — `HIGHLIGHT` with `FILTER` (fixtures 26, 27, 28)

- Compile `FILTER` to a predicate over the resolved IR row.
- Reuse SQL expression machinery from the existing reader/executor where
  possible; if not, ship a minimal expression evaluator in `src/tabulate/`
  scoped to the operators the fixtures need (`<`, `>`, `=`, `AND`, `IN`).
- Apply style block (`face`, `color`, `background`, ...) to matching cells.
- Support multiple `HIGHLIGHT`s in one query; later clauses override earlier
  on conflict (gt's behavior — verify against fixture 28).

### Phase 9 — `FACET` row groups + summaries (fixtures 29, 30)

- `FACET <col>` partitions rows; emit a group-label row before each group.
- `FACET ... SETTING target, aggregate, label, side, groups, missing_text`
  computes summary rows with named aggregates.
- Multi-aggregate produces multiple summary rows in `label` order.

### Phase 10 — title case, forced sign (fixtures 31, 33)

- `{:Title}` (plus `{:UPPER}`, `{:lower}`) case transforms.
- `{:num %+.1f}` forced-sign already done in phase 5; fixture 33 is the
  acceptance test for it.

### Phase 11 — integration (fixture 34)

- One query exercising header + spanner + multi-aggregate facet + scale +
  two highlights + summary + per-column formatters.
- No new features; pure regression test that earlier phases compose.

---

## 5. Implementation notes (do not relitigate per phase)

These are decisions baked in once so per-phase work does not rediscover
them. The spec is authoritative for syntax; these notes capture
implementation choices the spec does not pin down.

1. **Printf body is a bare conversion spec.** Parse `{:num <spec>}` as
   printf **without** the `%` introducer; recognized flags `'` `+` `0`;
   conversions `d` `f` `e`. Reject specs that begin with `%` — the
   surrounding `{:num ...}` is the introducer. Match `gt::fmt_*()` defaults
   (e.g. `'${:num \'.2f}'` == `fmt_currency(currency = "USD")`). This
   diverges from the upstream spec, which still shows `%` in printf bodies.
2. **Percent suffix outside the `{...}` scales by 100.** `'{:num .1f}%'`
   on `0.085` renders `8.5%`. Don't double-scale if the user writes
   `'{:num .1f}%%'` — a literal `%%` should not trigger scaling.
3. **strftime in `{:time ...}` is gt-compatible, not strict strftime.** In
   particular `%d` and `%I` render unpadded to match gt's named styles
   (`date_style`, `time_style`). See §2.2 “Implementation note
   (day/hour padding)”.
4. **`AS` inside `TABULATE` is a column rename, not a display label.**
   Display text belongs to `LABEL` (spec open-question 3).
5. **`AS <id>` after `FORMAT SPAN` is always a bareword.** Quoted strings
   are a parse error. The bareword is the default display label and the
   reference used by `LABEL` and nested `FORMAT SPAN`.
6. **No global locale.** `locale` only via `FORMAT ... SETTING locale =>
   '...'` per column (spec open-question 6).
7. **Named palettes from day one.** `TO viridis`, `TO RdYlGn` — reuse the
   VISUALISE palette catalogue (spec open-question 7). Spec examples 22
   and 24 require this in phase 7.
8. **Captured fixtures use this repo's canonical syntax.** Every
   `tests/fixtures/*/query.ggsql` uses the bare `{:num <body>}` form
   (no `%`) and gt-compatible strftime (`%d`, `%I`). No re-capture is
   needed when starting phases that touch the formatter mini-language.

---

## 6. Acceptance

A phase is done when:

- Every fixture listed for that phase passes `cargo test --test
  tabulate_fixtures -- <fixture>` under strict normalization.
- `make check` is green (fmt, clippy `-D warnings`, tree-sitter tests, all
  cargo tests, fixture-diff suite).
- No `tests/fixtures/*/expected.html`, `src/tabulate/gt_default.css`, or
  `src/tabulate/test_normalize.rs` change.
- Any new `allowed_diff` has a one-line justification in `AGENT_LOG.md`.
- Branch `agent/tabulate-phase-<N>` opens a PR; a human merges it before
  phase `<N+1>` starts.
