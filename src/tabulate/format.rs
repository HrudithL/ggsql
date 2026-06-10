//! Cell formatters for `FORMAT ... RENAMING * => '<spec>'`.
//!
//! Two mini-languages live inside a single-quoted RHS string:
//!
//! * `{:num <printf>}` — numeric formatter; current spec accepts a full
//!   printf body (with `%` introducer). The captured fixtures pre-date this
//!   change and use a legacy form without the `%` (e.g. `{:num ,d}`,
//!   `{:num .3f}`, `{:num .2e}`); both forms are accepted so the fixtures
//!   pass without re-capture.
//! * `{:time <strftime>}` — date / time formatter; strftime tokens are
//!   gt-compatible: `%d`, `%I`, `%H`, `%M`, `%S` render unpadded to match
//!   gt's named styles, matching the `%-d` / `%-I` GNU extensions used in
//!   the captured fixtures.
//!
//! Per-column locale (`FORMAT … SETTING locale => 'fr'`) selects month and
//! weekday names. Only `en` (default) and `fr` are needed by the corpus.

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

/// Boxed numeric formatter (`Option<f64>` → rendered string).
pub type NumFn = Box<dyn Fn(Option<f64>) -> String + Send + Sync>;
/// Boxed time formatter (string cell value → rendered string).
pub type TimeFn = Box<dyn Fn(Option<&str>) -> String + Send + Sync>;

/// Formatter kind selected based on the RHS template.
pub enum CellFmt {
    /// Numeric formatter.
    Numeric(NumFn),
    /// Time formatter.
    Time(TimeFn),
}

/// Parse a `FORMAT ... RENAMING * => '<rhs>'` template into a `CellFmt`
/// plus a `raw_html` flag for the column.
///
/// Returns `None` when the RHS is not a recognised formatter template.
pub fn build_format(rhs: &str, locale: Option<&str>) -> Option<(CellFmt, bool)> {
    if let Some((fmt, raw_html)) = build_num_format(rhs) {
        return Some((CellFmt::Numeric(fmt), raw_html));
    }
    if let Some(fmt) = build_time_format(rhs, locale) {
        return Some((CellFmt::Time(fmt), false));
    }
    None
}

// ============================================================================
// Numeric: `{:num <printf>}` with optional literal prefix / suffix.
// ============================================================================

fn build_num_format(rhs: &str) -> Option<(NumFn, bool)> {
    let open = rhs.find("{:num ")?;
    let after = &rhs[open + "{:num ".len()..];
    let close = after.find('}')?;
    let body = after[..close].trim();
    let prefix = rhs[..open].to_string();
    let suffix = rhs[open + "{:num ".len() + close + 1..].to_string();

    let spec = NumSpec::parse(body)?;
    // A literal `%` suffix outside the `{...}` triggers gt's percent
    // semantics (×100 scaling). `%%` is treated as a single literal `%`.
    let (suffix_clean, scale_pct) = parse_percent_suffix(&suffix);
    let raw_html = spec.conv == 'e';

    let render = move |v: Option<f64>| -> String {
        match v {
            None => "NA".to_string(),
            Some(x) if x.is_nan() => "NA".to_string(),
            Some(mut x) => {
                if scale_pct {
                    x *= 100.0;
                }
                format!("{}{}{}", prefix, spec.render(x), suffix_clean)
            }
        }
    };
    Some((Box::new(render), raw_html))
}

/// Detect a trailing literal `%` on the post-`{...}` suffix. The `%` stays
/// in the suffix either way; the boolean signals to the caller that the
/// numeric value should be multiplied by 100 first (`fmt_percent`
/// semantics). A literal `%%` collapses to a single `%` and does NOT
/// scale.
fn parse_percent_suffix(suffix: &str) -> (String, bool) {
    if let Some(stripped) = suffix.strip_suffix("%%") {
        let mut s = stripped.to_string();
        s.push('%');
        (s, false)
    } else if suffix.ends_with('%') {
        (suffix.to_string(), true)
    } else {
        (suffix.to_string(), false)
    }
}

struct NumSpec {
    /// Locale-aware thousands separator. Set by either `'` (current spec)
    /// or `,` (legacy capture).
    thousands: bool,
    /// `+` flag — always print a sign.
    force_sign: bool,
    /// Conversion: `d`, `f`, `e`.
    conv: char,
    /// Digits after `.`; `None` if unspecified (only used by `f` and `e`).
    precision: Option<u32>,
}

impl NumSpec {
    fn parse(body: &str) -> Option<Self> {
        // Optional `%` introducer (current spec requires it; the legacy
        // captures don't include it).
        let mut s = body.strip_prefix('%').unwrap_or(body);
        let mut thousands = false;
        let mut force_sign = false;
        // Flags loop. Stop on the first byte that is not a recognised flag.
        loop {
            let c = s.chars().next()?;
            match c {
                '\'' | ',' => {
                    thousands = true;
                    s = &s[c.len_utf8()..];
                }
                '+' => {
                    force_sign = true;
                    s = &s[c.len_utf8()..];
                }
                '0' | '-' | ' ' | '#' => {
                    s = &s[c.len_utf8()..];
                }
                _ => break,
            }
        }
        // Optional width (ignored — gt's defaults handle width).
        while s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            s = &s[1..];
        }
        // Optional precision.
        let mut precision: Option<u32> = None;
        if let Some(rest) = s.strip_prefix('.') {
            let mut n: u32 = 0;
            let mut consumed = 0;
            for c in rest.chars() {
                if let Some(d) = c.to_digit(10) {
                    n = n * 10 + d;
                    consumed += 1;
                } else {
                    break;
                }
            }
            precision = Some(n);
            s = &rest[consumed..];
        }
        let conv = s.chars().next()?;
        if !matches!(conv, 'd' | 'f' | 'e') {
            return None;
        }
        Some(NumSpec {
            thousands,
            force_sign,
            conv,
            precision,
        })
    }

    fn render(&self, x: f64) -> String {
        match self.conv {
            'd' => {
                let n = x.round() as i64;
                let abs = insert_thousands_opt(n.unsigned_abs() as i64, self.thousands);
                match (n < 0, self.force_sign) {
                    (true, _) => format!("-{}", abs),
                    (false, true) => format!("+{}", abs),
                    (false, false) => abs,
                }
            }
            'f' => {
                let p = self.precision.unwrap_or(6) as usize;
                let s = format!("{:.*}", p, x.abs());
                let body = if self.thousands {
                    let (int_part, frac_part) = match s.find('.') {
                        Some(i) => (&s[..i], &s[i..]),
                        None => (s.as_str(), ""),
                    };
                    let int_n: i64 = int_part.parse().unwrap_or(0);
                    format!("{}{}", insert_thousands(int_n), frac_part)
                } else {
                    s
                };
                match (x.is_sign_negative() && x != 0.0, self.force_sign) {
                    (true, _) => format!("-{}", body),
                    (false, true) => format!("+{}", body),
                    (false, false) => body,
                }
            }
            'e' => render_scientific_html(x, self.precision.unwrap_or(6) as usize),
            _ => format!("{}", x),
        }
    }
}

fn insert_thousands_opt(n: i64, on: bool) -> String {
    if on {
        insert_thousands(n)
    } else {
        n.to_string()
    }
}

fn insert_thousands(n: i64) -> String {
    let neg = n < 0;
    let abs = if neg {
        (n as i128).unsigned_abs().to_string()
    } else {
        n.to_string()
    };
    let len = abs.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, b) in abs.as_bytes().iter().enumerate() {
        let from_end = len - i;
        if i > 0 && from_end % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    if neg {
        format!("-{}", out)
    } else {
        out
    }
}

/// Render `x` as gt's HTML scientific notation:
///   `<mantissa>&nbsp;×&nbsp;10<sup style="font-size: 65%;"><exp></sup>`
/// with a Unicode minus sign on a negative exponent. The exponent is
/// printed without leading zeroes and without a `+` for positives.
fn render_scientific_html(x: f64, precision: usize) -> String {
    if x == 0.0 {
        return format!(
            "{:.*}&nbsp;×&nbsp;10<sup style=\"font-size: 65%;\">0</sup>",
            precision, 0.0
        );
    }
    let abs = x.abs();
    let exp = abs.log10().floor() as i32;
    let mantissa = x / 10f64.powi(exp);
    let mantissa_s = format!("{:.*}", precision, mantissa);
    if exp < 0 {
        format!(
            "{}&nbsp;×&nbsp;10<sup style=\"font-size: 65%;\">\u{2212}{}</sup>",
            mantissa_s,
            exp.unsigned_abs()
        )
    } else {
        format!(
            "{}&nbsp;×&nbsp;10<sup style=\"font-size: 65%;\">{}</sup>",
            mantissa_s, exp
        )
    }
}

// ============================================================================
// Time: `{:time <strftime>}` with optional prefix / suffix.
// ============================================================================

fn build_time_format(rhs: &str, locale: Option<&str>) -> Option<TimeFn> {
    let open = rhs.find("{:time ")?;
    let after = &rhs[open + "{:time ".len()..];
    let close = after.find('}')?;
    let fmt = after[..close].to_string();
    let prefix = rhs[..open].to_string();
    let suffix = rhs[open + "{:time ".len() + close + 1..].to_string();
    let lang = Lang::from_locale(locale);

    Some(Box::new(move |v: Option<&str>| -> String {
        match v {
            None => "NA".to_string(),
            Some(s) => {
                let body = match parse_temporal(s) {
                    Some(t) => render_strftime(&fmt, &t, lang),
                    None => s.to_string(),
                };
                format!("{}{}{}", prefix, body, suffix)
            }
        }
    }))
}

#[derive(Copy, Clone)]
enum Lang {
    En,
    Fr,
}

impl Lang {
    fn from_locale(loc: Option<&str>) -> Self {
        match loc {
            Some(s) if s.eq_ignore_ascii_case("fr") || s.to_ascii_lowercase().starts_with("fr") => {
                Lang::Fr
            }
            _ => Lang::En,
        }
    }
    fn month(self, m: u32) -> &'static str {
        const EN: [&str; 12] = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        const FR: [&str; 12] = [
            "janvier",
            "février",
            "mars",
            "avril",
            "mai",
            "juin",
            "juillet",
            "août",
            "septembre",
            "octobre",
            "novembre",
            "décembre",
        ];
        let idx = (m as usize).saturating_sub(1).min(11);
        match self {
            Lang::En => EN[idx],
            Lang::Fr => FR[idx],
        }
    }
    fn month_abbrev(self, m: u32) -> String {
        let full = self.month(m);
        let mut s: String = full.chars().take(3).collect();
        if matches!(self, Lang::En) {
            // Title-case (months are already title-cased in English, but
            // be explicit for the FR fallback path).
            if let Some(c) = s.chars().next() {
                s = c.to_ascii_uppercase().to_string() + &s[c.len_utf8()..];
            }
        }
        s
    }
    fn weekday(self, w: u32) -> &'static str {
        // 0 = Sunday … 6 = Saturday (chrono's `num_days_from_sunday`).
        const EN: [&str; 7] = [
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ];
        const FR: [&str; 7] = [
            "dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi",
        ];
        let idx = (w as usize).min(6);
        match self {
            Lang::En => EN[idx],
            Lang::Fr => FR[idx],
        }
    }
    fn weekday_abbrev(self, w: u32) -> String {
        let full = self.weekday(w);
        full.chars().take(3).collect()
    }
}

/// A single instant — date, time, or datetime — extracted from a string.
struct Temporal {
    date: Option<NaiveDate>,
    time: Option<NaiveTime>,
}

fn parse_temporal(s: &str) -> Option<Temporal> {
    let s = s.trim();
    // Datetime: YYYY-MM-DD HH:MM[:SS]
    for fmt in &["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(Temporal {
                date: Some(dt.date()),
                time: Some(dt.time()),
            });
        }
    }
    // Date: YYYY-MM-DD
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(Temporal {
            date: Some(d),
            time: None,
        });
    }
    // Time: HH:MM[:SS]
    for fmt in &["%H:%M:%S", "%H:%M"] {
        if let Ok(t) = NaiveTime::parse_from_str(s, fmt) {
            return Some(Temporal {
                date: None,
                time: Some(t),
            });
        }
    }
    None
}

/// Walk a strftime format string substituting `%X` tokens. Recognised
/// tokens reproduce gt's named-style output. The `-` modifier is accepted
/// (e.g. `%-d`) but ignored: padding-stripped variants are the default to
/// match gt's behaviour.
fn render_strftime(fmt: &str, t: &Temporal, lang: Lang) -> String {
    let mut out = String::with_capacity(fmt.len() * 2);
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        // Consume an optional `-` modifier (GNU strftime: pad-stripping).
        let mut had_dash = false;
        if matches!(chars.peek(), Some('-')) {
            had_dash = true;
            chars.next();
        }
        let Some(tok) = chars.next() else {
            out.push('%');
            if had_dash {
                out.push('-');
            }
            break;
        };
        match tok {
            '%' => out.push('%'),
            'Y' => {
                if let Some(d) = t.date {
                    out.push_str(&format!("{:04}", d.year()));
                }
            }
            'y' => {
                if let Some(d) = t.date {
                    out.push_str(&format!("{:02}", d.year() % 100));
                }
            }
            'm' => {
                if let Some(d) = t.date {
                    if had_dash {
                        out.push_str(&d.month().to_string());
                    } else {
                        out.push_str(&format!("{:02}", d.month()));
                    }
                }
            }
            'd' => {
                if let Some(d) = t.date {
                    // gt's day_month_year style renders the day unpadded;
                    // match that whether or not the format had `-`.
                    out.push_str(&d.day().to_string());
                }
            }
            'e' => {
                if let Some(d) = t.date {
                    out.push_str(&format!("{:>2}", d.day()));
                }
            }
            'B' => {
                if let Some(d) = t.date {
                    out.push_str(lang.month(d.month()));
                }
            }
            'b' | 'h' => {
                if let Some(d) = t.date {
                    out.push_str(&lang.month_abbrev(d.month()));
                }
            }
            'A' => {
                if let Some(d) = t.date {
                    out.push_str(lang.weekday(d.weekday().num_days_from_sunday()));
                }
            }
            'a' => {
                if let Some(d) = t.date {
                    out.push_str(&lang.weekday_abbrev(d.weekday().num_days_from_sunday()));
                }
            }
            'H' => {
                if let Some(t) = t.time {
                    // gt's h_m_p / 24-hour styles drop the leading zero on
                    // the hour; mirror that here.
                    out.push_str(&t.hour().to_string());
                }
            }
            'I' => {
                if let Some(t) = t.time {
                    let h12 = match t.hour() % 12 {
                        0 => 12,
                        h => h,
                    };
                    out.push_str(&h12.to_string());
                }
            }
            'M' => {
                if let Some(t) = t.time {
                    if had_dash {
                        out.push_str(&t.minute().to_string());
                    } else {
                        out.push_str(&format!("{:02}", t.minute()));
                    }
                }
            }
            'S' => {
                if let Some(t) = t.time {
                    if had_dash {
                        out.push_str(&t.second().to_string());
                    } else {
                        out.push_str(&format!("{:02}", t.second()));
                    }
                }
            }
            'p' => {
                if let Some(t) = t.time {
                    out.push_str(if t.hour() < 12 { "AM" } else { "PM" });
                }
            }
            'P' => {
                if let Some(t) = t.time {
                    out.push_str(if t.hour() < 12 { "am" } else { "pm" });
                }
            }
            other => {
                // Unknown token — pass through verbatim.
                out.push('%');
                if had_dash {
                    out.push('-');
                }
                out.push(other);
            }
        }
    }
    out
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn num(rhs: &str, v: f64) -> String {
        let (fmt, _) = build_format(rhs, None).expect("num parse");
        match fmt {
            CellFmt::Numeric(f) => f(Some(v)),
            _ => panic!("expected numeric"),
        }
    }
    fn time(rhs: &str, v: &str, loc: Option<&str>) -> String {
        let (fmt, _) = build_format(rhs, loc).expect("time parse");
        match fmt {
            CellFmt::Time(f) => f(Some(v)),
            _ => panic!("expected time"),
        }
    }

    #[test]
    fn num_legacy_decimals() {
        assert_eq!(num("{:num .3f}", 0.1111), "0.111");
        assert_eq!(num("{:num .3f}", 5550.0), "5550.000");
    }
    #[test]
    fn num_modern_decimals() {
        assert_eq!(num("{:num %.3f}", 0.1111), "0.111");
    }
    #[test]
    fn num_legacy_thousands_int() {
        assert_eq!(num("{:num ,d}", 5550.0), "5,550");
        assert_eq!(num("{:num ,d}", 8_880_000.0), "8,880,000");
    }
    #[test]
    fn num_modern_thousands_int() {
        assert_eq!(num("{:num %'d}", 5550.0), "5,550");
    }
    #[test]
    fn num_currency_prefix() {
        assert_eq!(num("${:num ,d}", 447_000.0), "$447,000");
    }
    #[test]
    fn num_percent_suffix_scales() {
        assert_eq!(num("{:num .1f}%", 0.085), "8.5%");
    }
    #[test]
    fn num_double_percent_no_scale() {
        assert_eq!(num("{:num .1f}%%", 8.5), "8.5%");
    }
    #[test]
    fn num_forced_sign() {
        assert_eq!(num("{:num %+.1f}%", 0.085), "+8.5%");
        assert_eq!(num("{:num %+.1f}%", -0.085), "-8.5%");
    }
    #[test]
    fn num_scientific_html() {
        let s = num("{:num .2e}", 1e-6);
        assert!(s.contains("<sup"), "got {}", s);
        assert!(s.contains("\u{2212}6"), "minus exponent: {}", s);
        assert!(s.starts_with("1.00&nbsp;"), "mantissa: {}", s);
    }
    #[test]
    fn time_english_date() {
        assert_eq!(
            time("{:time %B %-d, %Y}", "2015-01-15", None),
            "January 15, 2015"
        );
    }
    #[test]
    fn time_french_date() {
        assert_eq!(
            time("{:time %A %-d %B %Y}", "2015-01-15", Some("fr")),
            "jeudi 15 janvier 2015"
        );
    }
    #[test]
    fn time_h_m_p() {
        assert_eq!(time("{:time %-I:%M %p}", "13:35", None), "1:35 PM");
        assert_eq!(time("{:time %-I:%M %p}", "02:22", None), "2:22 AM");
    }
    #[test]
    fn time_datetime_long() {
        assert_eq!(
            time(
                "{:time %A, %B %-d, %Y at %-I:%M %p}",
                "2018-01-01 02:22",
                None
            ),
            "Monday, January 1, 2018 at 2:22 AM"
        );
    }
}
