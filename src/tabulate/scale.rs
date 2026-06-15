//! Continuous colour scale for TABULATE `SCALE background` clauses.
//!
//! Mirrors gt's `data_color()` -> `scales::col_numeric()` pipeline:
//!
//! 1. Resolve named palette stops (`viridis`, `RdYlGn`, ...) or accept the
//!    explicit stops listed in `TO (c1, c2, ...)`.
//! 2. For each numeric value `v` in the target column, compute the
//!    normalized position `t = (v - lo) / (hi - lo)` (optionally
//!    log10-transformed first).
//! 3. Linearly interpolate the stops in CIE Lab space (`grDevices::colorRamp(...,
//!    space = "Lab")` — what `scales::colour_ramp` uses under the hood).
//! 4. Values outside the domain (or non-finite / null) get
//!    `na.color = "#808080"`.
//! 5. Pick a foreground text colour via gt's `ideal_fgnd_color`: YIQ
//!    perceived brightness with threshold 156 — `>= 156` → black, else
//!    white.
//!
//! Colours are emitted as uppercase 6-digit hex (`#RRGGBB`) so byte-level
//! diffs against gt's reference output stay tidy.

use crate::plot::scale::palettes::get_color_palette;
use crate::tabulate::ast::ScalePalette;
use palette::white_point::D65;
use palette::{FromColor, IntoColor, Lab, LinSrgb, Mix, Srgb};

/// Out-of-domain / NA fill colour, matching `scales::col_numeric`'s default.
pub const NA_COLOR: &str = "#808080";

/// R / X11 colour names whose RGB values differ from CSS3. gt -> scales ->
/// farver decodes these via the X11 table, so a SCALE clause written as
/// e.g. `TO (white, green)` must produce `#00FF00`, not CSS's `#008000`.
fn r_named_color_override(name: &str) -> Option<Srgb<f32>> {
    // Lowercase, strip spaces (R is case-insensitive and ignores spaces).
    let key: String = name.chars().filter(|c| !c.is_whitespace()).collect();
    let key = key.to_ascii_lowercase();
    let (r, g, b) = match key.as_str() {
        "green" => (0u8, 255, 0),
        "maroon" => (176, 48, 96),
        "purple" => (160, 32, 240),
        "gray" | "grey" => (190, 190, 190),
        _ => return None,
    };
    Some(Srgb::new(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
    ))
}

/// Parse a colour name / hex into a display `Srgb<f32>`. Recognises R / X11
/// names that differ from CSS3 (`green`, `maroon`, `purple`, `gray`) before
/// falling back to the CSS parser. Returns mid-grey on parse failure so a
/// typo'd palette doesn't crash the renderer.
fn parse_srgb(c: &str) -> Srgb<f32> {
    if let Some(s) = r_named_color_override(c) {
        return s;
    }
    match csscolorparser::parse(c) {
        Ok(p) => Srgb::new(p.r, p.g, p.b),
        Err(_) => Srgb::new(0.5, 0.5, 0.5),
    }
}

/// Parse any CSS colour string (named, `#rgb`, `#rrggbb`, `rgb(...)`)
/// into the uppercase 6-digit hex form (`#RRGGBB`) gt uses in HTML
/// output. Falls back to mid-grey on parse failure.
pub fn parse_to_hex_upper(c: &str) -> String {
    srgb_to_hex_upper(parse_srgb(c))
}

fn srgb_to_hex_upper(c: Srgb<f32>) -> String {
    let r = (c.red.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (c.green.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (c.blue.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

/// Resolve a `ScalePalette` to the list of input `Srgb` stops.
pub fn resolve_stops(p: &ScalePalette) -> Vec<Srgb<f32>> {
    match p {
        ScalePalette::Stops(s) => s.iter().map(|c| parse_srgb(c)).collect(),
        ScalePalette::Named(name) => match get_color_palette(name) {
            Some(slice) => slice.iter().map(|c| parse_srgb(c)).collect(),
            None => vec![Srgb::new(0.5, 0.5, 0.5)],
        },
    }
}

/// Linearly interpolate a list of colour stops in CIE Lab (D65) at
/// parameter `t` in `[0, 1]`.
fn interp_lab(stops: &[Srgb<f32>], t: f32) -> Srgb<f32> {
    debug_assert!(!stops.is_empty());
    if stops.len() == 1 {
        return stops[0];
    }
    let t = t.clamp(0.0, 1.0);
    let n = stops.len() - 1;
    let seg_f = t * n as f32;
    let seg = (seg_f.floor() as usize).min(n - 1);
    let sub_t = seg_f - seg as f32;
    let a: Lab<D65, f32> = Lab::from_color(LinSrgb::from(stops[seg]));
    let b: Lab<D65, f32> = Lab::from_color(LinSrgb::from(stops[seg + 1]));
    let m = a.mix(b, sub_t);
    let lin: LinSrgb<f32> = m.into_color();
    Srgb::from(lin)
}

/// gt's `ideal_fgnd_color`: YIQ perceived-brightness check with a 156
/// threshold. Brightness >= 156 picks `#000000`, else `#FFFFFF`.
pub fn ideal_fg(bg_hex: &str) -> &'static str {
    let h = bg_hex.trim_start_matches('#');
    if h.len() < 6 {
        return "#000000";
    }
    let r = u32::from_str_radix(&h[0..2], 16).unwrap_or(0) as f64;
    let g = u32::from_str_radix(&h[2..4], 16).unwrap_or(0) as f64;
    let b = u32::from_str_radix(&h[4..6], 16).unwrap_or(0) as f64;
    let yiq = (r * 299.0 + g * 587.0 + b * 114.0) / 1000.0;
    if yiq >= 156.0 {
        "#000000"
    } else {
        "#FFFFFF"
    }
}

/// Map one numeric value to a hex colour given a domain, palette stops, and
/// an optional `VIA log10` transform.
///
/// Returns `NA_COLOR` for null / non-finite values and for values outside the
/// supplied domain.
pub fn map_value(
    v: Option<f64>,
    domain: (f64, f64),
    stops: &[Srgb<f32>],
    transform: Option<&str>,
) -> String {
    let Some(v) = v else {
        return NA_COLOR.to_string();
    };
    if !v.is_finite() {
        return NA_COLOR.to_string();
    }
    let log = matches!(transform, Some(t) if t.eq_ignore_ascii_case("log10"));
    if v < domain.0 || v > domain.1 {
        return NA_COLOR.to_string();
    }
    let (lo, hi, vv) = if log {
        if v <= 0.0 {
            return NA_COLOR.to_string();
        }
        // gt's behaviour with `domain = c(0, hi)` + `trans = "log10"`:
        // domain.0 <= 0 collapses to a transformed lower bound of 0
        // (i.e. as if the value 1 sits at the floor of the scale).
        let lo_t = if domain.0 <= 0.0 {
            0.0
        } else {
            domain.0.log10()
        };
        let hi_t = if domain.1 <= 0.0 {
            return NA_COLOR.to_string();
        } else {
            domain.1.log10()
        };
        (lo_t, hi_t, v.log10())
    } else {
        (domain.0, domain.1, v)
    };
    let t = if (hi - lo).abs() < f64::EPSILON {
        0.0
    } else {
        ((vv - lo) / (hi - lo)) as f32
    };
    let col = interp_lab(stops, t);
    srgb_to_hex_upper(col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn na_for_out_of_domain() {
        let stops = vec![parse_srgb("white"), parse_srgb("black")];
        assert_eq!(map_value(Some(-1.0), (0.0, 1.0), &stops, None), NA_COLOR);
        assert_eq!(map_value(Some(2.0), (0.0, 1.0), &stops, None), NA_COLOR);
        assert_eq!(map_value(None, (0.0, 1.0), &stops, None), NA_COLOR);
    }

    #[test]
    fn ideal_fg_picks_black_on_light_bg() {
        assert_eq!(ideal_fg("#FFFFFF"), "#000000");
        assert_eq!(ideal_fg("#000000"), "#FFFFFF");
        assert_eq!(ideal_fg("#808080"), "#FFFFFF");
    }
}
