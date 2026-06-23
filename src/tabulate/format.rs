//! Cell formatters for `FORMAT ... RENAMING * => '<spec>'`.
//!
//! The RHS is a literal template with a single `{...}` formatter span. Any
//! text before or after the span is appended verbatim to every formatted
//! cell — `'${:num \'d}abc'` formats the number with thousands separators
//! then renders `${value}abc`. There is no special handling of `%`, `$`,
//! or any other character; they are all literal.
//!
//! Two formatter mini-languages live inside the `{...}`:
//!
//! * `{:num <printf-body>}` — numeric formatter. The body is a
//!   `printf(3)`-style conversion specification *with* the leading `%`
//!   introducer (e.g. `{:num %\'d}`, `{:num %.3f}`, `{:num %.2e}`). The
//!   only thousands flag is `'` (written as `\'` inside the
//!   single-quoted RHS).
//! * `{:time <strftime>}` — date / time formatter; strftime tokens are
//!   gt-compatible: `%d`, `%I`, `%H`, `%M`, `%S` render unpadded to match
//!   gt's named styles, matching the `%-d` / `%-I` GNU extensions used in
//!   the captured fixtures.
//!
//! Plus three string-only case transforms (keyword case-insensitive,
//! matching SQL — `{:title}` and `{:Title}` are equivalent):
//!
//! * `{:title}` — title-case (first letter of each word).
//! * `{:upper}` — all upper-case.
//! * `{:lower}` — all lower-case.
//!
//! Per-column locale (`FORMAT … SETTING locale => 'fr'`) selects month and
//! weekday names. Only `en` (default) and `fr` are needed by the corpus.

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

/// Boxed numeric formatter (`Option<f64>` → rendered string).
pub type NumFn = Box<dyn Fn(Option<f64>) -> String + Send + Sync>;
/// Boxed time formatter (string cell value → rendered string).
pub type TimeFn = Box<dyn Fn(Option<&str>) -> String + Send + Sync>;
/// Boxed string formatter (string cell value → rendered string).
/// Used by case transforms (`{:title}`, `{:upper}`, `{:lower}`).
pub type StringFn = Box<dyn Fn(Option<&str>) -> String + Send + Sync>;

/// Formatter kind selected based on the RHS template.
pub enum CellFmt {
    /// Numeric formatter.
    Numeric(NumFn),
    /// Time formatter.
    Time(TimeFn),
    /// String / case-transform formatter.
    Str(StringFn),
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
    if let Some(fmt) = build_string_format(rhs) {
        return Some((CellFmt::Str(fmt), false));
    }
    None
}

// ============================================================================
// String / case transforms: `{:title}`, `{:upper}`, `{:lower}`, `{}`
// with optional literal prefix / suffix.
// ============================================================================

fn build_string_format(rhs: &str) -> Option<StringFn> {
    let open = rhs.find('{')?;
    let after = &rhs[open + 1..];
    let close = after.find('}')?;
    let body = &after[..close];
    let prefix = rhs[..open].to_string();
    let suffix = rhs[open + 1 + close + 1..].to_string();

    // Body is either empty (`{}`, identity) or `:<keyword>`. Numeric and
    // time bodies are handled by their dedicated builders above and won't
    // reach here. The keyword match is case-insensitive (`{:title}`,
    // `{:Title}`, `{:TITLE}` are all equivalent), matching SQL keyword
    // case-handling.
    let kind = if body.is_empty() {
        StringTransform::Identity
    } else if let Some(kw) = body.strip_prefix(':') {
        match kw.to_ascii_lowercase().as_str() {
            "title" => StringTransform::Title,
            "upper" => StringTransform::Upper,
            "lower" => StringTransform::Lower,
            _ => return None,
        }
    } else {
        return None;
    };

    Some(Box::new(move |v: Option<&str>| -> String {
        match v {
            None => "NA".to_string(),
            Some(s) => format!("{}{}{}", prefix, kind.apply(s), suffix),
        }
    }))
}

#[derive(Copy, Clone)]
enum StringTransform {
    Identity,
    Title,
    Upper,
    Lower,
}

impl StringTransform {
    fn apply(self, s: &str) -> String {
        match self {
            StringTransform::Identity => s.to_string(),
            StringTransform::Upper => s.to_uppercase(),
            StringTransform::Lower => s.to_lowercase(),
            // Title-case the input the same way `tools::toTitleCase`
            // does: lowercase everything, then upper-case the first
            // character of every whitespace-separated word.
            StringTransform::Title => {
                let lower = s.to_lowercase();
                let mut out = String::with_capacity(lower.len());
                let mut at_word_start = true;
                for c in lower.chars() {
                    if c.is_whitespace() {
                        out.push(c);
                        at_word_start = true;
                    } else if at_word_start {
                        for u in c.to_uppercase() {
                            out.push(u);
                        }
                        at_word_start = false;
                    } else {
                        out.push(c);
                    }
                }
                out
            }
        }
    }
}

// ============================================================================
// Numeric: `{:num <printf>}` with optional literal prefix / suffix.
// ============================================================================

/// Find the byte offset of `{:<keyword> ` (with trailing space) inside
/// `rhs`, matching the keyword case-insensitively. Returns `(open,
/// after_open)` where `open` is the offset of `{` and `after_open` is
/// the offset just past the trailing space.
fn find_keyword(rhs: &str, keyword: &str) -> Option<(usize, usize)> {
    let needle_lower = format!("{{:{} ", keyword.to_ascii_lowercase());
    let needle_len = needle_lower.len();
    let rhs_lower = rhs.to_ascii_lowercase();
    let open = rhs_lower.find(&needle_lower)?;
    Some((open, open + needle_len))
}

fn build_num_format(rhs: &str) -> Option<(NumFn, bool)> {
    let (open, after_open) = find_keyword(rhs, "num")?;
    let after = &rhs[after_open..];
    let close = after.find('}')?;
    let body = after[..close].trim();
    let prefix = rhs[..open].to_string();
    let suffix = rhs[after_open + close + 1..].to_string();

    let spec = NumSpec::parse(body)?;
    let raw_html = spec.conv == 'e';

    let render = move |v: Option<f64>| -> String {
        match v {
            None => "NA".to_string(),
            Some(x) if x.is_nan() => "NA".to_string(),
            Some(x) => format!("{}{}{}", prefix, spec.render(x), suffix),
        }
    };
    Some((Box::new(render), raw_html))
}

struct NumSpec {
    /// Locale-aware thousands separator. Enabled by the `'` flag.
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
        // The `%` printf introducer is required (matches the spec). A body
        // that does not start with `%` is not a numeric formatter.
        let mut s = body.strip_prefix('%')?;
        let mut thousands = false;
        let mut force_sign = false;
        // Flags loop. Stop on the first byte that is not a recognised flag.
        loop {
            let c = s.chars().next()?;
            match c {
                '\'' => {
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
                    (true, true) => format!("\u{2212}{}", abs),
                    (true, false) => format!("-{}", abs),
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
                    (true, true) => format!("\u{2212}{}", body),
                    (true, false) => format!("-{}", body),
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
///
/// When the exponent is 0 (i.e. `1 <= |x| < 10`, or `x == 0`) the
/// `× 10⁰` suffix carries no information, so we render the mantissa
/// alone.
fn render_scientific_html(x: f64, precision: usize) -> String {
    if x == 0.0 {
        return format!("{:.*}", precision, 0.0);
    }
    let abs = x.abs();
    let exp = abs.log10().floor() as i32;
    let mantissa = x / 10f64.powi(exp);
    let mantissa_s = format!("{:.*}", precision, mantissa);
    match exp.cmp(&0) {
        std::cmp::Ordering::Equal => mantissa_s,
        std::cmp::Ordering::Less => format!(
            "{}&nbsp;×&nbsp;10<sup style=\"font-size: 65%;\">\u{2212}{}</sup>",
            mantissa_s,
            exp.unsigned_abs()
        ),
        std::cmp::Ordering::Greater => format!(
            "{}&nbsp;×&nbsp;10<sup style=\"font-size: 65%;\">{}</sup>",
            mantissa_s, exp
        ),
    }
}

// ============================================================================
// Time: `{:time <strftime>}` with optional prefix / suffix.
// ============================================================================

fn build_time_format(rhs: &str, locale: Option<&str>) -> Option<TimeFn> {
    let (open, after_open) = find_keyword(rhs, "time")?;
    let after = &rhs[after_open..];
    let close = after.find('}')?;
    let fmt = after[..close].to_string();
    let prefix = rhs[..open].to_string();
    let suffix = rhs[after_open + close + 1..].to_string();
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
        assert_eq!(num("{:num %.3f}", 0.1111), "0.111");
        assert_eq!(num("{:num %.3f}", 5550.0), "5550.000");
    }
    #[test]
    fn num_thousands_apostrophe_int() {
        assert_eq!(num("{:num %'d}", 5550.0), "5,550");
        assert_eq!(num("{:num %'d}", 8_880_000.0), "8,880,000");
    }
    #[test]
    fn num_thousands_apostrophe() {
        assert_eq!(num("{:num %'d}", 5550.0), "5,550");
    }
    #[test]
    fn num_currency_prefix() {
        assert_eq!(num("${:num %'d}", 447_000.0), "$447,000");
    }
    #[test]
    fn num_percent_suffix_is_literal() {
        // A trailing `%` is just an appended literal character — no
        // scaling, no special handling.
        assert_eq!(num("{:num %.1f}%", 8.5), "8.5%");
        assert_eq!(num("{:num %.1f}%", 0.085), "0.1%");
    }
    #[test]
    fn num_arbitrary_suffix_is_literal() {
        // Anything after the `{...}` is tacked on verbatim to each
        // formatted number.
        assert_eq!(num("{:num %'d}abc", 5550.0), "5,550abc");
        assert_eq!(num("{:num %.1f}%%", 8.5), "8.5%%");
    }
    #[test]
    fn num_forced_sign() {
        assert_eq!(num("{:num %+.1f}%", 8.5), "+8.5%");
        assert_eq!(num("{:num %+.1f}%", -8.5), "\u{2212}8.5%");
    }
    #[test]
    fn num_unforced_negative_is_ascii_minus() {
        // Without `+`, negatives keep the ASCII hyphen-minus.
        assert_eq!(num("{:num %.1f}", -1.5), "-1.5");
    }
    #[test]
    fn num_percent_introducer_is_required() {
        // The `%` introducer is now mandatory; a body without it does
        // not parse as a numeric formatter, so build_format returns
        // None and the RHS is treated as plain text.
        assert!(build_format("{:num .2f}", None).is_none());
        assert!(build_format("{:num 'd}", None).is_none());
    }
    #[test]
    fn num_scientific_html() {
        let s = num("{:num %.2e}", 1e-6);
        assert!(s.contains("<sup"), "got {}", s);
        assert!(s.contains("\u{2212}6"), "minus exponent: {}", s);
        assert!(s.starts_with("1.00&nbsp;"), "mantissa: {}", s);
    }
    #[test]
    fn num_scientific_exp_zero_is_plain_mantissa() {
        // 1 <= |x| < 10 ⇒ exponent is 0; emit the mantissa alone,
        // not `× 10⁰`.
        assert_eq!(num("{:num %.2e}", 4.2), "4.20");
        assert_eq!(num("{:num %.2e}", 0.0), "0.00");
        assert_eq!(num("{:num %.2e}", -1.5), "-1.50");
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

    fn s(rhs: &str, v: &str) -> String {
        let (fmt, _) = build_format(rhs, None).expect("string parse");
        match fmt {
            CellFmt::Str(f) => f(Some(v)),
            _ => panic!("expected string"),
        }
    }

    #[test]
    fn raw_passthrough_identity() {
        // `{}` with no keyword passes the cell value through unchanged.
        assert_eq!(s("{}", "hello"), "hello");
    }
    #[test]
    fn raw_passthrough_with_prefix_suffix() {
        // Literal prefix/suffix outside the `{}` is tacked on verbatim
        // to every cell.
        assert_eq!(s("${}abc", "42"), "$42abc");
        assert_eq!(s(">>{}", "x"), ">>x");
    }
    #[test]
    fn case_keywords_are_case_insensitive() {
        // The keyword match in build_string_format is
        // ASCII-case-insensitive, matching SQL keyword handling.
        assert_eq!(s("{:title}", "hello world"), "Hello World");
        assert_eq!(s("{:Title}", "hello world"), "Hello World");
        assert_eq!(s("{:TITLE}", "hello world"), "Hello World");
        assert_eq!(s("{:upper}", "abc"), "ABC");
        assert_eq!(s("{:UPPER}", "abc"), "ABC");
        assert_eq!(s("{:Upper}", "abc"), "ABC");
        assert_eq!(s("{:lower}", "ABC"), "abc");
        assert_eq!(s("{:LOWER}", "ABC"), "abc");
    }
    #[test]
    fn num_keyword_is_case_insensitive() {
        assert_eq!(num("{:NUM %d}", 42.0), "42");
        assert_eq!(num("{:Num %.1f}", 4.56), "4.6");
    }
    #[test]
    fn time_keyword_is_case_insensitive() {
        assert_eq!(time("{:TIME %Y}", "2024-06-23", None), "2024");
        assert_eq!(time("{:Time %Y}", "2024-06-23", None), "2024");
    }
}
