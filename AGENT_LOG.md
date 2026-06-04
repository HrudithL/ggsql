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
