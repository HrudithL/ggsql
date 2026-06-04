#!/usr/bin/env Rscript
# Capture gt's HTML output for every example in GTSQL_EXAMPLES.qmd.
#
# Output layout:
#   tests/fixtures/<NN_short_name>/
#     query.ggsql      <- GTSQL query, verbatim from the .qmd
#     data.parquet     <- input rows (deterministic)
#     expected.html    <- gt::as_raw_html(...) output
#     meta.toml        <- source dataset, gt version, capture timestamp,
#                         normalization profile name
#
# This script is run ONCE by a human, not by the agent. Re-run only when
# you intentionally re-baseline (e.g. after a gt upgrade you want to track).

suppressPackageStartupMessages({
  library(gt)
  library(dplyr)
  library(tidyr)
  library(lubridate)
  library(arrow)
})

SPEC_QMD <- Sys.getenv("GTSQL_EXAMPLES_QMD",
                       unset = file.path(Sys.getenv("GGSQL_SPEC_DIR", "/spec"),
                                         "GTSQL_EXAMPLES.qmd"))
OUT_DIR  <- "tests/fixtures"
dir.create(OUT_DIR, recursive = TRUE, showWarnings = FALSE)

stopifnot(file.exists(SPEC_QMD))
qmd <- readLines(SPEC_QMD, warn = FALSE)

# ---------------------------------------------------------------------------
# Parse the .qmd into example records: (number, title, r_code, gtsql_query)
# ---------------------------------------------------------------------------
heading_idx <- grep("^## [0-9]+\\.", qmd)
examples <- list()
for (i in seq_along(heading_idx)) {
  start <- heading_idx[i]
  end   <- if (i < length(heading_idx)) heading_idx[i + 1] - 1 else length(qmd)
  block <- qmd[start:end]
  header <- block[1]
  m <- regmatches(header, regexec("^## ([0-9]+)\\.\\s*(.*)$", header))[[1]]
  if (length(m) < 3) next
  num   <- as.integer(m[2])
  title <- m[3]
  # extract first ```{r} ... ``` and first ```sql ... ```
  r_start  <- grep("^```\\{r\\}", block)
  r_end    <- grep("^```\\s*$", block)
  sql_start <- grep("^```sql", block)
  if (length(r_start) == 0 || length(sql_start) == 0) next
  r_close   <- r_end[r_end > r_start[1]][1]
  sql_close <- r_end[r_end > sql_start[1]][1]
  r_code     <- block[(r_start[1] + 1):(r_close - 1)]
  gtsql_query <- block[(sql_start[1] + 1):(sql_close - 1)]
  slug <- gsub("[^a-z0-9]+", "_", tolower(title))
  slug <- gsub("^_|_$", "", slug)
  examples[[length(examples) + 1]] <- list(
    num = num, title = title, slug = slug,
    r_code = r_code, gtsql_query = gtsql_query
  )
}

cat(sprintf("Parsed %d examples from %s\n", length(examples), SPEC_QMD))

# ---------------------------------------------------------------------------
# Each example: evaluate the gt code in a fresh env, capture HTML + the
# tibble that gt was called on (so we can write it to Parquet).
# ---------------------------------------------------------------------------
# We instrument gt() / gt(rowname_col=, groupname_col=) to side-channel the
# input data into a per-example variable.
capture_one <- function(ex) {
  fixture_dir <- file.path(OUT_DIR, sprintf("%02d_%s", ex$num, ex$slug))
  dir.create(fixture_dir, recursive = TRUE, showWarnings = FALSE)

  captured_input <- NULL
  env <- new.env(parent = globalenv())
  local_gt <- function(data, ...) {
    captured_input <<- data
    gt::gt(data, ...)
  }
  env$gt <- local_gt

  code <- paste(ex$r_code, collapse = "\n")
  res <- tryCatch(
    eval(parse(text = code), envir = env),
    error = function(e) {
      message(sprintf("  [%02d %s] ERROR: %s", ex$num, ex$slug, conditionMessage(e)))
      NULL
    }
  )
  if (is.null(res) || is.null(captured_input)) {
    message(sprintf("  [%02d %s] SKIP (no gt object or no data captured)", ex$num, ex$slug))
    return(invisible(NULL))
  }

  html <- as.character(gt::as_raw_html(res))
  writeLines(html, file.path(fixture_dir, "expected.html"))
  writeLines(ex$gtsql_query, file.path(fixture_dir, "query.ggsql"))

  # Write data as Parquet (preserves types; readable by DuckDB).
  arrow::write_parquet(as.data.frame(captured_input),
                       file.path(fixture_dir, "data.parquet"))

  meta <- c(
    sprintf('example_number = %d', ex$num),
    sprintf('title = "%s"', gsub('"', '\\\\"', ex$title)),
    sprintf('gt_version = "%s"', as.character(packageVersion("gt"))),
    sprintf('captured_at = "%s"', format(Sys.time(), "%Y-%m-%dT%H:%M:%S%z")),
    'normalization_profile = "default"',
    '# allowed_diff = []  # add regex strings only with justification in AGENT_LOG.md'
  )
  writeLines(meta, file.path(fixture_dir, "meta.toml"))
  cat(sprintf("  [%02d %s] OK\n", ex$num, ex$slug))
}

for (ex in examples) capture_one(ex)
cat("done.\n")
