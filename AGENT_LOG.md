# Agent Log

Append-only record of decisions the agent made while implementing TABULATE.

Each entry: date, phase, fixture (if applicable), one sentence describing
the deviation or `allowed_diff` justification.

---

## 2026-06-04 — bootstrap

- Cloned `posit-dev/ggsql` to local sibling of the `gtsql` spec repo.
- Created branch `agent/tabulate-bootstrap`.
- Added devcontainer, Makefile, fixture-diff harness skeleton, normalization
  module, and capture/extract R scripts.
- `src/tabulate/gt_default.css` is a placeholder; human must run
  `make css-extract` once to vendor real gt CSS.
- `tests/fixtures/` is empty; human must run `make fixtures-capture` once on
  the host (R + gt + arrow) to populate it.

## 2026-06-09 — phase 1 complete

- Fixtures 01 (minimal table), 02 (column selection / reordering), and 09
  (FORMAT hide) pass under strict normalization.
- Branch `agent/tabulate-phase-1` merged into `main` (no-ff merge commit
  `bfeba4a`); local feature branches `agent/tabulate-bootstrap` and
  `agent/tabulate-phase-1` deleted. No push to origin.
- New branch `agent/tabulate-phase-2` cut from `main` to begin phase 2
  (FORMAT STUB + LABEL title/subtitle/caption + LABEL <col>, fixtures 03,
  04, 05).
