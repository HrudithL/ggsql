# TABULATE — implementation plan for ggsql

This document is the working implementation plan for the `TABULATE` clause in
this repo. It is derived from the upstream spec but reflects the **syntax
actually used by the captured fixtures**, which are the oracle the agent
loop tests against. Where the prose in `/spec/GTSQL_PLAN.md` disagrees with
`tests/fixtures/*/query.ggsql`, the fixtures win.

Read in this order:

1. [AGENTS.md](AGENTS.md) — agent rules, branches, stop conditions.
2. [/spec/GTSQL_AGENTBUILD_SPEC.md](../gtsql/GTSQL_AGENTBUILD_SPEC.md) — phase
   ordering, pass gate, sandbox.
3. [/spec/GTSQL_PLAN.md](../gtsql/GTSQL_PLAN.md) — language reference (prose).
4. This file — the implementation contract, plus syntax deltas vs the prose.

---

## 1. Top-level grammar

A query is `SELECT ... TABULATE ... <clauses>*`. The SQL preamble is
optional when `FROM` is given inside `TABULATE`. Six clauses can follow
`TABULATE`, each in any order:

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
| `units`   | string  | column units, rendered in header (`{{km^2}}`, etc.) | 32 |

> `units` is **not** in the GTSQL_PLAN.md prose but is used by fixture 32 —
> implement it.

#### `FORMAT ... RENAMING` LHS

| LHS pattern  | Meaning                                | gt mapping        |
| ------------ | -------------------------------------- | ----------------- |
| `*`          | all values (formatter)                 | `fmt_*()`         |
| `null`       | NA / missing                           | `sub_missing()`   |
| `0`          | zero                                   | `sub_zero()`      |
| `'literal'`  | exact value match                      | `text_transform()`|

Precedence (highest first): literal > `null` > `0` > `*`. So
`RENAMING * => '{:num ,d}', 30 => 'Thirty'` formats everything but `30` as
an integer, and `30` becomes the literal string.

#### `FORMAT ... RENAMING` RHS — string interpolation

The RHS is a single-quoted string. Any text outside `{...}` is preserved as a
literal prefix/suffix. The two formatter mini-languages are **`{:num ...}`**
and **`{:time ...}`**, plus case transforms.

##### Numeric formatter `{:num <spec>}` — **delta from prose**

The body after `{:num ` is a printf-like spec **with no leading `%`**. The
fixtures use the form `{:num [flags][width][.precision]<conv>}`:

| Spec              | Means                                  | Fixture |
| ----------------- | -------------------------------------- | ------- |
| `{:num .3f}`      | 3 decimal places                       | 11      |
| `{:num ,d}`       | integer with thousands separators      | 12, 13  |
| `{:num ,.2f}`     | float, 2 decimals, thousands seps      | 22      |
| `{:num ,.1f}`     | float, 1 decimal, thousands seps       | 14, 30  |
| `{:num .1f}`      | float, 1 decimal, no seps              | 14      |
| `{:num .2e}`      | scientific notation, 2 decimals        | 15      |
| `{:num +.1f}`     | float, 1 decimal, forced sign          | 33      |

Flags supported: `,` (thousands separator), `+` (forced sign), `0` (zero
pad). Conversions supported: `d` (integer), `f` (fixed-point float), `e`
(scientific).

> The GTSQL_PLAN.md prose shows specs with a leading `%`
> (`{:num %\'.2f}`); the fixtures **do not** use `%` and use `,` for
> thousands separation rather than `'`. Implement the fixture form. The `%`
> form should be a parse error (or a "did you mean..." hint) so we don't
> silently accept the wrong syntax.

##### Percent semantics

A literal `%` suffix outside the `{...}` (e.g. `'{:num .1f}%'`) triggers
gt's `fmt_percent()` behavior: the value is **multiplied by 100** before
formatting. Fixture 14 takes a proportion `0.085` and renders `8.5%`.
Implement the same scaling when a trailing `%` is present.

##### Time formatter `{:time <strftime>}`

The body is a literal `strftime(3)` format string and the `%`s in it are
real percent-introducers (because that is what strftime uses). gt's
locale-aware rendering applies; per-column locale comes from `FORMAT ...
SETTING locale => '...'` (no global locale — see open question 6 in the
spec).

| Spec                              | Fixture |
| --------------------------------- | ------- |
| `{:time %B %-d, %Y}`              | 16      |
| `{:time %-d %B %Y}`               | 17      |
| `{:time %-I:%M %p}`               | 17      |
| `{:time %A, %B %-d, %Y at %-I:%M %p}` | 17  |
| `{:time %A %-d %B %Y}` (`locale='fr'`) | 21 |

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
- `aggregate` functions: `'min'`, `'max'`, `'mean'`, `'median'`, `'sd'`, `'sum'`.
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

- `src/tabulate/format.rs`: tokenize `{:num <spec>}`, `{:time <spec>}`,
  `{:Title}` etc. inside a single-quoted RHS string. Preserve literal
  prefix/suffix.
- Numeric: implement `,`, `+`, `0` flags; precision; conversions `d`, `f`, `e`.
- Trailing literal `%` outside the `{...}` triggers ×100 scaling.
- Currency is just `$` (or other) prefix outside `{...}` — no special case.
- Time: pass the strftime string through with locale resolved per
  `FORMAT ... SETTING locale => '...'`.
- Reject the `%`-introducer prose form (`{:num %.2f}`) with a clear error.

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

### Phase 10 — title case, units, forced sign (fixtures 31, 32, 33)

- `{:Title}` (plus `{:UPPER}`, `{:lower}`) case transforms.
- `FORMAT ... SETTING units => '<s>'` renders units in the column header
  (gt's `cols_units` semantics — inspect fixture 32 expected HTML for the
  exact markup, likely `<span class="gt_units">`).
- `{:num +.1f}` forced-sign already done in phase 5; fixture 33 is the
  acceptance test for it.

### Phase 11 — integration (fixture 34)

- One query exercising header + spanner + multi-aggregate facet + scale +
  two highlights + summary + per-column formatters.
- No new features; pure regression test that earlier phases compose.

---

## 5. Deltas vs `/spec/GTSQL_PLAN.md`

Codify these once; do not relitigate during phase work.

1. **Numeric formatter uses no `%` introducer.** Prose: `{:num %\'.2f}`;
   fixtures: `{:num ,.2f}`. Implement the fixture form. Reject the prose
   form.
2. **Thousands separator is `,` not `'`.** Same root cause as (1).
3. **Percent suffix scales by 100.** `'{:num .1f}%'` on a value of `0.085`
   renders `8.5%` (fixture 14).
4. **`SETTING units` exists on `FORMAT`.** Not in the prose but used by
   fixture 32. Add to the supported keys.
5. **`AS` inside `TABULATE` is a column rename, not a label.** Per spec
   open-question 3: `AS` only renames the column. Display text belongs to
   `LABEL`. (The prose has example lines like
   `TABULATE date, region, revenue AS 'Revenue ($)'` and
   `FORMAT date AS 'Order Date'` that contradict this — ignore them.)
6. **`AS <id>` after `FORMAT SPAN` is always a bareword.** Quoted strings
   are a parse error. The bareword is the default display label and the
   reference used by `LABEL` and nested `FORMAT SPAN`.
7. **No global locale.** `locale` only via `FORMAT ... SETTING locale =>
   '...'` per column (spec open-question 6).
8. **Named palettes from day one.** `TO viridis`, `TO RdYlGn` — reuse the
   VISUALISE palette catalogue (spec open-question 7). Fixtures 22 and 24
   require this in phase 7.

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
