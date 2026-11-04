//! The world clock demo: live clocks in time zones, and a converter.
//!
//! The module has no time zone database, so the zones carry their own rules:
//! a standard offset and, for the ones that keep summer time, one of the few
//! daylight saving rules in use. The clocks run on the host's wall clock (the
//! tick's Unix milliseconds) and are patched once a second. The converter
//! takes a wall time in one zone, today or a day either side, and shows that
//! moment on every clock. The home zone and the clocks are kept in the
//! address (`#home=…&clocks=…`), so a reload opens the same ones.

use super::common::{replace, text};
use hypereffect::{Action, App, Element, Handler, Map, Metadata, Outbox, Value, cel, json, ui};
use std::collections::BTreeMap;

/// Often enough that a new second shows without visible lag.
const TICK_MS: u32 = 250;
/// The clocks of a fresh session, besides the home zone.
const DEFAULT_CLOCKS: &[&str] = &[
    "America/Los_Angeles",
    "America/New_York",
    "Europe/London",
    "Europe/Berlin",
    "Asia/Kolkata",
    "Asia/Tokyo",
    "Australia/Sydney",
];
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// A daylight saving rule: summer time is one hour ahead of standard time.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Dst {
    None,
    /// The United States and Canada.
    Us,
    /// The European Union and the United Kingdom.
    Eu,
    /// South-eastern Australia.
    Au,
    /// New Zealand.
    Nz,
}

struct Zone {
    /// The IANA name, as kept in the address.
    id: &'static str,
    city: &'static str,
    /// The standard offset from UTC, in minutes.
    std: i64,
    dst: Dst,
}

const fn zone(id: &'static str, city: &'static str, std: i64, dst: Dst) -> Zone {
    Zone { id, city, std, dst }
}

/// The zones on offer, west to east by standard offset.
const ZONES: &[Zone] = &[
    zone("UTC", "UTC", 0, Dst::None),
    zone("Pacific/Honolulu", "Honolulu", -600, Dst::None),
    zone("America/Anchorage", "Anchorage", -540, Dst::Us),
    zone("America/Los_Angeles", "Los Angeles", -480, Dst::Us),
    zone("America/Denver", "Denver", -420, Dst::Us),
    // British Columbia has kept summer time all year since November 2026.
    zone("America/Vancouver", "Vancouver", -420, Dst::None),
    zone("America/Phoenix", "Phoenix", -420, Dst::None),
    zone("America/Chicago", "Chicago", -360, Dst::Us),
    zone("America/Mexico_City", "Mexico City", -360, Dst::None),
    zone("America/New_York", "New York", -300, Dst::Us),
    zone("America/Toronto", "Toronto", -300, Dst::Us),
    zone("America/Bogota", "Bogotá", -300, Dst::None),
    zone("America/Sao_Paulo", "São Paulo", -180, Dst::None),
    zone(
        "America/Argentina/Buenos_Aires",
        "Buenos Aires",
        -180,
        Dst::None,
    ),
    zone("Atlantic/Reykjavik", "Reykjavík", 0, Dst::None),
    zone("Europe/London", "London", 0, Dst::Eu),
    zone("Europe/Lisbon", "Lisbon", 0, Dst::Eu),
    zone("Africa/Lagos", "Lagos", 60, Dst::None),
    zone("Europe/Madrid", "Madrid", 60, Dst::Eu),
    zone("Europe/Paris", "Paris", 60, Dst::Eu),
    zone("Europe/Berlin", "Berlin", 60, Dst::Eu),
    zone("Europe/Rome", "Rome", 60, Dst::Eu),
    zone("Africa/Johannesburg", "Johannesburg", 120, Dst::None),
    zone("Europe/Athens", "Athens", 120, Dst::Eu),
    zone("Europe/Helsinki", "Helsinki", 120, Dst::Eu),
    zone("Europe/Istanbul", "Istanbul", 180, Dst::None),
    zone("Europe/Moscow", "Moscow", 180, Dst::None),
    zone("Africa/Nairobi", "Nairobi", 180, Dst::None),
    zone("Asia/Dubai", "Dubai", 240, Dst::None),
    zone("Asia/Karachi", "Karachi", 300, Dst::None),
    zone("Asia/Kolkata", "Kolkata", 330, Dst::None),
    zone("Asia/Kathmandu", "Kathmandu", 345, Dst::None),
    zone("Asia/Dhaka", "Dhaka", 360, Dst::None),
    zone("Asia/Bangkok", "Bangkok", 420, Dst::None),
    zone("Asia/Jakarta", "Jakarta", 420, Dst::None),
    zone("Asia/Singapore", "Singapore", 480, Dst::None),
    zone("Asia/Shanghai", "Shanghai", 480, Dst::None),
    zone("Asia/Hong_Kong", "Hong Kong", 480, Dst::None),
    zone("Australia/Perth", "Perth", 480, Dst::None),
    zone("Asia/Seoul", "Seoul", 540, Dst::None),
    zone("Asia/Tokyo", "Tokyo", 540, Dst::None),
    zone("Australia/Adelaide", "Adelaide", 570, Dst::Au),
    zone("Australia/Brisbane", "Brisbane", 600, Dst::None),
    zone("Australia/Sydney", "Sydney", 600, Dst::Au),
    zone("Australia/Melbourne", "Melbourne", 600, Dst::Au),
    zone("Pacific/Auckland", "Auckland", 720, Dst::Nz),
];

fn find(id: &str) -> Option<usize> {
    ZONES.iter().position(|z| z.id == id)
}

/// Days since 1970-01-01 of a civil date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The civil date of a day since 1970-01-01, as (year, month, day).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// The weekday of a day since 1970-01-01, Sunday being 0.
fn weekday(days: i64) -> usize {
    (days + 4).rem_euclid(7) as usize
}

/// The day of the `n`th Sunday of a month, or of the last one for `n == 0`.
fn sunday(y: i64, m: u32, n: i64) -> i64 {
    if n == 0 {
        let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
        let last = days_from_civil(ny, nm, 1) - 1;
        last - weekday(last) as i64
    } else {
        let first = days_from_civil(y, m, 1);
        first + (7 - weekday(first) as i64) % 7 + 7 * (n - 1)
    }
}

impl Dst {
    /// When summer time starts and ends in `year`, in Unix minutes, for a
    /// zone `std` minutes ahead of UTC. In the south it starts late in the
    /// year and ends early in it.
    fn span(self, year: i64, std: i64) -> Option<(i64, i64)> {
        // A Sunday at a time of day in standard time.
        let at = |m, n, min| sunday(year, m, n) * 1440 + min - std;
        let utc = |m, n, min| sunday(year, m, n) * 1440 + min;
        Some(match self {
            Dst::None => return None,
            // 02:00 on the second Sunday of March to 02:00 summer time on
            // the first Sunday of November.
            Dst::Us => (at(3, 2, 120), at(11, 1, 60)),
            // 01:00 UTC on the last Sundays of March and October.
            Dst::Eu => (utc(3, 0, 60), utc(10, 0, 60)),
            // 02:00 on the first Sunday of October to 03:00 summer time on
            // the first Sunday of April.
            Dst::Au => (at(10, 1, 120), at(4, 1, 120)),
            // 02:00 on the last Sunday of September to 03:00 summer time on
            // the first Sunday of April.
            Dst::Nz => (at(9, 0, 120), at(4, 1, 120)),
        })
    }
}

impl Zone {
    /// The offset from UTC at a Unix minute, in minutes.
    fn offset(&self, minute: i64) -> i64 {
        let year = civil_from_days((minute + self.std).div_euclid(1440)).0;
        let summer = match self.dst.span(year, self.std) {
            None => false,
            Some((start, end)) if start < end => (start..end).contains(&minute),
            Some((start, end)) => minute >= start || minute < end,
        };
        self.std + if summer { 60 } else { 0 }
    }

    /// The Unix minute of a wall-clock minute here. A time skipped when
    /// summer time starts is read in standard time, and one repeated when it
    /// ends is the first of the two.
    fn minute_at_wall(&self, wall: i64) -> i64 {
        // The offsets in force a little before and after: the ones that
        // read back as this wall time are two in an overlap, none in a gap.
        let before = self.offset(wall - self.std - 120);
        let after = self.offset(wall - self.std + 120);
        let fits = [before, after]
            .into_iter()
            .find(|&o| self.offset(wall - o) == o);
        wall - fits.unwrap_or(before)
    }
}

/// An offset in minutes as `+5:30`, `-4` or `+0`.
fn hours(min: i64) -> String {
    let sign = if min < 0 { '-' } else { '+' };
    let (h, m) = (min.abs() / 60, min.abs() % 60);
    if m == 0 {
        format!("{sign}{h}")
    } else {
        format!("{sign}{h}:{m:02}")
    }
}

/// A time of day in seconds, as `14:05`, `2:05 pm` or with seconds.
fn clock(secs: i64, h12: bool, with_secs: bool) -> String {
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    let tail = if with_secs {
        format!(":{s:02}")
    } else {
        String::new()
    };
    if h12 {
        let hh = if h % 12 == 0 { 12 } else { h % 12 };
        let ampm = if h < 12 { "am" } else { "pm" };
        format!("{hh}:{m:02}{tail} {ampm}")
    } else {
        format!("{h:02}:{m:02}{tail}")
    }
}

/// A day since 1970-01-01 as `Tue 3 Nov`, with the year when `long`.
fn date(days: i64, long: bool) -> String {
    let (y, m, d) = civil_from_days(days);
    let base = format!("{} {d} {}", WEEKDAYS[weekday(days)], MONTHS[m as usize - 1]);
    if long { format!("{base} {y}") } else { base }
}

/// What a wall time means for calling someone, and its badge variant.
fn daypart(days: i64, secs: i64) -> (&'static str, &'static str) {
    let h = secs / 3600;
    let workday = (1..=5).contains(&weekday(days));
    match h {
        9..17 if workday => ("Working hours", "success"),
        7..22 if workday => ("Off hours", "warning"),
        7..22 => ("Weekend", "info"),
        _ => ("Night", "neutral"),
    }
}

/// A wall time typed by a person: `9`, `9:30`, `0930`, `9.30`, `9pm`,
/// `9:30 PM`. Minutes past midnight.
fn parse_time(s: &str) -> Option<i64> {
    let s = s.trim().to_lowercase();
    let (s, half) = if let Some(t) = s.strip_suffix("am") {
        (t.trim_end(), Some(false))
    } else if let Some(t) = s.strip_suffix("pm") {
        (t.trim_end(), Some(true))
    } else {
        (s.as_str(), None)
    };
    let (h, m) = match s.split_once([':', '.', 'h']) {
        Some((h, m)) if m.len() == 2 || m.is_empty() => (h, if m.is_empty() { "0" } else { m }),
        Some(_) => return None,
        None if s.len() == 4 => s.split_at(2),
        None if s.len() == 3 => s.split_at(1),
        None => (s, "0"),
    };
    let all_digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    if !all_digits(h) || !all_digits(m) || h.len() > 2 {
        return None;
    }
    let (mut h, m): (i64, i64) = (h.parse().ok()?, m.parse().ok()?);
    if m > 59 {
        return None;
    }
    match half {
        Some(pm) if (1..=12).contains(&h) => h = h % 12 + if pm { 12 } else { 0 },
        Some(_) => return None,
        None if h > 23 => return None,
        None => {}
    }
    Some(h * 60 + m)
}

#[derive(Default)]
pub struct WorldClock {
    /// Indexes into `ZONES`.
    home: usize,
    clocks: Vec<usize>,
    h12: bool,
    /// The converter's inputs: the typed time, its zone and the day, as
    /// days from today in that zone.
    conv_time: String,
    conv_from: usize,
    conv_day: i64,
    /// The host's clock at the last tick, in Unix milliseconds.
    now_ms: Option<f64>,
    /// The Unix second and minute last shown.
    shown_sec: Option<i64>,
    shown_min: Option<i64>,
    /// The elements of the clock list.
    ids: Vec<String>,
}

impl WorldClock {
    fn sec(&self) -> Option<i64> {
        Some((self.now_ms? / 1000.0).floor() as i64)
    }

    /// The zones offered for a new clock: those without one.
    fn menus(&self, out: &mut Outbox) {
        let all = |keep: &dyn Fn(usize) -> bool| -> Vec<Value> {
            ZONES
                .iter()
                .enumerate()
                .filter(|(i, _)| keep(*i))
                .map(|(_, z)| json!({"value": z.id, "label": z.city}))
                .collect()
        };
        let add = all(&|i| !self.clocks.contains(&i));
        let first = add
            .first()
            .and_then(|o| o["value"].as_str())
            .unwrap_or_default()
            .to_owned();
        out.signal(json!({
            "menu": {"zones": all(&|_| true), "add": add},
            "add": first,
        }));
    }

    /// Keeps the home zone and the clocks, in the order drawn, in the address.
    fn remember(&self, out: &mut Outbox) {
        let clocks: Vec<&str> = self.clocks.iter().map(|&i| ZONES[i].id).collect();
        let mut m = Metadata::new().name("World Clock");
        m.extra.insert(
            "url".into(),
            format!("#home={}&clocks={}", ZONES[self.home].id, clocks.join(",")),
        );
        out.metadata(m);
    }

    /// Draws the clock list again, west to east by the offset they have now.
    fn draw(&mut self, out: &mut Outbox) {
        let minute = self.sec().unwrap_or_default().div_euclid(60);
        self.clocks
            .sort_by_key(|&i| (ZONES[i].offset(minute), ZONES[i].city));
        let mut els: Vec<Element> = Vec::new();
        for &i in &self.clocks {
            let z = &ZONES[i];
            let (row, s) = (format!("clock_{i}"), format!("s.clocks.z{i}"));
            let id = |what: &str| format!("{what}_{i}");
            els.push(
                ui::row(&row)
                    .parent("clocks")
                    .gap("medium")
                    .justify("between")
                    .wrap(true),
            );
            els.push(ui::column(id("place")).parent(&row).gap("none"));
            els.push(
                ui::text(id("city"))
                    .parent(id("place"))
                    .variant("h3")
                    .text(z.city),
            );
            els.push(
                ui::text(id("date"))
                    .parent(id("place"))
                    .variant("caption")
                    .text(cel(format!("{s}.date + ' · ' + {s}.rel"))),
            );
            els.push(
                ui::text(id("conv"))
                    .parent(id("place"))
                    .variant("caption")
                    .visible(cel(format!("s.conv.on && {s}.conv != ''")))
                    .text(cel(format!("'⇄ ' + {s}.conv"))),
            );
            els.push(ui::row(id("right")).parent(&row).gap("medium").wrap(true));
            els.push(
                ui::badge(id("part"))
                    .parent(id("right"))
                    .variant(cel(format!("{s}.kind")))
                    .text(cel(format!("{s}.part"))),
            );
            els.push(
                ui::text(id("time"))
                    .parent(id("right"))
                    .variant("h2")
                    .text(cel(format!("{s}.time"))),
            );
            els.push(
                ui::button(id("remove"))
                    .parent(id("right"))
                    .label("Remove")
                    .variant("plain")
                    .on_press(Handler::action("remove").payload(z.id)),
            );
        }
        replace(out, &mut self.ids, els);
    }

    /// Patches the home clock and every clock, for the second now.
    fn show_clocks(&self, out: &mut Outbox) {
        let Some(sec) = self.sec() else { return };
        let minute = sec.div_euclid(60);
        let home = &ZONES[self.home];
        let home_off = home.offset(minute);
        let home_day = (sec + home_off * 60).div_euclid(86_400);
        let mut clocks = Map::new();
        for &i in &self.clocks {
            let z = &ZONES[i];
            let off = z.offset(minute);
            let local = sec + off * 60;
            let (days, secs) = (local.div_euclid(86_400), local.rem_euclid(86_400));
            let diff = off - home_off;
            let mut rel = match diff {
                0 => "same as home".to_owned(),
                d => format!("{}h from home", hours(d)),
            };
            match days - home_day {
                1 => rel.push_str(", tomorrow"),
                -1 => rel.push_str(", yesterday"),
                _ => {}
            }
            let (part, kind) = daypart(days, secs);
            clocks.insert(
                format!("z{i}"),
                json!({
                    "time": clock(secs, self.h12, true),
                    "date": format!("{} · UTC{}", date(days, false), hours(off)),
                    "rel": rel,
                    "part": part,
                    "kind": kind,
                }),
            );
        }
        let local = sec + home_off * 60;
        out.signal(json!({
            "now": {
                "time": clock(local.rem_euclid(86_400), self.h12, true),
                "date": date(local.div_euclid(86_400), true),
                "zone": format!("{} · UTC{}", home.city, hours(home_off)),
            },
            "clocks": clocks,
        }));
    }

    /// Converts the converter's time onto every clock. "Today" is today in
    /// the zone it was typed in, so this follows the date as it changes.
    fn show_conv(&self, out: &mut Outbox) {
        let empty = self.conv_time.trim().is_empty();
        let parsed = parse_time(&self.conv_time);
        let (Some(sec), Some(wall)) = (self.sec(), parsed) else {
            let error = if empty || parsed.is_some() {
                ""
            } else {
                "Type a time such as 9:30, 1430 or 2pm."
            };
            out.signal(json!({"conv": {"on": false, "error": error, "summary": ""}}));
            return;
        };
        let from = &ZONES[self.conv_from];
        let minute = sec.div_euclid(60);
        let today = (minute + from.offset(minute)).div_euclid(1440);
        let at = from.minute_at_wall((today + self.conv_day) * 1440 + wall);
        let shown = |z: &Zone| {
            let local = at + z.offset(at);
            let (days, min) = (local.div_euclid(1440), local.rem_euclid(1440));
            format!("{} {}", clock(min * 60, self.h12, false), date(days, false))
        };
        let mut clocks = Map::new();
        for &i in &self.clocks {
            let text = if i == self.conv_from {
                String::new()
            } else {
                shown(&ZONES[i])
            };
            clocks.insert(format!("z{i}"), json!({"conv": text}));
        }
        let home = &ZONES[self.home];
        let mut summary = format!("{} in {}", shown(from), from.city);
        if self.home != self.conv_from {
            summary.push_str(&format!(" is {} at home in {}", shown(home), home.city));
        }
        summary.push('.');
        out.signal(json!({
            "conv": {"on": true, "error": "", "summary": summary},
            "clocks": clocks,
        }));
    }

    /// Reads the converter's inputs from an action.
    fn read_conv(&mut self, action: &Action) {
        self.conv_time = text(action, "conv.time");
        if let Some(i) = find(&text(action, "conv.from")) {
            self.conv_from = i;
        }
        self.conv_day = text(action, "conv.day").parse().unwrap_or(0).clamp(-1, 1);
    }

    fn show(&mut self, out: &mut Outbox) {
        self.draw(out);
        self.show_clocks(out);
        self.show_conv(out);
    }
}

impl App for WorldClock {
    fn hello(&mut self, out: &mut Outbox, params: &BTreeMap<String, String>) {
        self.home = params.get("home").and_then(|h| find(h)).unwrap_or(0);
        self.clocks = match params.get("clocks") {
            Some(c) => c.split(',').filter_map(find).collect(),
            None => DEFAULT_CLOCKS.iter().filter_map(|c| find(c)).collect(),
        };
        self.clocks.dedup();
        self.conv_from = self.home;
        out.metadata(Metadata::new().name("World Clock"));
        out.signal(json!({
            "home": ZONES[self.home].id,
            "h12": false,
            "now": {"time": "--:--:--", "date": "", "zone": ZONES[self.home].city},
            "conv": {"time": "", "from": ZONES[self.home].id, "day": "0"},
            "menu": {"days": [
                {"value": "-1", "label": "Yesterday"},
                {"value": "0", "label": "Today"},
                {"value": "1", "label": "Tomorrow"},
            ]},
        }));
        self.menus(out);
        out.element(ui::column("main").gap("large"));
        out.element(
            ui::text("title")
                .parent("main")
                .variant("h1")
                .text("World Clock"),
        );
        out.element(ui::card("home_card").parent("main").gap("small"));
        out.element(
            ui::text("home_zone")
                .parent("home_card")
                .variant("caption")
                .text(cel("s.now.zone")),
        );
        out.element(
            ui::text("home_time")
                .parent("home_card")
                .variant("h1")
                .text(cel("s.now.time")),
        );
        out.element(
            ui::text("home_date")
                .parent("home_card")
                .text(cel("s.now.date")),
        );
        out.element(
            ui::row("home_controls")
                .parent("home_card")
                .gap("medium")
                .align("end")
                .wrap(true),
        );
        out.element(
            ui::select("home")
                .parent("home_controls")
                .label("Home zone")
                .bind("home")
                .options(cel("s.menu.zones"))
                .on_change(Handler::action("home")),
        );
        out.element(
            ui::checkbox("h12")
                .parent("home_controls")
                .label("12-hour clock")
                .variant("switch")
                .bind("h12")
                .on_change(Handler::action("h12")),
        );
        // The converter.
        out.element(
            ui::card("converter")
                .parent("main")
                .title("Convert a time")
                .gap("medium"),
        );
        out.element(
            ui::row("conv_inputs")
                .parent("converter")
                .gap("medium")
                .align("end")
                .wrap(true),
        );
        out.element(
            ui::input("conv_time")
                .parent("conv_inputs")
                .label("Time")
                .placeholder("e.g. 9:30 or 2pm")
                .bind("conv.time")
                .error(cel("s.conv.error"))
                .on_change(Handler::action("convert")),
        );
        out.element(
            ui::select("conv_from")
                .parent("conv_inputs")
                .label("In")
                .bind("conv.from")
                .options(cel("s.menu.zones"))
                .on_change(Handler::action("convert")),
        );
        out.element(
            ui::select("conv_day")
                .parent("converter")
                .variant("segmented")
                .bind("conv.day")
                .options(cel("s.menu.days"))
                .on_change(Handler::action("convert")),
        );
        out.element(
            ui::notice("conv_summary")
                .parent("converter")
                .variant("info")
                .visible(cel("s.conv.on"))
                .text(cel("s.conv.summary")),
        );
        out.element(
            ui::text("conv_hint")
                .parent("converter")
                .variant("caption")
                .visible(cel("!s.conv.on"))
                .text("Type a time to see it on every clock below."),
        );
        out.element(
            ui::row("conv_buttons")
                .parent("converter")
                .gap("medium")
                .wrap(true),
        );
        out.element(
            ui::button("conv_now")
                .parent("conv_buttons")
                .label("Now")
                .on_press(Handler::action("conv_now")),
        );
        out.element(
            ui::button("conv_clear")
                .parent("conv_buttons")
                .label("Clear")
                .variant("plain")
                .disabled(cel("s.conv.time == ''"))
                .on_press(Handler::action("conv_clear")),
        );
        // The clocks.
        out.element(ui::card("clocks_card").parent("main").title("Clocks"));
        out.element(
            ui::list("clocks")
                .parent("clocks_card")
                .empty("No clocks. Add a zone below."),
        );
        out.element(
            ui::row("add_row")
                .parent("clocks_card")
                .gap("medium")
                .align("end")
                .wrap(true),
        );
        out.element(
            ui::select("add")
                .parent("add_row")
                .label("Add a clock")
                .bind("add")
                .options(cel("s.menu.add"))
                .disabled(cel("size(s.menu.add) == 0")),
        );
        out.element(
            ui::button("add_button")
                .parent("add_row")
                .label("Add")
                .variant("primary")
                .disabled(cel("size(s.menu.add) == 0"))
                .on_press(Handler::action("add")),
        );
        self.draw(out);
        self.show_conv(out);
    }

    fn action(&mut self, out: &mut Outbox, action: &Action) {
        match action.name.as_str() {
            "home" => {
                if let Some(i) = find(&text(action, "home")) {
                    self.home = i;
                    self.remember(out);
                    self.show_clocks(out);
                    self.show_conv(out);
                }
            }
            "h12" => {
                self.h12 = action.signal("h12").and_then(Value::as_bool) == Some(true);
                self.show_clocks(out);
                self.show_conv(out);
            }
            "add" => {
                if let Some(i) = find(&text(action, "add"))
                    && !self.clocks.contains(&i)
                {
                    self.clocks.push(i);
                    self.menus(out);
                    self.show(out);
                    self.remember(out);
                }
            }
            "remove" => {
                let id = action.payload.as_ref().and_then(Value::as_str);
                if let Some(i) = id.and_then(find) {
                    self.clocks.retain(|&c| c != i);
                    out.signal(json!({"clocks": {format!("z{i}"): null}}));
                    self.menus(out);
                    self.draw(out);
                    self.remember(out);
                }
            }
            "convert" => {
                self.read_conv(action);
                self.show_conv(out);
            }
            "conv_now" => {
                self.read_conv(action);
                if let Some(sec) = self.sec() {
                    let from = &ZONES[self.conv_from];
                    let local = sec.div_euclid(60) + from.offset(sec.div_euclid(60));
                    self.conv_time = clock(local.rem_euclid(1440) * 60, self.h12, false);
                    self.conv_day = 0;
                    out.signal(json!({"conv": {"time": self.conv_time, "day": "0"}}));
                    self.show_conv(out);
                }
            }
            "conv_clear" => {
                self.conv_time.clear();
                out.signal(json!({"conv": {"time": ""}}));
                self.show_conv(out);
            }
            _ => {}
        }
    }

    fn tick(&mut self, out: &mut Outbox, now_ms: f64) {
        let first = self.now_ms.is_none();
        self.now_ms = Some(now_ms);
        let Some(sec) = self.sec() else { return };
        if self.shown_sec == Some(sec) {
            return;
        }
        self.shown_sec = Some(sec);
        let minute = sec.div_euclid(60);
        if self.shown_min != Some(minute) {
            self.shown_min = Some(minute);
            // Offsets change, and dates roll, only on a minute; the first
            // tick also sorts the clocks by their offsets now.
            if first {
                self.draw(out);
            }
            self.show_conv(out);
        }
        self.show_clocks(out);
    }

    fn tick_interval(&self) -> Option<u32> {
        Some(TICK_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypereffect::server::Loopback;

    /// A Unix minute from a UTC date and time.
    fn utc(y: i64, mo: u32, d: u32, h: i64, mi: i64) -> i64 {
        days_from_civil(y, mo, d) * 1440 + h * 60 + mi
    }

    fn zone(id: &str) -> &'static Zone {
        &ZONES[find(id).unwrap()]
    }

    #[test]
    fn calendar() {
        for days in -800_000..800_000 {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(weekday(days_from_civil(2026, 11, 3)), 2);
        assert_eq!(sunday(2026, 3, 2), days_from_civil(2026, 3, 8));
        assert_eq!(sunday(2026, 3, 0), days_from_civil(2026, 3, 29));
        assert_eq!(sunday(2026, 10, 1), days_from_civil(2026, 10, 4));
        assert_eq!(sunday(2026, 12, 0), days_from_civil(2026, 12, 27));
    }

    #[test]
    fn summer_time() {
        let ny = zone("America/New_York");
        assert_eq!(ny.offset(utc(2026, 3, 8, 6, 59)), -300);
        assert_eq!(ny.offset(utc(2026, 3, 8, 7, 0)), -240);
        assert_eq!(ny.offset(utc(2026, 11, 1, 5, 59)), -240);
        assert_eq!(ny.offset(utc(2026, 11, 1, 6, 0)), -300);
        let london = zone("Europe/London");
        assert_eq!(london.offset(utc(2026, 3, 29, 0, 59)), 0);
        assert_eq!(london.offset(utc(2026, 3, 29, 1, 0)), 60);
        assert_eq!(london.offset(utc(2026, 10, 25, 1, 0)), 0);
        let berlin = zone("Europe/Berlin");
        assert_eq!(berlin.offset(utc(2026, 7, 1, 12, 0)), 120);
        let sydney = zone("Australia/Sydney");
        assert_eq!(sydney.offset(utc(2026, 4, 4, 15, 59)), 660);
        assert_eq!(sydney.offset(utc(2026, 4, 4, 16, 0)), 600);
        assert_eq!(sydney.offset(utc(2026, 10, 3, 15, 59)), 600);
        assert_eq!(sydney.offset(utc(2026, 10, 3, 16, 0)), 660);
        assert_eq!(sydney.offset(utc(2027, 1, 1, 0, 0)), 660);
        let auckland = zone("Pacific/Auckland");
        assert_eq!(auckland.offset(utc(2026, 9, 26, 13, 59)), 720);
        assert_eq!(auckland.offset(utc(2026, 9, 26, 14, 0)), 780);
        assert_eq!(zone("Asia/Kathmandu").offset(utc(2026, 7, 1, 0, 0)), 345);
    }

    #[test]
    fn wall_times() {
        let ny = zone("America/New_York");
        let wall = |d, h: i64, m: i64| days_from_civil(2026, 3, d) * 1440 + h * 60 + m;
        assert_eq!(ny.minute_at_wall(wall(7, 12, 0)), utc(2026, 3, 7, 17, 0));
        assert_eq!(ny.minute_at_wall(wall(8, 12, 0)), utc(2026, 3, 8, 16, 0));
        // 02:30 does not exist on the morning summer time starts.
        assert_eq!(ny.minute_at_wall(wall(8, 2, 30)), utc(2026, 3, 8, 7, 30));
        // 01:30 happens twice on the morning it ends: the first one.
        let nov = days_from_civil(2026, 11, 1) * 1440 + 90;
        assert_eq!(ny.minute_at_wall(nov), utc(2026, 11, 1, 5, 30));
    }

    #[test]
    fn typed_times() {
        assert_eq!(parse_time("9"), Some(540));
        assert_eq!(parse_time("9:30"), Some(570));
        assert_eq!(parse_time("09.30"), Some(570));
        assert_eq!(parse_time("0930"), Some(570));
        assert_eq!(parse_time("930"), Some(570));
        assert_eq!(parse_time("14h"), Some(840));
        assert_eq!(parse_time("2pm"), Some(840));
        assert_eq!(parse_time("12 am"), Some(0));
        assert_eq!(parse_time("12:15 PM"), Some(735));
        assert_eq!(parse_time("24:00"), None);
        assert_eq!(parse_time("13pm"), None);
        assert_eq!(parse_time("9:5"), None);
        assert_eq!(parse_time("noon"), None);
        assert_eq!(parse_time(""), None);
    }

    #[test]
    fn formats() {
        assert_eq!(hours(0), "+0");
        assert_eq!(hours(330), "+5:30");
        assert_eq!(hours(-240), "-4");
        assert_eq!(clock(0, true, false), "12:00 am");
        assert_eq!(clock(13 * 3600 + 5 * 60 + 9, false, true), "13:05:09");
        assert_eq!(clock(13 * 3600 + 5 * 60, true, false), "1:05 pm");
        assert_eq!(date(days_from_civil(2026, 11, 3), true), "Tue 3 Nov 2026");
    }

    /// 2026-11-03 14:30:15 UTC, in milliseconds.
    const NOW: f64 = ((20_760_i64 * 86_400 + 14 * 3600 + 30 * 60 + 15) * 1000) as f64;

    fn params(home: &str, clocks: &str) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("home".into(), home.into()),
            ("clocks".into(), clocks.into()),
        ])
    }

    #[test]
    fn clocks_tick() {
        assert_eq!(days_from_civil(2026, 11, 3), 20_760);
        let mut l = Loopback::<WorldClock>::with_params(&params(
            "Europe/Paris",
            "Asia/Tokyo,America/New_York,Asia/Kolkata",
        ));
        assert_eq!(l.client.value("now.time"), Some(&json!("--:--:--")));
        l.tick(NOW);
        assert_eq!(l.client.value("now.time"), Some(&json!("15:30:15")));
        assert_eq!(l.client.value("now.date"), Some(&json!("Tue 3 Nov 2026")));
        assert_eq!(l.client.value("now.zone"), Some(&json!("Paris · UTC+1")));
        let ny = find("America/New_York").unwrap();
        let tokyo = find("Asia/Tokyo").unwrap();
        assert_eq!(
            l.client.value(&format!("clocks.z{ny}.time")),
            Some(&json!("09:30:15"))
        );
        assert_eq!(
            l.client.value(&format!("clocks.z{tokyo}.rel")),
            Some(&json!("+8h from home"))
        );
        assert_eq!(
            l.client.value(&format!("clocks.z{tokyo}.part")),
            Some(&json!("Night"))
        );
        assert_eq!(
            l.client.attr(&format!("date_{ny}"), "text"),
            Some(json!("Tue 3 Nov · UTC-5 · -6h from home"))
        );
        // West to east.
        assert_eq!(l.server.app().unwrap().clocks[0], ny);
        l.tick(NOW + 1000.0);
        assert_eq!(l.client.value("now.time"), Some(&json!("15:30:16")));
        assert!(l.client.take_diagnostics().is_empty());
    }

    #[test]
    fn converts_and_remembers() {
        let mut l = Loopback::<WorldClock>::with_params(&params(
            "Europe/Paris",
            "Asia/Tokyo,America/New_York",
        ));
        l.tick(NOW);
        let tokyo = find("Asia/Tokyo").unwrap();
        // 23:00 tomorrow in New York is Thursday morning in Tokyo.
        l.write("conv.time", json!("11pm"));
        l.write("conv.from", json!("America/New_York"));
        l.write("conv.day", json!("1"));
        assert!(l.trigger("conv_time", "onChange"));
        assert_eq!(l.client.value("conv.on"), Some(&json!(true)));
        assert_eq!(
            l.client.value(&format!("clocks.z{tokyo}.conv")),
            Some(&json!("13:00 Thu 5 Nov"))
        );
        assert_eq!(
            l.client.value("conv.summary"),
            Some(&json!(
                "23:00 Wed 4 Nov in New York is 05:00 Thu 5 Nov at home in Paris."
            ))
        );
        l.write("conv.time", json!("25:00"));
        assert!(l.trigger("conv_time", "onChange"));
        assert_eq!(l.client.value("conv.on"), Some(&json!(false)));
        assert_ne!(l.client.value("conv.error"), Some(&json!("")));
        // Add and remove clocks; the address follows.
        l.write("add", json!("Europe/London"));
        assert!(l.trigger("add_button", "onPress"));
        let london = find("Europe/London").unwrap();
        assert!(l.client.attr(&format!("time_{london}"), "text").is_some());
        assert_eq!(
            l.client.metadata().extra.get("url").map(String::as_str),
            Some("#home=Europe/Paris&clocks=America/New_York,Europe/London,Asia/Tokyo")
        );
        assert!(l.trigger(&format!("remove_{tokyo}"), "onPress"));
        assert!(l.client.attr(&format!("time_{tokyo}"), "text").is_none());
        assert_eq!(
            l.client.metadata().extra.get("url").map(String::as_str),
            Some("#home=Europe/Paris&clocks=America/New_York,Europe/London")
        );
        assert!(l.client.take_diagnostics().is_empty());
    }
}
