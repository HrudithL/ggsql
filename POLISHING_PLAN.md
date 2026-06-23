# TABULATE polishing plan

Branch: `polishing` cut from `main`. One change per commit. After every
commit: targeted `cargo test`, then `bash examples/tabulate/run.sh`, then
verify any negative/error examples actually error. `make check` only at
end of each phase to keep iteration fast.

Spec files to keep in sync at every doc edit:
- `/spec/GTSQL_PLAN.md` (authoritative language reference)
- `/workspaces/ggsql/TABULATE_PLAN.md` (implementation plan)
- `/workspaces/ggsql/examples/tabulate/README.md` (in-repo examples index)

**C7 ruling: `/spec/GTSQL_EXAMPLES.qmd` is no longer authoritative.** The
in-repo `examples/tabulate/*.ggsql` set is now the canonical example
corpus. Do not edit `GTSQL_EXAMPLES.qmd` as part of this pass; it can
diverge. Captured fixtures under `tests/fixtures/` are still the
regression oracle (their `expected.html` is immutable), but no further
attempt to keep `GTSQL_EXAMPLES.qmd` in sync with the language is
required.

Code surfaces:
- `tree-sitter-ggsql/grammar.js` + `test/corpus/tabulate/*`
- `src/tabulate/ast.rs`, `src/parser/tabulate.rs`
- `src/tabulate/format.rs` (printf body, raw `{}`, percent literal)
- `src/tabulate/ir.rs`, `src/tabulate/html.rs`
- `src/tabulate/facet.rs` (or wherever aggregate lookup lives)
- `tests/tabulate_fixtures.rs`
- `tests/fixtures/<NN>/query.ggsql` (queries are mutable; expected.html
  and gt_default.css are NOT)

Fixture rule reminder: `expected.html`, `gt_default.css`,
`test_normalize.rs` are immutable. `query.ggsql` is editable — we rewrite
it whenever syntax changes (Phase 2 sweep).

---

## Phase 0 — Branch + baseline

1. `git checkout main && git pull && git checkout -b polishing`.
2. Run `make check` once, record baseline status in a `## Baseline`
   section appended to the bottom of this file.

## Phase 1 — Pure documentation alignment (no code)

Subagent **Docs-1** handles all of Phase 1. Each step is one commit.
After each step: verify markdown renders, grep for residual stale
phrasing. 15 commits total.

1.1 **A3** — `AS` is SQL rename only.
   - `/spec/GTSQL_PLAN.md` §1 (`TABULATE`): change the
     "`AS` renames a column for display (maps to `cols_label()`)" bullet
     to "`AS` is a SQL alias — renames the underlying column.
     Display labels are set via `LABEL <col> => '<text>'`."
   - Update §2 prose example 4 (`FORMAT order_date AS 'Order Date'`) —
     that uses a quoted string which is now forbidden. Replace with
     `LABEL order_date => 'Order Date'`.
   - `TABULATE_PLAN.md` already correct in §2.1; re-read to confirm.

1.2 **A4** — `\'` is the only single-quote escape.
   - Add a one-sentence note in `/spec/GTSQL_PLAN.md` §2.2 reaffirming
     `\'` is the only form (already present; confirm wording).
   - C7: do NOT edit `/spec/GTSQL_EXAMPLES.qmd` example 5.

1.3 **A5** — FACET no-group summary is the documented way.
   - `TABULATE_PLAN.md` §2.3: already covers this; verify wording.
   - C7: do NOT edit `/spec/GTSQL_EXAMPLES.qmd` omissions note.

1.4 **A6** — Remove `units` from spec.
   - `/spec/GTSQL_PLAN.md` §2.1 SETTING list: delete the `units` bullet.
     Add a one-line note: "To annotate a column header with a unit, put
     it in the `LABEL <col> => '...'` text (e.g.
     `LABEL land_area => 'Land Area (km²)'`)."
   - `TABULATE_PLAN.md` §2.2 SETTING table: delete `units` row and the
     `> units is not in the GTSQL_PLAN.md prose ...` callout.
   - `TABULATE_PLAN.md` §4 Phase 10: remove the units bullet.
   - `TABULATE_PLAN.md` §5 note 4: delete.

1.5 **B1** — `avg` replaces `mean`.
   - `/spec/GTSQL_PLAN.md` §3.1 aggregate list: replace `'mean'` with
     `'avg'`. Add: "`'mean'` is rejected (use `'avg'`)."
   - `TABULATE_PLAN.md` §2.3: same swap.
   - In-repo example swaps (37, 42) happen in Phase 4. C7:
     `GTSQL_EXAMPLES.qmd` examples 30/33 stay as-is, no edit.

1.6 **B6** — VISUALISE and TABULATE are mutually exclusive.
   - `/spec/GTSQL_PLAN.md`: add a new "## Interaction with VISUALISE"
     section: "A single query may contain either `VISUALISE` clauses
     OR `TABULATE` clauses, never both. Mixing them is a parse-time
     error."
   - Revise open-question #2 answer to match.
   - `TABULATE_PLAN.md` §1: add a bullet to the same effect.

1.7 **B7** — Reserved-bareword list.
   - `/spec/GTSQL_PLAN.md`: add a new "## Reserved barewords" section
     listing every word that requires quoting when used as a column
     name: `TABULATE`, `FORMAT`, `FACET`, `SCALE`, `HIGHLIGHT`,
     `LABEL`, `SETTING`, `RENAMING`, `FILTER`, `FROM`, `STUB`, `SPAN`,
     `AS`, `TO`, `VIA`. Plus the reserved LABEL keys `title`,
     `subtitle`, `caption` (only inside `LABEL`).
   - Explicit non-list: inner SETTING keys (`target`, `aggregate`,
     `groups`, `side`, `label`, `missing_text`, `width`, `align`,
     `hide`, `locale`, `face`, `color`, `background`, `size`,
     `transform`, `decoration`, `foreground`, `opacity`) are **not**
     reserved — they may be used as column names without quoting
     because the LABEL clause does not accept them.
   - To use a reserved bareword as a column name, quote it
     (`"label" => 'My Label'` for a column literally named `label`).
   - `TABULATE_PLAN.md`: mirror this list in §2 with a pointer to the
     spec section.

1.8 **B8** — Spanner-ID collisions are errors.
   - `/spec/GTSQL_PLAN.md` §2 (FORMAT SPAN): add: "A spanner ID must
     not collide with an existing column name or another spanner ID
     in the same query. Collisions are a parse-time error."
   - `TABULATE_PLAN.md` §2.2: mirror.

1.9 **B9** — Later-wins for SCALE/HIGHLIGHT conflicts.
   - `/spec/GTSQL_PLAN.md` §4 (SCALE) and §5 (HIGHLIGHT): add a shared
     paragraph: "When two `SCALE` clauses or a `SCALE` and a
     `HIGHLIGHT` (or two `HIGHLIGHT`s) target the same cell, the
     clause appearing later in the query wins."
   - `TABULATE_PLAN.md` §2.4/§2.5: mirror.

1.10 **B10** — Auto-align rule documentation.
   - `/spec/GTSQL_PLAN.md` §2.1 (`align` bullet): add: "`'auto'` (the
     default) follows gt's `cols_align(align='auto')` rule —
     `numeric`/`integer`/`date`/`time`/`datetime`/`logical` columns
     right-align (logical center per gt), `character`/`factor`
     left-align." Phrase to exactly match gt's documented behavior;
     read the gt source comment in the existing fixture render to
     confirm before committing.
   - `TABULATE_PLAN.md` §2.2: mirror.

1.11 **B11** — `groups` semantics and validation.
   - `/spec/GTSQL_PLAN.md` §3.1 (groups bullet): rewrite to: "Optional.
     Restricts summary rows to the listed group values from the
     `<group_col>`. Default (omitted): all groups receive summaries.
     Referencing a non-existent group is a parse-/execution-time
     error."
   - `TABULATE_PLAN.md` §2.3: mirror.

1.12 **C1** — `target` only accepts the parenthesized form.
   - `/spec/GTSQL_PLAN.md` §3.1 (FACET `target`) and §4 (SCALE
     `SETTING target`): rewrite both to "`target => (<col>)` (single
     column) or `target => (<col1>, <col2>, ...)` (multiple). The
     bareword form `target => <col>` is no longer accepted."
   - `TABULATE_PLAN.md` §2.3 and §2.4: mirror.
   - All spec example snippets that currently show `target => col`
     (e.g. `target => satisfaction`, `target => revenue`) get
     rewritten to `target => (satisfaction)`, `target => (revenue)`.
   - The actual code+example sweep for this happens in Phase 6.6
     (parser rejection + in-repo example rewrite).

1.13 **C3** — `{:...}` mini-language keywords are case-insensitive,
   lowercase by convention.
   - `/spec/GTSQL_PLAN.md` §2.2: add a one-paragraph note: "All
     formatter keywords inside `{:...}` — `num`, `time`, `title`,
     `upper`, `lower` — are case-insensitive (matching SQL keyword
     handling). The lowercase form is canonical and used throughout
     this document; `{:TITLE}`, `{:Title}`, `{:title}` are
     equivalent."
   - `TABULATE_PLAN.md` §2.2 case-transforms table: rewrite rows in
     lowercase canonical form (`{:title}`, `{:upper}`, `{:lower}`)
     and add the case-insensitivity note.

1.14 **C5** — Multiple HIGHLIGHTs vs one HIGHLIGHT with OR.
   - `/spec/GTSQL_PLAN.md` §5 (HIGHLIGHT): add a doc note: "Two
     HIGHLIGHTs with disjoint filters and the same style produce the
     same output as one HIGHLIGHT with `FILTER (a) OR (b)`. Use
     separate HIGHLIGHTs when the styles differ per predicate; use
     one with `OR` when the style is shared."
   - `TABULATE_PLAN.md` §2.5: mirror.

1.15 **C6** — SCALE FILTER vs HIGHLIGHT and the later-wins tiebreaker.
   - `/spec/GTSQL_PLAN.md` §4 (SCALE) and §5 (HIGHLIGHT): add a
     shared doc note: "Use `SCALE` for continuous (data-driven)
     styling that interpolates a domain over a palette; use
     `HIGHLIGHT` for categorical / predicate-driven styling that
     applies a fixed style. When a `SCALE` (optionally with
     `FILTER`) and a `HIGHLIGHT` write the same CSS property on the
     same cell, the clause appearing later in the query wins. This is
     the same tiebreaker as B9 (later wins) and applies uniformly to
     SCALE-vs-SCALE, HIGHLIGHT-vs-HIGHLIGHT, and SCALE-vs-HIGHLIGHT
     conflicts."
   - `TABULATE_PLAN.md` §2.4/§2.5: mirror.

Phase 1 verification: each commit ends with a grep that the old
phrasing is gone. No code test runs needed in Phase 1.

---

## Phase 2 — Format mini-language refactor (A1, A2, B5)

This is the largest single change. Subagent **Format-2** owns it. It
must land before Phase 3+ because most other tasks reference the new
printf body.

2.1 Replace the `{:num <body>}` parser in `src/tabulate/format.rs` so
    the body is a full Rust-`sprintf`-compatible printf conversion:
    `%[flags][width][.precision]type`.
    - Flags: `-` (left-justify), `+` (force sign), ` ` (space), `0`
      (zero pad), `#` (alt form), `\'` (locale-aware thousands —
      ggsql extension; document as such).
    - Width: positive integer.
    - Precision: `.N`.
    - Types: `d`, `i`, `u`, `f`, `F`, `e`, `E`, `g`, `G`, `o`, `x`,
      `X`.
    - The body MUST start with `%` — reject the bare form with a clear
      error pointing at the spec.
    - Reuse an existing Rust sprintf crate if license-compatible
      (`sprintf` crate); otherwise implement against the formal
      grammar in the user message.

2.2 Remove percent-suffix `×100` scaling.
    - Delete any code path that detects `%` outside `{...}` and scales
      the input. Trailing `%` is a literal character — same as any
      other character outside `{...}`.
    - Update `TABULATE_PLAN.md` §5 note 2 to: "**Percent is just a
      literal character.** No scaling. A column of 0–1 proportions
      that should render as `xx.x%` must be multiplied by 100 in the
      upstream SQL."

2.3 Implement bare `{}` raw passthrough.
    - `'${}abc'` formats every cell as its default-rendered value with
      a `$` prefix and `abc` suffix; no type coercion, no formatting
      change. (Already specified in `GTSQL_PLAN.md` RHS list.)
    - Add a parser case that emits a `Raw` formatter token.

2.3a **C3** — Verify formatter keywords are case-insensitive.
    - `src/tabulate/format.rs` parser: ensure the keyword match for
      `num`, `time`, `title`, `upper`, `lower` is ASCII-case-insensitive
      (use `eq_ignore_ascii_case` or normalize to lower before
      matching). Add a unit test asserting `{:NUM %d}` and
      `{:Time %Y}` parse identically to their lowercase forms.

2.4 Update `/spec/GTSQL_PLAN.md` §2.2:
    - Replace the "without the leading `%` introducer" prose with the
      new grammar (verbatim from the user message).
    - Rewrite the mapping table at the end of the file: restore `%` in
      all printf bodies (`{:num %.1f}`, `{:num %\'d}`,
      `{:num %\'.2f}`, `{:num %.2e}`, `{:num %+.1f}`).
    - Add `{}` raw to the RHS list with the `${}abc` example.

2.5 Update `TABULATE_PLAN.md`:
    - §2.2 numeric formatter table: restore `%` in every row.
    - Remove §5 note 1.
    - Update note 9: fixtures now use the `%` form after the sweep.
    - Remove the "Printf body has no `%` introducer in this repo"
      warning at the top of the file.

2.6 Sweep `tests/fixtures/*/query.ggsql`:
    - Mechanical: every `{:num <body>}` becomes `{:num %<body>}`.
    - Every `{:num <body>}%` percent-suffix that previously implied
      ×100 must have its upstream SQL adjusted to multiply by 100 (or
      the existing data must already be a percentage — verify each
      fixture).
    - Run `cargo test -p ggsql --test tabulate_fixtures` with
      `--no-default-features --features
      duckdb,parquet,vegalite,builtin-data` after each fixture edit.

2.7 Sweep `examples/tabulate/*.ggsql`:
    - Same mechanical `{:num %...}` rewrite.
    - 20_percent and 41_forced_sign_growth already pre-multiply by
      100 — keep that.
    - Add example 43 `43_raw_passthrough.ggsql` for B5 (`'${}abc'`).
    - Run `bash examples/tabulate/run.sh`; visually sanity-check
      `examples/tabulate/out/index.html`.

2.8 Update `examples/tabulate/README.md` table for any added/changed
    examples.

2.9 Commit cadence inside Phase 2 (each is one commit):
    a. parser change (printf grammar).
    b. percent-scaling removal.
    c. raw `{}` support.
    d. case-insensitive formatter keyword check (2.3a).
    e. spec doc update (§2.2 + mapping table).
    f. TABULATE_PLAN.md doc update.
    g. fixtures sweep (one commit, mechanical).
    h. examples sweep (one commit).
    i. add example 43 raw passthrough.
    j. README.md update.

---

## Phase 3 — Remove `units` (A6)

Subagent **Units-3**.

3.1 Strip the `units` key from the parser/AST/IR/HTML writer.
    Reject `SETTING units => '...'` at parse time with: "units was
    removed; put units in the LABEL text instead".
3.2 Delete `examples/tabulate/40_units_in_header.ggsql` (or rewrite it
    as `40_unit_in_label.ggsql` showing the LABEL workaround).
3.3 Update `examples/tabulate/README.md`.
3.4 Run all examples + fixtures.

Commit cadence: (a) code+grammar removal, (b) example rewrite, (c)
README.

---

## Phase 4 — `avg` replaces `mean` (B1)

Subagent **Aggregate-4**.

4.1 In `src/tabulate/facet.rs` (or wherever aggregate name → SQL
    function maps): replace `mean` with `avg`. Reject `mean` at parse
    time with: "use 'avg' (SQL canonical), 'mean' was removed".
4.2 Sweep:
    - `examples/tabulate/37_facet_multi_aggregate.ggsql`: `mean` →
      `avg`. Remove the "alias avg" wording in the comment header —
      `avg` is the only spelling now.
    - `examples/tabulate/42_comprehensive_report.ggsql`: `mean` →
      `avg`.
    - Any `tests/fixtures/*/query.ggsql` that uses `mean`: same swap.
4.3 Run fixtures + examples.

Commit cadence: (a) code change, (b) fixtures+examples sweep, (c)
README/comment cleanup.

---

## Phase 5 — Missing-feature impls + examples

### 5a — `FORMAT *` wildcard (B4)

Subagent **Wildcard-5a**.

- Check `src/parser/tabulate.rs` / `src/tabulate/ir.rs` for `*` support
  in `FORMAT`. If missing, implement: `FORMAT * RENAMING null => '—'`
  applies to all columns whose type is compatible with the RHS
  template (gt's behavior — text transforms apply to character cols,
  `{:num}` to numeric cols, `{:time}` to temporal cols, `null =>` to
  all).
- Add `examples/tabulate/44_format_wildcard.ggsql`:
  short query mixing several column types, single
  `FORMAT * RENAMING null => '—'` replacing all nulls table-wide.
- README + run.sh.

### 5b — `SCALE` aesthetics (B2)

Subagent **Aesthetics-5b**.

- Audit `src/tabulate/scale.rs` (or wherever SCALE lowers): which of
  `background`, `foreground`, `size`, `opacity` are implemented?
- Implement the missing ones via `tab_style(cell_text(...))` /
  `cell_fill(alpha = ...)` per the gt mappings already in the spec.
- Add examples (one each):
  - `45_scale_foreground.ggsql` — text-color ramp on a numeric col.
  - `46_scale_size.ggsql` — font-size ramp.
  - `47_scale_opacity.ggsql` — opacity ramp.
- README + run.sh.

### 5c — `HIGHLIGHT` style keys (B3)

Subagent **HighlightKeys-5c**.

- Audit `HIGHLIGHT SETTING` handling. Implement missing keys: `size`,
  `transform`, `decoration`.
- Add examples:
  - `48_highlight_size.ggsql` — bigger font on matching cells.
  - `49_highlight_transform.ggsql` — uppercase on matching cells.
  - `50_highlight_decoration.ggsql` — line-through on matching cells.
- README + run.sh.

### 5d — Example for `groups` (B11)

Subagent **Groups-5d**.

- Add `51_facet_groups_restrict.ggsql`: FACET with `groups => ['North',
  'South']` to show summaries only on those two groups.
- Add `52_facet_groups_error.ggsql` (negative test, marked in README
  as "should error"): references a non-existent group and must
  produce a clean diagnostic.
- README + run.sh.

5a–5d are independent and can be fanned out as four parallel
subagents. They each produce 1–3 commits. Merge order
5a → 5b → 5c → 5d to keep README diffs tidy.

---

## Phase 6 — Validation rules (B6, B7, B8, B9, B11, C1)

Subagent **Validate-6**. Sequential commits inside.

6.1 **B6 mutual exclusion**:
    - Parser-level guard: if a query contains both `visualise_statement`
      and `tabulate_statement`, error: "VISUALISE and TABULATE are
      mutually exclusive in a single query".
    - Add a negative example or a cargo test asserting the error.

6.2 **B7 reserved-bareword enforcement**:
    - Verify the grammar already rejects unquoted reserved words as
      column names (it does for `LABEL` per repo memory). Add the
      remaining ones if missing: `TABULATE`, `FORMAT`, `FACET`,
      `SCALE`, `HIGHLIGHT`, `SETTING`, `RENAMING`, `FILTER`, `FROM`,
      `STUB`, `SPAN`, `AS`, `TO`, `VIA`.
    - Cargo test: a query attempting to reference a column literally
      named `format` without quotes must produce a parse error
      pointing at the spec section.
    - Cargo test: same query with `"format"` (quoted) parses cleanly.
    - Cargo test: inner SETTING key names (`target`, `aggregate`,
      `label`, `width`, ...) as unquoted column names parse cleanly.

6.3 **B8 spanner-id collisions**:
    - Add a semantic check after parsing that the set of spanner IDs
      is disjoint from the set of column names and that no two
      spanners share an ID. Error: "spanner ID '<id>' collides with
      column '<id>'" / "duplicate spanner ID '<id>'".
    - Cargo test: each error path.

6.4 **B9 later-wins**:
    - Audit the writer's style merge: confirm later SCALE/HIGHLIGHT
      overrides earlier on the same cell. If undefined order, fix to
      "latest clause wins per CSS property".
    - Cargo test: two HIGHLIGHTs with overlapping predicates produce
      the later style.

6.5 **B11 groups validation**:
    - Validate `groups => [...]` values against the actual distinct
      group_col values at execute time. Unknown group → error with the
      unknown value name.
    - Already covered by 5d example 52 (negative test).

6.6 **C1 enforce parenthesized `target`**:
    - Parser: reject bare `target => <col>`; require `target => (<col>)`
      even for one column. Clear error: "target requires a parenthesized
      list, e.g. `target => (col)`".
    - Mechanical sweep of in-repo examples that currently use the
      bareword form: 28_scale_named_palette, 29_scale_explicit_colors,
      31_scale_log_transform, 42_comprehensive_report (FACET target +
      SCALE target), plus any `target => <bareword>` in
      `tests/fixtures/*/query.ggsql`. Wrap every single-column target
      in parens.
    - Cargo test: bare form errors; parenthesized single-col parses
      and renders identically to the prior bareword form.

Commit cadence: one commit per sub-step (6.1, 6.2, 6.3, 6.4, 6.5, 6.6).

---

## Phase 7 — Final cleanup

Subagent **Cleanup-7**.

7.1 `cargo fmt -p ggsql`.
7.2 `cargo clippy --no-default-features --features
    duckdb,parquet,vegalite,builtin-data -- -D warnings`. The two
    pre-existing `src/writer/vegalite/layer.rs` warnings are unrelated
    and may be left.
7.3 Final `make check` with full default features.
7.4 Regenerate `examples/tabulate/out/index.html` one last time.
7.5 Append a single dated entry to `AGENT_LOG.md` summarizing the
    polishing pass.
7.6 Final commit: README/CHANGELOG bumps if any.

---

## Subagent fan-out summary

Sequential phases (each must complete before the next starts):

Phase 0 → Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5 → Phase 6 → Phase 7
setup docs printf units avg feats valid clean


Within each phase, sub-tasks fan out where independent:
- Phase 1: single Docs-1 subagent (15 commits, ordered, no parallel
  work because they share files).
- Phase 2: single Format-2 subagent (10 commits, sequential).
- Phase 3: single Units-3 subagent (3 commits).
- Phase 4: single Aggregate-4 subagent (3 commits).
- Phase 5: **four parallel subagents** (5a, 5b, 5c, 5d). They touch
  disjoint files; each opens its own micro-rebase. Merge order
  5a → 5b → 5c → 5d to keep README diffs tidy.
- Phase 6: single Validate-6 subagent (6 commits).
- Phase 7: single Cleanup-7 subagent.

Each subagent's job description includes:
- The list of commits it owns.
- The exact `cargo test` filter to run after each commit.
- The exact `bash examples/tabulate/run.sh` invocation.
- A bullet of "stop and ask the human" conditions (e.g. spec wording
  ambiguous, fixture HTML regression that can't be solved without
  changing immutable files).

## Risks / stop conditions

- If Phase 2's printf-body change causes any fixture to fail with a
  non-mechanical diff (i.e. the captured HTML embedded `%` differently
  from what the new parser produces), stop and hand back — re-capture
  may be needed and that requires R.
- If implementing a missing SCALE aesthetic or HIGHLIGHT key turns
  out to require oracle HTML we don't have, mark the example as
  best-effort and ask before adding `allowed_diff` entries.
- If `FORMAT *` type-compat logic is non-trivial, scope it to a
  minimum: only `null => '...'` and `0 => '...'` apply across all
  types in the first pass; `{:num}` / `{:time}` on `FORMAT *` may be
  rejected as "not yet supported" in a follow-up.
- C1 (parenthesized `target` only) may invalidate a captured fixture
  that uses the bareword form. If a fixture's `query.ggsql` uses
  bareword target and the rewrite cannot be done without semantic
  change, stop and ask.

## Out of scope (deliberately not changed)

- Locale support (B12): leave the current `'en'` / `'fr'` impl as-is.
- LaTeX/RTF/Word writers.
- Nanoplots / footnotes / multi-stub.
- New ggsql syntax beyond what is in the spec.
- `/spec/GTSQL_EXAMPLES.qmd` (per C7): no edits anywhere in this pass.
---

## Baseline

- Branch `polishing` cut from `main` at `5c6bc7a` (Merge phase 12).
- `cargo test -p ggsql --no-default-features --features duckdb,parquet,vegalite,builtin-data --test tabulate_fixtures`: **35 passed, 0 failed, 3 ignored**.
- Untracked working tree clean apart from the plan itself.
