//! Dates, calendars and time.

use super::Output;
use super::{Opts, Tool, fmt, grid, table, tool};

/// Days since 1970-01-01 of a proleptic Gregorian date.
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// The date of a day since 1970-01-01.
pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        if m <= 2 {
            yoe + era * 400 + 1
        } else {
            yoe + era * 400
        },
        m,
        d,
    )
}

pub fn is_leap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

pub fn month_days(y: i64, m: i64) -> i64 {
    match m {
        2 if is_leap(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

pub const MONTHS: [&str; 12] = [
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
pub const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// 0 for Monday.
pub fn weekday(days: i64) -> usize {
    (days + 3).rem_euclid(7) as usize
}

fn month_number(name: &str) -> Option<i64> {
    let n = name.trim_end_matches('.').to_lowercase();
    if n.len() < 3 {
        return None;
    }
    MONTHS
        .iter()
        .position(|m| m.to_lowercase().starts_with(&n))
        .map(|i| i as i64 + 1)
}

/// A date and time as seconds since 1970, UTC: ISO dates and times,
/// `3/5/2024` (US), `5.3.2024`, `March 5, 2024`, `5 March 2024`, or a Unix
/// timestamp of ten digits.
pub fn parse_datetime(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.len() >= 9 && s.chars().all(|c| c.is_ascii_digit()) {
        let n: f64 = s.parse().ok()?;
        return Some(if s.len() >= 13 { n / 1000.0 } else { n });
    }
    let (date, time) = match s
        .find(['T', ' '])
        .filter(|&i| s[..i].contains(['-', '/', '.']))
    {
        Some(i) if s[..i].chars().filter(|c| c.is_ascii_digit()).count() >= 4 => {
            (&s[..i], s[i + 1..].trim())
        }
        _ => (s, ""),
    };
    let (y, m, d) = parse_date_part(date).or_else(|| {
        // Month names: the time is whatever follows the year.
        parse_date_part(s).map(|ymd| ymd)
    })?;
    if !(1..=12).contains(&m) || d < 1 || d > month_days(y, m) {
        return None;
    }
    let mut secs = days_from_civil(y, m, d) as f64 * 86400.0;
    let time = if parse_date_part(date).is_some() {
        time
    } else {
        ""
    };
    if !time.is_empty() {
        let (clock, zone) = split_zone(time);
        let lower = clock.to_lowercase();
        let pm = lower.ends_with("pm");
        let am = lower.ends_with("am");
        let clock = lower.trim_end_matches("pm").trim_end_matches("am").trim();
        let parts: Vec<f64> = clock
            .split(':')
            .map(|p| p.parse().ok())
            .collect::<Option<_>>()?;
        let mut h = *parts.first()?;
        if pm && h < 12.0 {
            h += 12.0;
        }
        if am && h == 12.0 {
            h = 0.0;
        }
        secs +=
            h * 3600.0 + parts.get(1).unwrap_or(&0.0) * 60.0 + parts.get(2).unwrap_or(&0.0) - zone;
    }
    Some(secs)
}

/// A time and its zone offset in seconds: `12:00Z`, `12:00+02:00`.
fn split_zone(t: &str) -> (&str, f64) {
    if let Some(c) = t
        .strip_suffix('Z')
        .or_else(|| t.strip_suffix(" UTC"))
        .or_else(|| t.strip_suffix("UTC"))
    {
        return (c.trim(), 0.0);
    }
    if let Some(i) = t.rfind(['+', '-']).filter(|&i| i > 0) {
        let z = &t[i + 1..].replace(':', "");
        if let (Ok(n), 4) = (z.parse::<f64>(), z.len()) {
            let sign = if t.as_bytes()[i] == b'-' { -1.0 } else { 1.0 };
            return (
                &t[..i],
                sign * ((n / 100.0).floor() * 3600.0 + (n % 100.0) * 60.0),
            );
        }
    }
    (t, 0.0)
}

fn parse_date_part(s: &str) -> Option<(i64, i64, i64)> {
    let s = s.trim().trim_end_matches(',');
    let nums: Vec<&str> = s.split(['-', '/', '.']).collect();
    if nums.len() == 3
        && nums
            .iter()
            .all(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    {
        let n: Vec<i64> = nums.iter().map(|n| n.parse().unwrap()).collect();
        return Some(if nums[0].len() == 4 {
            (n[0], n[1], n[2])
        } else {
            let y = if nums[2].len() == 2 {
                2000 + n[2] - if n[2] > 69 { 100 } else { 0 }
            } else {
                n[2]
            };
            if s.contains('/') {
                (y, n[0], n[1])
            } else {
                (y, n[1], n[0])
            }
        });
    }
    // Words: "March 5, 2024", "5 March 2024", "Tue, 5 Mar 2024".
    let words: Vec<&str> = s.split([' ', ',']).filter(|w| !w.is_empty()).collect();
    let month = words.iter().find_map(|w| month_number(w))?;
    let nums: Vec<i64> = words
        .iter()
        .filter_map(|w| w.trim_end_matches(|c: char| c.is_alphabetic()).parse().ok())
        .collect();
    let year = *nums.iter().find(|n| **n > 31)?;
    let day = nums.iter().find(|n| **n <= 31).copied().unwrap_or(1);
    Some((year, month, day))
}

/// A date for people: `Tuesday 5 March 2024`, with the time unless it is
/// midnight.
pub fn format_unix(secs: f64) -> String {
    let days = (secs / 86400.0).floor() as i64;
    let (y, m, d) = civil_from_days(days);
    let rest = secs - days as f64 * 86400.0;
    let time = if rest.abs() < 0.5 {
        String::new()
    } else {
        let t = rest.round() as i64;
        format!(" {:02}:{:02}:{:02} UTC", t / 3600, t % 3600 / 60, t % 60)
    };
    format!(
        "{} {d} {} {y}{time}",
        WEEKDAYS[weekday(days)],
        MONTHS[m as usize - 1]
    )
}

pub fn iso(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// A length of time in words: `3 days, 4 hours`.
pub fn duration_words(secs: f64) -> String {
    if !secs.is_finite() {
        return "forever".into();
    }
    let s = secs.abs();
    let units = [
        ("year", 365.25 * 86400.0),
        ("day", 86400.0),
        ("hour", 3600.0),
        ("minute", 60.0),
        ("second", 1.0),
    ];
    if s >= 1e6 * 365.25 * 86400.0 {
        return format!("{} years", fmt(s / (365.25 * 86400.0), 0));
    }
    if s < 1.0 {
        return format!("{} seconds", fmt(s, 3));
    }
    let mut left = s.round();
    let mut parts = Vec::new();
    for (name, size) in units {
        let n = (left / size).floor();
        if n >= 1.0 {
            parts.push(format!(
                "{} {name}{}",
                fmt(n, 0),
                if n == 1.0 { "" } else { "s" }
            ));
            left -= n * size;
        }
        if parts.len() == 2 {
            break;
        }
    }
    parts.join(", ")
}

impl Opts<'_> {
    /// Field `i` as days since 1970; empty or "today" is today.
    pub fn date(&self, i: usize) -> Result<i64, String> {
        Ok((self.datetime(i)? / 86400.0).floor() as i64)
    }

    /// Field `i` as seconds since 1970; empty or "now" is now.
    pub fn datetime(&self, i: usize) -> Result<f64, String> {
        let raw = self.field(i);
        match raw.to_lowercase().as_str() {
            "" | "today" | "now" => Ok((self.now()? / 1000.0).floor()),
            "tomorrow" => Ok((self.now()? / 1000.0 / 86400.0).floor() * 86400.0 + 86400.0),
            "yesterday" => Ok((self.now()? / 1000.0 / 86400.0).floor() * 86400.0 - 86400.0),
            _ => parse_datetime(raw)
                .ok_or_else(|| format!("“{raw}” is not a date; write it as 2024-03-05.")),
        }
    }
}

/// Whole years, months and days from one date to a later one.
fn ymd_between(a: i64, b: i64) -> (i64, i64, i64) {
    let (y1, m1, d1) = civil_from_days(a);
    let (y2, m2, d2) = civil_from_days(b);
    let mut months = (y2 - y1) * 12 + (m2 - m1);
    let mut days = d2 - d1;
    if days < 0 {
        months -= 1;
        let (py, pm) = if m2 == 1 { (y2 - 1, 12) } else { (y2, m2 - 1) };
        days += month_days(py, pm);
    }
    (months / 12, months % 12, days)
}

fn plural(n: i64, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

fn age(_: &str, o: &mut Opts) -> Result<Output, String> {
    let born = o.date(0)?;
    let at = o.date(1)?;
    if at < born {
        return Err("The second date is before the birth date.".into());
    }
    let (y, m, d) = ymd_between(born, at);
    let (by, bm, bd) = civil_from_days(born);
    let (ay, ..) = civil_from_days(at);
    let birthday = |y: i64| days_from_civil(y, bm, bd.min(month_days(y, bm)));
    let next = if birthday(ay) >= at {
        birthday(ay)
    } else {
        birthday(ay + 1)
    };
    let days = at - born;
    let _ = by;
    Ok(table(&[
        (
            "Age",
            format!(
                "{}, {}, {}",
                plural(y, "year"),
                plural(m, "month"),
                plural(d, "day")
            ),
        ),
        ("In months", fmt((y * 12 + m) as f64, 0)),
        (
            "In weeks",
            format!(
                "{} and {}",
                fmt((days / 7) as f64, 0),
                plural(days % 7, "day")
            ),
        ),
        ("In days", fmt(days as f64, 0)),
        ("In hours", fmt(days as f64 * 24.0, 0)),
        ("Born on a", WEEKDAYS[weekday(born)].into()),
        (
            "Next birthday",
            format!(
                "{} ({} away, turning {})",
                iso(next),
                plural(next - at, "day"),
                civil_from_days(next).0 - civil_from_days(born).0
            ),
        ),
    ]))
}

fn business_days_between(a: i64, b: i64, holidays: &[i64], saturday: bool) -> i64 {
    (a.min(b)..a.max(b))
        .filter(|d| {
            let w = weekday(*d);
            w < 5 || (saturday && w == 5)
        })
        .filter(|d| !holidays.contains(d))
        .count() as i64
}

fn date_diff(_: &str, o: &mut Opts) -> Result<Output, String> {
    let a = o.date(0)?;
    let mut b = o.date(1)?;
    if o.flag {
        b += if b >= a { 1 } else { -1 };
    }
    let (lo, hi) = (a.min(b), a.max(b));
    let (y, m, d) = ymd_between(lo, hi);
    let days = hi - lo;
    Ok(table(&[
        (
            "Difference",
            format!(
                "{}, {}, {}",
                plural(y, "year"),
                plural(m, "month"),
                plural(d, "day")
            ),
        ),
        ("Days", fmt(days as f64, 0)),
        (
            "Weeks",
            format!(
                "{} and {}",
                fmt((days / 7) as f64, 0),
                plural(days % 7, "day")
            ),
        ),
        ("Months", fmt((y * 12 + m) as f64 + d as f64 / 30.44, 2)),
        ("Years", fmt(days as f64 / 365.2425, 3)),
        (
            "Weekdays (Mon–Fri)",
            fmt(business_days_between(lo, hi, &[], false) as f64, 0),
        ),
        ("Hours", fmt(days as f64 * 24.0, 0)),
        (
            "Direction",
            if b >= a { "forward" } else { "backward" }.into(),
        ),
    ]))
}

fn add_months(days: i64, months: i64) -> i64 {
    let (y, m, d) = civil_from_days(days);
    let total = y * 12 + (m - 1) + months;
    let (ny, nm) = (total.div_euclid(12), total.rem_euclid(12) + 1);
    days_from_civil(ny, nm, d.min(month_days(ny, nm)))
}

fn date_add(_: &str, o: &mut Opts) -> Result<Output, String> {
    let start = o.date(0)?;
    let n = o.int(1)? * if o.choice(1) == "subtract" { -1 } else { 1 };
    let result = match o.mode {
        "weeks" => start + n * 7,
        "months" => add_months(start, n),
        "years" => add_months(start, n * 12),
        "business" => {
            let mut d = start;
            let mut left = n.abs();
            while left > 0 {
                d += n.signum();
                if weekday(d) < 5 {
                    left -= 1;
                }
            }
            d
        }
        _ => start + n,
    };
    Ok(table(&[
        ("Date", iso(result)),
        ("", format_unix(result as f64 * 86400.0)),
    ]))
}

fn business_days(_: &str, o: &mut Opts) -> Result<Output, String> {
    let a = o.date(0)?;
    let mut b = o.date(1)?;
    if o.flag {
        b += 1;
    }
    let holidays: Vec<i64> = o
        .field(2)
        .split([',', ';'])
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .map(|h| {
            parse_datetime(h)
                .map(|s| (s / 86400.0).floor() as i64)
                .ok_or_else(|| format!("“{h}” is not a date."))
        })
        .collect::<Result<_, _>>()?;
    let work = business_days_between(a, b, &holidays, o.check(1));
    let all = (b - a).abs();
    Ok(table(&[
        ("Business days", fmt(work as f64, 0)),
        ("Calendar days", fmt(all as f64, 0)),
        ("Weekend and holiday days", fmt((all - work) as f64, 0)),
    ]))
}

/// ISO week number and its year.
fn iso_week(days: i64) -> (i64, i64) {
    let thursday = days - weekday(days) as i64 + 3;
    let (y, ..) = civil_from_days(thursday);
    (y, (thursday - days_from_civil(y, 1, 1)) / 7 + 1)
}

fn day_of_week(_: &str, o: &mut Opts) -> Result<Output, String> {
    let d = o.date(0)?;
    let (y, m, _) = civil_from_days(d);
    let (wy, w) = iso_week(d);
    Ok(table(&[
        ("Day", WEEKDAYS[weekday(d)].into()),
        ("Date", format_unix(d as f64 * 86400.0)),
        ("ISO week", format!("{wy}-W{w:02}")),
        (
            "Day of the year",
            format!(
                "{} of {}",
                d - days_from_civil(y, 1, 1) + 1,
                if is_leap(y) { 366 } else { 365 }
            ),
        ),
        ("Quarter", format!("Q{}", (m - 1) / 3 + 1)),
        ("Leap year", if is_leap(y) { "yes" } else { "no" }.into()),
        ("Days in the month", month_days(y, m).to_string()),
    ]))
}

/// `strftime`-like formatting: %Y %m %d %H %M %S %B %b %A %a %j %U %e %y %p %I.
fn strftime(secs: f64, pattern: &str) -> String {
    let days = (secs / 86400.0).floor() as i64;
    let (y, m, d) = civil_from_days(days);
    let t = (secs - days as f64 * 86400.0) as i64;
    let (h, mi, s) = (t / 3600, t % 3600 / 60, t % 60);
    let mut out = String::new();
    let mut it = pattern.chars();
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let part = match it.next() {
            Some('Y') => y.to_string(),
            Some('y') => format!("{:02}", y % 100),
            Some('m') => format!("{m:02}"),
            Some('d') => format!("{d:02}"),
            Some('e') => d.to_string(),
            Some('H') => format!("{h:02}"),
            Some('I') => format!("{:02}", if h % 12 == 0 { 12 } else { h % 12 }),
            Some('p') => if h < 12 { "AM" } else { "PM" }.into(),
            Some('M') => format!("{mi:02}"),
            Some('S') => format!("{s:02}"),
            Some('B') => MONTHS[m as usize - 1].into(),
            Some('b') => MONTHS[m as usize - 1][..3].into(),
            Some('A') => WEEKDAYS[weekday(days)].into(),
            Some('a') => WEEKDAYS[weekday(days)][..3].into(),
            Some('j') => format!("{:03}", days - days_from_civil(y, 1, 1) + 1),
            Some('V') => format!("{:02}", iso_week(days).1),
            Some('s') => (secs as i64).to_string(),
            Some('%') => "%".into(),
            Some(o) => format!("%{o}"),
            None => "%".into(),
        };
        out.push_str(&part);
    }
    out
}

fn date_format(_: &str, o: &mut Opts) -> Result<Output, String> {
    let t = o.datetime(0)?;
    let mut rows = vec![
        ("ISO 8601", strftime(t, "%Y-%m-%dT%H:%M:%SZ")),
        ("RFC 2822", strftime(t, "%a, %d %b %Y %H:%M:%S +0000")),
        ("US", strftime(t, "%m/%d/%Y %I:%M %p")),
        ("European", strftime(t, "%d/%m/%Y %H:%M")),
        ("German", strftime(t, "%d.%m.%Y %H:%M")),
        ("Long", strftime(t, "%A, %B %e, %Y")),
        ("Short", strftime(t, "%b %e, %Y")),
        ("Unix seconds", fmt(t.floor(), 0).replace(',', "")),
        ("Unix milliseconds", format!("{}", (t * 1000.0) as i64)),
        ("ISO week date", strftime(t, "%Y-W%V")),
        ("Ordinal date", strftime(t, "%Y-%j")),
    ];
    if !o.second.is_empty() {
        rows.insert(0, ("Your pattern", strftime(t, o.second)));
    }
    Ok(table(&rows))
}

fn calendar(_: &str, o: &mut Opts) -> Result<Output, String> {
    let today = (o.now()? / 86400000.0).floor() as i64;
    let (ty, tm, _) = civil_from_days(today);
    let y = o.opt_num(0)?.map_or(ty, |v| v as i64);
    let months: Vec<i64> = match o.mode {
        "year" => (1..=12).collect(),
        _ => vec![o.opt_num(1)?.map_or(tm, |v| v as i64)],
    };
    if months.iter().any(|m| !(1..=12).contains(m)) {
        return Err("A month goes from 1 to 12.".into());
    }
    let sunday = o.flag;
    let mut out = Vec::new();
    for m in months {
        let title = format!("{} {y}", MONTHS[m as usize - 1]);
        out.push(format!("{title:^27}"));
        out.push(
            if sunday {
                "Su  Mo  Tu  We  Th  Fr  Sa"
            } else {
                "Mo  Tu  We  Th  Fr  Sa  Su"
            }
            .trim_end()
            .to_owned(),
        );
        let first = days_from_civil(y, m, 1);
        let offset = if sunday {
            (weekday(first) + 1) % 7
        } else {
            weekday(first)
        };
        let mut line = "    ".repeat(offset);
        for d in 1..=month_days(y, m) {
            let mark = if first + d - 1 == today { '*' } else { ' ' };
            line.push_str(&format!("{d:>2}{mark} "));
            if (offset + d as usize) % 7 == 0 {
                out.push(line.trim_end().to_owned());
                line.clear();
            }
        }
        if !line.is_empty() {
            out.push(line.trim_end().to_owned());
        }
        out.push(String::new());
    }
    Ok(out.join("\n").trim_end().to_owned().into())
}

fn countdown(_: &str, o: &mut Opts) -> Result<Output, String> {
    let target = o.datetime(0)?;
    let now = o.now()? / 1000.0;
    let left = target - now;
    let days = left.abs() / 86400.0;
    Ok(table(&[
        (
            if left >= 0.0 {
                "Time left"
            } else {
                "Time since"
            },
            duration_words(left),
        ),
        ("Days", fmt(days, 2)),
        ("Hours", fmt(left.abs() / 3600.0, 1)),
        ("Weeks", fmt(days / 7.0, 1)),
        (
            "Weekdays",
            fmt(
                business_days_between(
                    (now / 86400.0).floor() as i64,
                    (target / 86400.0).floor() as i64,
                    &[],
                    false,
                ) as f64,
                0,
            ),
        ),
        ("Target", format_unix(target)),
    ]))
}

fn unix(_: &str, o: &mut Opts) -> Result<Output, String> {
    let raw = o.field(0);
    let now = o.now()? / 1000.0;
    if let Ok(n) = raw.parse::<f64>() {
        let secs = match o.mode {
            "ms" => n / 1000.0,
            "us" => n / 1e6,
            "s" => n,
            // Guess the unit from the size.
            _ if n.abs() > 1e14 => n / 1e6,
            _ if n.abs() > 1e11 => n / 1000.0,
            _ => n,
        };
        return Ok(table(&[
            ("UTC", format_unix(secs)),
            ("ISO 8601", strftime(secs, "%Y-%m-%dT%H:%M:%SZ")),
            (
                "Relative",
                format!(
                    "{} {}",
                    duration_words(secs - now),
                    if secs >= now { "from now" } else { "ago" }
                ),
            ),
        ]));
    }
    let secs = o.datetime(0)?;
    Ok(table(&[
        ("Seconds", format!("{}", secs as i64)),
        ("Milliseconds", format!("{}", (secs * 1000.0) as i64)),
        ("UTC", format_unix(secs)),
    ]))
}

const ANIMALS: [&str; 12] = [
    "Rat", "Ox", "Tiger", "Rabbit", "Dragon", "Snake", "Horse", "Goat", "Monkey", "Rooster", "Dog",
    "Pig",
];
const ELEMENTS: [&str; 5] = ["Wood", "Fire", "Earth", "Metal", "Water"];

fn chinese_zodiac(_: &str, o: &mut Opts) -> Result<Output, String> {
    let y = o.int(0)?;
    let i = (y - 4).rem_euclid(60);
    Ok(table(&[
        ("Animal", ANIMALS[(i % 12) as usize].into()),
        ("Element", ELEMENTS[(i % 10 / 2) as usize].into()),
        ("Polarity", if i % 2 == 0 { "Yang" } else { "Yin" }.into()),
        ("Sign", format!("{} {}", ELEMENTS[(i % 10 / 2) as usize], ANIMALS[(i % 12) as usize])),
        ("", "The Chinese year starts in late January or February, so January dates belong to the year before.".into()),
    ]))
}

pub fn zodiac_sign(m: i64, d: i64) -> &'static str {
    const SIGNS: [(&str, i64, i64); 12] = [
        ("Capricorn ♑", 1, 19),
        ("Aquarius ♒", 2, 18),
        ("Pisces ♓", 3, 20),
        ("Aries ♈", 4, 19),
        ("Taurus ♉", 5, 20),
        ("Gemini ♊", 6, 20),
        ("Cancer ♋", 7, 22),
        ("Leo ♌", 8, 22),
        ("Virgo ♍", 9, 22),
        ("Libra ♎", 10, 22),
        ("Scorpio ♏", 11, 21),
        ("Sagittarius ♐", 12, 21),
    ];
    let (name, _, last) = SIGNS[m as usize - 1];
    if d <= last {
        name
    } else {
        SIGNS[m as usize % 12].0
    }
}

fn zodiac(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (_, m, d) = civil_from_days(o.date(0)?);
    Ok(zodiac_sign(m, d).into())
}

/// The Julian calendar date of a day since 1970.
fn julian_from_days(days: i64) -> (i64, i64, i64) {
    let jdn = days + 2440588;
    let c = jdn + 32082;
    let d = (4 * c + 3) / 1461;
    let e = c - 1461 * d / 4;
    let m = (5 * e + 2) / 153;
    (
        d - 4800 + (m + 3) / 10 - 1 + 1,
        m + 3 - 12 * (m / 10),
        e - (153 * m + 2) / 5 + 1,
    )
}

fn julian(_: &str, o: &mut Opts) -> Result<Output, String> {
    let t = o.datetime(0)?;
    let jd = t / 86400.0 + 2440587.5;
    let (y, m, d) = julian_from_days((t / 86400.0).floor() as i64);
    Ok(table(&[
        ("Julian day", fmt(jd, 5)),
        ("Julian day number", fmt((jd + 0.5).floor(), 0)),
        ("Modified Julian day", fmt(jd - 2400000.5, 5)),
        (
            "Julian calendar date",
            format!("{d} {} {}", MONTHS[m as usize - 1], y - 1),
        ),
        ("Days since 1970", fmt((t / 86400.0).floor(), 0)),
    ]))
}

// The tabular Islamic calendar, in days since 1970.

const ISLAMIC_EPOCH: i64 = 227015 - 719163;
const ISLAMIC_MONTHS: [&str; 12] = [
    "Muharram",
    "Safar",
    "Rabi al-Awwal",
    "Rabi al-Thani",
    "Jumada al-Awwal",
    "Jumada al-Thani",
    "Rajab",
    "Sha'ban",
    "Ramadan",
    "Shawwal",
    "Dhu al-Qadah",
    "Dhu al-Hijjah",
];

fn from_islamic(y: i64, m: i64, d: i64) -> i64 {
    ISLAMIC_EPOCH - 1 + (y - 1) * 354 + (3 + 11 * y).div_euclid(30) + 29 * (m - 1) + m / 2 + d
}

fn to_islamic(days: i64) -> (i64, i64, i64) {
    let y = (30 * (days - ISLAMIC_EPOCH) + 10646).div_euclid(10631);
    let prior = days - from_islamic(y, 1, 1);
    let m = (11 * prior + 330).div_euclid(325);
    (y, m, days - from_islamic(y, m, 1) + 1)
}

fn islamic(_: &str, o: &mut Opts) -> Result<Output, String> {
    if o.mode == "to-gregorian" {
        let (y, m, d) = (o.int(1)?, o.int(2)?, o.int(3)?);
        if !(1..=12).contains(&m) || !(1..=30).contains(&d) {
            return Err("An Islamic month goes from 1 to 12, a day from 1 to 30.".into());
        }
        return Ok(table(&[(
            "Gregorian date",
            format_unix(from_islamic(y, m, d) as f64 * 86400.0),
        )]));
    }
    let (y, m, d) = to_islamic(o.date(0)?);
    Ok(table(&[(
        "Islamic date",
        format!("{d} {} {y} AH", ISLAMIC_MONTHS[m as usize - 1]),
    )])
    .note("The tabular calendar; the observed one may differ by a day or two."))
}

// The Hebrew calendar, after Reingold and Dershowitz, in days since 1970.

const HEBREW_EPOCH: i64 = -1373427 - 719163;
const HEBREW_MONTHS: [&str; 13] = [
    "Nisan", "Iyyar", "Sivan", "Tammuz", "Av", "Elul", "Tishrei", "Cheshvan", "Kislev", "Tevet",
    "Shevat", "Adar", "Adar II",
];

fn hebrew_leap(y: i64) -> bool {
    (7 * y + 1).rem_euclid(19) < 7
}

fn hebrew_elapsed(y: i64) -> i64 {
    let months = (235 * y - 234).div_euclid(19);
    let parts = 12084 + 13753 * months;
    let day = months * 29 + parts.div_euclid(25920);
    if (3 * (day + 1)).rem_euclid(7) < 3 {
        day + 1
    } else {
        day
    }
}

fn hebrew_new_year(y: i64) -> i64 {
    let (a, b, c) = (
        hebrew_elapsed(y - 1),
        hebrew_elapsed(y),
        hebrew_elapsed(y + 1),
    );
    let delay = if c - b == 356 {
        2
    } else if b - a == 382 {
        1
    } else {
        0
    };
    HEBREW_EPOCH + b + delay
}

fn hebrew_month_days(y: i64, m: i64) -> i64 {
    let year_days = hebrew_new_year(y + 1) - hebrew_new_year(y);
    match m {
        2 | 4 | 6 | 10 | 13 => 29,
        12 if !hebrew_leap(y) => 29,
        8 if year_days % 10 != 5 => 29,
        9 if year_days % 10 == 3 => 29,
        _ => 30,
    }
}

fn from_hebrew(y: i64, m: i64, d: i64) -> i64 {
    let last = if hebrew_leap(y) { 13 } else { 12 };
    let mut days = hebrew_new_year(y) + d - 1;
    if m < 7 {
        days += (7..=last).map(|mm| hebrew_month_days(y, mm)).sum::<i64>();
        days += (1..m).map(|mm| hebrew_month_days(y, mm)).sum::<i64>();
    } else {
        days += (7..m).map(|mm| hebrew_month_days(y, mm)).sum::<i64>();
    }
    days
}

fn to_hebrew(days: i64) -> (i64, i64, i64) {
    let approx = ((days - HEBREW_EPOCH) as f64 / (35975351.0 / 98496.0)).floor() as i64 + 1;
    let mut y = approx - 1;
    while hebrew_new_year(y + 1) <= days {
        y += 1;
    }
    let mut m = if days < from_hebrew(y, 1, 1) { 7 } else { 1 };
    while days > from_hebrew(y, m, hebrew_month_days(y, m)) {
        m += 1;
    }
    (y, m, days - from_hebrew(y, m, 1) + 1)
}

fn hebrew(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (y, m, d) = to_hebrew(o.date(0)?);
    let name = if m == 12 && hebrew_leap(y) {
        "Adar I"
    } else {
        HEBREW_MONTHS[m as usize - 1]
    };
    Ok(table(&[("Hebrew date", format!("{d} {name} {y}"))])
        .note("Hebrew days begin at sunset the evening before."))
}

/// The moon's age in days, and the fraction lit, at a time.
pub fn moon(secs: f64) -> (f64, f64) {
    const SYNODIC: f64 = 29.530588853;
    // A new moon: 2000-01-06 18:14 UTC.
    let age = ((secs - 947182440.0) / 86400.0).rem_euclid(SYNODIC);
    let lit = (1.0 - (2.0 * std::f64::consts::PI * age / SYNODIC).cos()) / 2.0;
    (age, lit)
}

fn moon_phase(_: &str, o: &mut Opts) -> Result<Output, String> {
    let t = o.datetime(0)?;
    let (age, lit) = moon(t);
    let names = [
        "New moon 🌑",
        "Waxing crescent 🌒",
        "First quarter 🌓",
        "Waxing gibbous 🌔",
        "Full moon 🌕",
        "Waning gibbous 🌖",
        "Last quarter 🌗",
        "Waning crescent 🌘",
    ];
    let phase = names[((age / 29.530588853 * 8.0 + 0.5).floor() as usize) % 8];
    let next = |target: f64| {
        let wait = (target - age).rem_euclid(29.530588853);
        format_unix(((t + wait * 86400.0) / 60.0).round() * 60.0)
    };
    Ok(table(&[
        ("Phase", phase.into()),
        ("Illuminated", format!("{:.0}%", lit * 100.0)),
        ("Age", format!("{age:.1} days")),
        ("Next full moon", next(14.765)),
        ("Next new moon", next(0.0)),
    ]))
}

/// Sunrise and sunset in hours UTC, and noon, by the NOAA formulas; `None`
/// for polar day or night. `depression` is the sun's angle below the horizon
/// that counts (0.833 for sunrise).
pub fn sun_times(days: i64, lat: f64, lon: f64, depression: f64) -> (Option<(f64, f64)>, f64) {
    let jd = days as f64 + 2440587.5 + 0.5;
    let n = jd - 2451545.0;
    let mean_noon = n - lon / 360.0;
    let m = (357.5291 + 0.98560028 * mean_noon)
        .rem_euclid(360.0)
        .to_radians();
    let c = 1.9148 * m.sin() + 0.02 * (2.0 * m).sin() + 0.0003 * (3.0 * m).sin();
    let lambda = (m.to_degrees() + c + 180.0 + 102.9372)
        .rem_euclid(360.0)
        .to_radians();
    let transit = 2451545.0 + mean_noon + 0.0053 * m.sin() - 0.0069 * (2.0 * lambda).sin();
    let decl = (lambda.sin() * 23.4397f64.to_radians().sin()).asin();
    let cos_h = ((-depression).to_radians().sin() - lat.to_radians().sin() * decl.sin())
        / (lat.to_radians().cos() * decl.cos());
    let noon = (transit - 2440587.5 - days as f64) * 24.0;
    if !(-1.0..=1.0).contains(&cos_h) {
        return (None, noon);
    }
    let h = cos_h.acos().to_degrees() / 360.0 * 24.0;
    (Some((noon - h, noon + h)), noon)
}

pub fn clock(hours: f64) -> String {
    let m = (hours * 60.0).round().rem_euclid(1440.0) as i64;
    format!("{:02}:{:02}", m / 60, m % 60)
}

fn sunrise(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (lat, lon) = (o.num(0)?, o.num(1)?);
    if lat.abs() > 90.0 || lon.abs() > 180.0 {
        return Err("Latitude goes from -90 to 90, longitude from -180 to 180.".into());
    }
    let day = o.date(2)?;
    let tz = o.opt_num(3)?.unwrap_or(0.0);
    let at = |h: f64| clock(h + tz);
    let (sun, noon) = sun_times(day, lat, lon, 0.833);
    let mut rows = vec![("Date", iso(day))];
    match sun {
        Some((rise, set)) => {
            rows.push(("Sunrise", at(rise)));
            rows.push(("Sunset", at(set)));
            rows.push(("Day length", duration_words((set - rise) * 3600.0)));
        }
        None => rows.push((
            "Sun",
            if lat
                * (if (80..270).contains(&(day.rem_euclid(365))) {
                    1.0
                } else {
                    -1.0
                })
                > 0.0
            {
                "up all day"
            } else {
                "down all day"
            }
            .into(),
        )),
    }
    rows.push(("Solar noon", at(noon)));
    if let (Some((dawn, dusk)), _) = sun_times(day, lat, lon, 6.0) {
        rows.push(("Civil dawn", at(dawn)));
        rows.push(("Civil dusk", at(dusk)));
    }
    if let (Some((a, b)), _) = sun_times(day, lat, lon, -6.0) {
        rows.push((
            "Morning golden hour",
            format!("{} – {}", at(sun.map_or(a, |s| s.0)), at(a)),
        ));
        rows.push((
            "Evening golden hour",
            format!("{} – {}", at(b), at(sun.map_or(b, |s| s.1))),
        ));
    }
    rows.push(("", format!("Times at UTC{:+}.", tz)));
    Ok(table(&rows))
}

fn ics(_: &str, o: &mut Opts) -> Result<Output, String> {
    let start = o.datetime(1)?;
    let end = match o.field(2) {
        "" => start + 3600.0,
        _ => o.datetime(2)?,
    };
    if end < start {
        return Err("The end is before the start.".into());
    }
    let stamp = |t: f64| strftime(t, "%Y%m%dT%H%M%SZ");
    let esc = |s: &str| {
        s.replace('\\', "\\\\")
            .replace(';', "\\;")
            .replace(',', "\\,")
            .replace('\n', "\\n")
    };
    let uid = format!("{}-{}@hyperb", stamp(start), o.rng.next() % 1_000_000);
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_owned(),
        "VERSION:2.0".into(),
        "PRODID:-//hyperb//Text Utilities//EN".into(),
        "BEGIN:VEVENT".into(),
        format!("UID:{uid}"),
        format!("DTSTAMP:{}", stamp(o.now()? / 1000.0)),
        format!("DTSTART:{}", stamp(start)),
        format!("DTEND:{}", stamp(end)),
        format!("SUMMARY:{}", esc(o.field(0))),
    ];
    if !o.field(3).is_empty() {
        lines.push(format!("LOCATION:{}", esc(o.field(3))));
    }
    if !o.field(4).is_empty() {
        lines.push(format!("DESCRIPTION:{}", esc(o.field(4))));
    }
    if o.mode != "none" {
        lines.push(format!("RRULE:FREQ={}", o.mode.to_uppercase()));
    }
    lines.extend(["END:VEVENT".into(), "END:VCALENDAR".into()]);
    Ok(lines.join("\n").into())
}

fn ics_view(text: &str, _: &mut Opts) -> Result<Output, String> {
    // Unfold continuation lines first.
    let unfolded = text
        .replace("\r\n", "\n")
        .replace("\n ", "")
        .replace("\n\t", "");
    let mut events = Vec::new();
    let mut cur: Vec<(String, String)> = Vec::new();
    let mut inside = false;
    for line in unfolded.lines() {
        match line.trim() {
            "BEGIN:VEVENT" => {
                inside = true;
                cur.clear();
            }
            "END:VEVENT" => {
                inside = false;
                events.push(std::mem::take(&mut cur));
            }
            l if inside => {
                if let Some((k, v)) = l.split_once(':') {
                    let k = k.split(';').next().unwrap_or(k).to_owned();
                    cur.push((
                        k,
                        v.replace("\\n", "\n")
                            .replace("\\,", ",")
                            .replace("\\;", ";"),
                    ));
                }
            }
            _ => {}
        }
    }
    if events.is_empty() {
        return Err("There are no events (BEGIN:VEVENT … END:VEVENT).".into());
    }
    let when = |v: &str| {
        let v = v.trim();
        let (d, t) = v.split_once('T').unwrap_or((v, ""));
        if d.len() != 8 {
            return v.to_owned();
        }
        let date = format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..]);
        if t.len() >= 4 {
            format!(
                "{date} {}:{}{}",
                &t[..2],
                &t[2..4],
                if t.ends_with('Z') { " UTC" } else { "" }
            )
        } else {
            date
        }
    };
    events.sort_by_key(|e| {
        e.iter()
            .find(|(k, _)| k == "DTSTART")
            .map(|(_, v)| v.clone())
    });
    let rows = events.iter().map(|e| {
        let get = |k: &str| {
            e.iter()
                .find(|(kk, _)| kk == k)
                .map(|(_, v)| v.as_str())
                .unwrap_or("")
        };
        vec![
            if get("SUMMARY").is_empty() {
                "(no title)".to_owned()
            } else {
                get("SUMMARY").to_owned()
            },
            format!("{} → {}", when(get("DTSTART")), when(get("DTEND"))),
            get("LOCATION").to_owned(),
            get("RRULE").to_owned(),
            get("DESCRIPTION").to_owned(),
        ]
    });
    Ok(grid(&["Event", "When", "Where", "Repeats", "Notes"], rows))
}

/// A duration such as `1:30`, `1:30:15`, `2h 15m`, `90m` or `45s`, in seconds.
fn parse_duration(s: &str) -> Option<f64> {
    let s = s.trim();
    if s.contains(':') {
        let neg = s.starts_with('-');
        let parts: Vec<f64> = s
            .trim_start_matches('-')
            .split(':')
            .map(|p| p.trim().parse().ok())
            .collect::<Option<_>>()?;
        let v = match parts.as_slice() {
            [h, m] => h * 3600.0 + m * 60.0,
            [h, m, sec] => h * 3600.0 + m * 60.0 + sec,
            _ => return None,
        };
        return Some(if neg { -v } else { v });
    }
    let re = regex_lite::Regex::new(
        r"(?i)(-?[\d.]+)\s*(d|h|m|s|days?|hours?|hrs?|min(?:ute)?s?|sec(?:ond)?s?)",
    )
    .unwrap();
    let mut total = 0.0;
    let mut any = false;
    for c in re.captures_iter(s) {
        let n: f64 = c[1].parse().ok()?;
        let unit = c[2].to_lowercase();
        total += n * match unit.chars().next()? {
            'd' => 86400.0,
            'h' => 3600.0,
            's' => 1.0,
            _ => 60.0,
        };
        any = true;
    }
    any.then_some(total)
}

fn time_calc(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut total = 0.0;
    for l in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (sign, l) = match l.strip_prefix('-') {
            Some(r) if !r.contains(':') || o.mode == "sum" => (-1.0, r),
            _ => (1.0, l.trim_start_matches('+')),
        };
        total += sign
            * parse_duration(l)
                .ok_or_else(|| format!("“{l}” is not a duration such as 1:30 or 2h 15m."))?;
    }
    let t = total.abs().round() as i64;
    let sign = if total < 0.0 { "-" } else { "" };
    Ok(table(&[
        (
            "Total",
            format!("{sign}{}:{:02}:{:02}", t / 3600, t % 3600 / 60, t % 60),
        ),
        ("In words", duration_words(total)),
        ("Hours", fmt(total / 3600.0, 4)),
        ("Minutes", fmt(total / 60.0, 2)),
        ("Seconds", fmt(total, 0)),
    ]))
}

pub const DATES: &[Tool] = &[
    tool(
        "age",
        "Age calculator",
        "Age in years, months and days, and the next birthday.",
        age,
    )
    .form()
    .fields(&[("Birth date", "1990-05-15"), ("On (empty: today)", "")]),
    tool(
        "date-diff",
        "Date difference",
        "The time between two dates in days, weeks, months and years.",
        date_diff,
    )
    .form()
    .fields(&[("From", "2024-01-01"), ("To (empty: today)", "")])
    .checks(&["Count the end date too"]),
    tool(
        "date-add",
        "Date calculator",
        "Adds days, weeks, months, years or business days to a date.",
        date_add,
    )
    .form()
    .fields(&[("Date (empty: today)", ""), ("How many", "90")])
    .selects(&[
        (
            "Unit",
            &[
                ("days", "Days"),
                ("weeks", "Weeks"),
                ("months", "Months"),
                ("years", "Years"),
                ("business", "Business days"),
            ],
        ),
        ("Operation", &[("add", "Add"), ("subtract", "Subtract")]),
    ]),
    tool(
        "business-days",
        "Business days",
        "Working days between two dates, without weekends and holidays.",
        business_days,
    )
    .form()
    .fields(&[
        ("From", "2024-12-01"),
        ("To", "2024-12-31"),
        (
            "Holidays (dates, comma separated)",
            "2024-12-25, 2024-12-26",
        ),
    ])
    .checks(&["Count the end date too", "Saturdays are working days"]),
    tool(
        "day-of-week",
        "Day of the week",
        "The weekday, ISO week, day of the year and quarter of a date.",
        day_of_week,
    )
    .form()
    .fields(&[("Date (empty: today)", "")]),
    tool(
        "date-format",
        "Date formatter",
        "A date in ISO, RFC 2822, US, European and other formats, or your own %Y-%m-%d pattern.",
        date_format,
    )
    .form()
    .fields(&[
        ("Date and time (empty: now)", ""),
        ("Pattern (%Y %m %d %H %M %S %B %A…)", "%A %e %B %Y"),
    ]),
    tool(
        "calendar",
        "Calendar",
        "A month or a year as a calendar; today is marked with *.",
        calendar,
    )
    .form()
    .fields(&[
        ("Year (empty: this one)", ""),
        ("Month (empty: this one)", ""),
    ])
    .selects(&[("Show", &[("month", "A month"), ("year", "The whole year")])])
    .checks(&["Weeks start on Sunday"]),
    tool(
        "countdown",
        "Event countdown",
        "The time left until an event, or since.",
        countdown,
    )
    .form()
    .fields(&[("Event date and time", "2030-01-01 00:00")]),
    tool(
        "unix-time",
        "Unix time converter",
        "Unix timestamps to dates, and dates to timestamps.",
        unix,
    )
    .form()
    .fields(&[("Timestamp or date (empty: now)", "1700000000")])
    .selects(&[(
        "Timestamp unit",
        &[
            ("auto", "Guess"),
            ("s", "Seconds"),
            ("ms", "Milliseconds"),
            ("us", "Microseconds"),
        ],
    )]),
    tool(
        "time-calc",
        "Time calculator",
        "Adds and subtracts durations such as 1:30, 2h 15m or -45m.",
        time_calc,
    )
    .text("One duration per line (a leading - subtracts)")
    .selects(&[(
        "Lines starting with -",
        &[("subtract", "Subtract"), ("sum", "Subtract too")],
    )]),
    tool(
        "chinese-zodiac",
        "Chinese zodiac",
        "The animal, element and polarity of a year.",
        chinese_zodiac,
    )
    .form()
    .fields(&[("Year", "2024")]),
    tool(
        "zodiac",
        "Zodiac sign",
        "The western zodiac sign of a birth date.",
        zodiac,
    )
    .form()
    .fields(&[("Birth date", "1990-05-15")]),
    tool(
        "julian",
        "Julian date",
        "The Julian day number, modified Julian day and Julian calendar date.",
        julian,
    )
    .form()
    .fields(&[("Date and time (empty: now)", "")]),
    tool(
        "islamic",
        "Islamic calendar",
        "Gregorian dates in the tabular Hijri calendar, and back.",
        islamic,
    )
    .form()
    .fields(&[
        ("Gregorian date (empty: today)", ""),
        ("Hijri year", "1446"),
        ("Hijri month", "9"),
        ("Hijri day", "1"),
    ])
    .selects(&[(
        "Convert",
        &[
            ("to-islamic", "Gregorian to Hijri"),
            ("to-gregorian", "Hijri to Gregorian"),
        ],
    )]),
    tool(
        "hebrew",
        "Hebrew calendar",
        "The Hebrew date of a Gregorian date.",
        hebrew,
    )
    .form()
    .fields(&[("Date (empty: today)", "")]),
    tool(
        "moon",
        "Moon phase",
        "The moon's phase, how much is lit, and the next full and new moons.",
        moon_phase,
    )
    .form()
    .fields(&[("Date (empty: now)", "")]),
    tool(
        "sunrise",
        "Sunrise and sunset",
        "Sunrise, sunset, day length, twilight and golden hours anywhere.",
        sunrise,
    )
    .form()
    .fields(&[
        ("Latitude", "48.8566"),
        ("Longitude", "2.3522"),
        ("Date (empty: today)", ""),
        ("UTC offset (hours)", "1"),
    ]),
    tool(
        "ics",
        "ICS file generator",
        "A calendar event (.ics) to import in any calendar.",
        ics,
    )
    .form()
    .fields(&[
        ("Title", "Team meeting"),
        ("Start (UTC)", "2030-01-15 09:00"),
        ("End (UTC, empty: an hour later)", ""),
        ("Location", "Room 4"),
        ("Description", ""),
    ])
    .selects(&[(
        "Repeats",
        &[
            ("none", "Never"),
            ("daily", "Daily"),
            ("weekly", "Weekly"),
            ("monthly", "Monthly"),
            ("yearly", "Yearly"),
        ],
    )]),
    tool(
        "ics-view",
        "ICS viewer",
        "The events of an .ics calendar file, in order.",
        ics_view,
    )
    .text("ICS file"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(days_from_civil(2024, 2, 29)), (2024, 2, 29));
        assert_eq!(weekday(days_from_civil(2024, 3, 5)), 1);
        assert_eq!(iso_week(days_from_civil(2021, 1, 3)), (2020, 53));
    }

    #[test]
    fn parsing() {
        let d = |s| parse_datetime(s).map(|t| iso((t / 86400.0).floor() as i64));
        assert_eq!(d("2024-03-05").as_deref(), Some("2024-03-05"));
        assert_eq!(d("3/5/2024").as_deref(), Some("2024-03-05"));
        assert_eq!(d("5.3.2024").as_deref(), Some("2024-03-05"));
        assert_eq!(d("March 5th, 2024").as_deref(), Some("2024-03-05"));
        assert_eq!(d("5 Mar 2024").as_deref(), Some("2024-03-05"));
        assert_eq!(d("2024-02-30"), None);
        assert_eq!(parse_datetime("1970-01-02 01:00+01:00"), Some(86400.0));
        assert_eq!(parse_datetime("1970-01-01T12:30:00Z"), Some(45000.0));
        assert_eq!(parse_datetime("1700000000"), Some(1.7e9));
    }

    #[test]
    fn calendars() {
        let day = days_from_civil(2024, 10, 3);
        assert_eq!(to_hebrew(day), (5785, 7, 1));
        assert_eq!(to_hebrew(days_from_civil(2024, 4, 23)), (5784, 1, 15));
        let (y, m, _) = to_islamic(days_from_civil(2024, 3, 12));
        assert_eq!((y, m), (1445, 9));
        assert_eq!(julian_from_days(days_from_civil(2024, 3, 14)), (2024, 3, 1));
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration("1:30"), Some(5400.0));
        assert_eq!(parse_duration("2h 15m"), Some(8100.0));
        assert_eq!(duration_words(90061.0), "1 day, 1 hour");
        let (rise, set) = sun_times(days_from_civil(2024, 6, 21), 51.5, 0.0, 0.833)
            .0
            .unwrap();
        assert_eq!(
            (clock(rise + 1.0), clock(set + 1.0)),
            ("04:43".to_owned(), "21:21".to_owned())
        );
    }
}
