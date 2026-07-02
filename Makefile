# Top-level convenience targets for the TABULATE implementation work.
# The agent's pass gate is `make check`.

.PHONY: check fmt-check clippy ts-test cargo-test fixtures fixtures-capture css-extract sync-fixtures

check: fmt-check clippy ts-test cargo-test fixtures

fmt-check:
	cargo fmt --check

clippy:
	cargo clippy --all-targets --workspace -- -D warnings

ts-test:
	cd tree-sitter-ggsql && tree-sitter generate && tree-sitter test

cargo-test:
	cargo test --workspace --no-fail-fast

# Fixture-diff harness (added in phase 0). Lives at tests/tabulate_fixtures.rs.
fixtures:
	cargo test --test tabulate_fixtures -- --include-ignored

# One-time fixture capture from gt (R). Human-run, not in the agent loop.
# Writes tests/fixtures/<NN_name>/{query.ggsql, data.parquet, expected.html, meta.toml}.
fixtures-capture:
	Rscript scripts/capture_fixtures.R

# One-time vendoring of gt's default CSS into src/tabulate/gt_default.css.
css-extract:
	Rscript scripts/extract_gt_css.R

# Re-sync fixtures from $GGSQL_SPEC_DIR (defaults to /spec).
sync-fixtures:
	@if [ -d "$${GGSQL_SPEC_DIR:-/spec}/fixtures" ]; then \
		mkdir -p tests/fixtures && \
		rsync -a --delete "$${GGSQL_SPEC_DIR:-/spec}/fixtures/" tests/fixtures/; \
		echo "fixtures synced from $${GGSQL_SPEC_DIR:-/spec}/fixtures"; \
	else \
		echo "no fixtures at $${GGSQL_SPEC_DIR:-/spec}/fixtures (run 'make fixtures-capture' on the host)"; \
	fi
