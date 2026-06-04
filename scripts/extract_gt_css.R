#!/usr/bin/env Rscript
# Extract gt's default <style> block once and vendor it at
# src/tabulate/gt_default.css. The Rust HTML writer must emit this verbatim.
#
# Run by a human after a gt upgrade. The agent must never touch the output.

suppressPackageStartupMessages(library(gt))

OUT <- "src/tabulate/gt_default.css"
dir.create(dirname(OUT), recursive = TRUE, showWarnings = FALSE)

# gt 1.3 inlines its base styles as `style=` attributes on the rendered table
# rather than emitting a top-level <style> block. There is therefore no single
# CSS bundle to vendor; the per-element style strings come out of gt's own
# rendering and the Rust writer must produce the same `style=` content
# (already covered by the byte-level diff against expected.html).
#
# We still write a CSS file containing any *additional* <style> blocks gt
# does emit (e.g. for nanoplots) so the Rust writer has them available; if
# there are none, an empty marker file is written. Either way, the file
# exists so `include_str!` in src/tabulate/mod.rs is satisfied.

suppressPackageStartupMessages(library(gt))

OUT <- "src/tabulate/gt_default.css"
dir.create(dirname(OUT), recursive = TRUE, showWarnings = FALSE)

html <- as.character(gt::as_raw_html(gt::gt(data.frame(a = 1L, b = "x"))))
matches <- regmatches(html, gregexpr("<style[^>]*>[\\s\\S]*?</style>", html, perl = TRUE))[[1]]
css_chunks <- sub("</style>$", "", sub("^<style[^>]*>", "", matches))

writeLines(c(
  sprintf("/* Vendored from gt %s on %s.", packageVersion("gt"), Sys.Date()),
  " * Treat as opaque bytes. Re-run scripts/extract_gt_css.R to refresh.",
  " * gt 1.3+ inlines most styles via `style=` attributes; this file holds",
  " * only the supplementary <style> blocks gt emits (may be empty). */",
  ""
), OUT)
if (length(css_chunks) > 0) {
  cat(paste(css_chunks, collapse = "\n\n"), file = OUT, append = TRUE)
  cat(sprintf("wrote %d <style> block(s) (%d bytes) to %s\n",
              length(css_chunks), sum(nchar(css_chunks)), OUT))
} else {
  cat(sprintf("no <style> blocks in gt's output; wrote marker file to %s\n", OUT))
}
