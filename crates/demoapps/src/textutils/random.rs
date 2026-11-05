//! Random picks and generators, and a few things for fun. The randomness
//! comes from the clock: fine for games and samples, not for secrets.

use super::Output;
use super::names;
use super::{Opts, Rng, Tool, fmt, table, tool};

fn list(text: &str) -> Vec<&str> {
    text.lines()
        .flat_map(|l| {
            if l.contains(',') && !l.contains(' ') {
                l.split(',').collect::<Vec<_>>()
            } else {
                vec![l]
            }
        })
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect()
}

/// Items drawn at random: one as a value, more as a list.
fn drawn(label: &str, items: Vec<String>) -> Output {
    if items.len() == 1 {
        table(&[(label, items[0].clone())])
    } else {
        super::list(items)
    }
}

/// Picks `n` items, without repeats unless `repeat`.
fn pick<'a>(items: &[&'a str], n: usize, repeat: bool, rng: &mut Rng) -> Vec<&'a str> {
    if repeat {
        return (0..n).map(|_| items[rng.below(items.len())]).collect();
    }
    let mut v = items.to_vec();
    rng.shuffle(&mut v);
    v.truncate(n);
    v
}

/// A name generator over a list, with the filters of text-utils.com:
/// how many, starting and ending letters, letters it must contain, and
/// the number of words.
fn generate(all: &[&str], o: &mut Opts) -> Result<Output, String> {
    let (starts, ends, has) = (
        o.field(0).to_lowercase(),
        o.field(1).to_lowercase(),
        o.field(2).to_lowercase(),
    );
    let words = o.opt_num(3)?.map(|w| w as usize);
    let pool: Vec<&str> = all
        .iter()
        .copied()
        .filter(|n| {
            let l = n.to_lowercase();
            l.starts_with(&starts)
                && l.ends_with(&ends)
                && has
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .all(|c| l.contains(c))
                && words.is_none_or(|w| n.split_whitespace().count() == w)
        })
        .collect();
    if pool.is_empty() {
        return Err("Nothing matches those filters.".into());
    }
    let n = o.number.max(1);
    let mut out = pick(
        &pool,
        n.min(if o.flag { n } else { pool.len() }),
        o.flag,
        o.rng,
    );
    if o.check(1) {
        out.sort();
    }
    let note = format!("{} of {} that match", out.len(), pool.len());
    Ok(drawn("Picked", out.iter().map(|s| s.to_string()).collect()).note(note))
}

macro_rules! generators {
    ($($fn:ident, $list:expr, $id:literal, $name:literal, $about:literal;)*) => {
        $(
            fn $fn(_: &str, o: &mut Opts) -> Result<Output, String> {
                generate($list, o)
            }
        )*
        const GENERATORS: [Tool; 6] = [
            $(
                tool($id, $name, $about, $fn)
                    .form()
                    .fields(&[("Starts with", ""), ("Ends with", ""), ("Contains the letters", ""), ("Words in it", "")])
                    .checks(&["Allow repeats", "Sort them"])
                    .slider("How many", 1.0, 50.0, 5.0),
            )*
        ];
    };
}

generators! {
    countries, &names::COUNTRIES.map(|c| c.0), "random-country", "Random country", "Random countries of the world.";
    capitals, &names::COUNTRIES.map(|c| c.1), "random-capital", "Random national capital", "Random capital cities.";
    female, names::FEMALE, "random-female-name", "Random female name", "Random female given names.";
    male, names::MALE, "random-male-name", "Random male name", "Random male given names.";
    teams, names::NBA_TEAMS, "random-nba-team", "Random NBA team", "Random NBA teams.";
    presidents, names::PRESIDENTS, "random-president", "Random US president", "Random presidents of the United States.";
}

fn random_number(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (lo, hi) = (o.num(0)?, o.num(1)?);
    if hi < lo {
        return Err("The maximum is below the minimum.".into());
    }
    let n = o.number.max(1);
    let decimals = o.opt_num(2)?.unwrap_or(0.0).clamp(0.0, 10.0) as usize;
    if decimals == 0 {
        let (lo, hi) = (lo.ceil() as i64, hi.floor() as i64);
        let span = (hi - lo + 1) as usize;
        if o.flag && n > span {
            return Err(format!(
                "There are only {span} different numbers in that range."
            ));
        }
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        while out.len() < n {
            let v = lo + o.rng.below(span) as i64;
            if !o.flag || seen.insert(v) {
                out.push(v);
            }
        }
        if o.check(1) {
            out.sort();
        }
        return Ok(drawn(
            "Random number",
            out.iter().map(i64::to_string).collect(),
        ));
    }
    let out: Vec<String> = (0..n)
        .map(|_| format!("{:.*}", decimals, lo + o.rng.unit() * (hi - lo)))
        .collect();
    Ok(drawn("Random number", out))
}

fn dice(_: &str, o: &mut Opts) -> Result<Output, String> {
    let re =
        regex_lite::Regex::new(r"(?i)^\s*(\d*)\s*d\s*(\d+|%)\s*(?:([+-])\s*(\d+))?\s*$").unwrap();
    let mut out = Vec::new();
    for spec in o.field(0).split(',') {
        let c = re
            .captures(spec)
            .ok_or_else(|| format!("“{}” is not dice notation such as 2d6+1.", spec.trim()))?;
        let count: usize = c[1].parse().unwrap_or(1).clamp(1, 1000);
        let sides: usize = if &c[2] == "%" {
            100
        } else {
            c[2].parse().unwrap_or(6)
        };
        if sides < 2 {
            return Err("Dice have at least two sides.".into());
        }
        let bonus: i64 = c.get(4).map_or(0, |m| m.as_str().parse().unwrap_or(0))
            * if c.get(3).is_some_and(|s| s.as_str() == "-") {
                -1
            } else {
                1
            };
        let rolls: Vec<usize> = (0..count).map(|_| o.rng.below(sides) + 1).collect();
        let total = rolls.iter().sum::<usize>() as i64 + bonus;
        let rolled = rolls
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(" + ");
        let bonus = if bonus != 0 {
            format!(" {} {}", if bonus < 0 { "−" } else { "+" }, bonus.abs())
        } else {
            String::new()
        };
        out.push(vec![
            spec.trim().to_owned(),
            total.to_string(),
            format!("{rolled}{bonus}"),
        ]);
    }
    Ok(super::grid(&["Dice", "Total", "Rolls"], out))
}

fn choice(text: &str, o: &mut Opts) -> Result<Output, String> {
    let items = list(text);
    if items.is_empty() {
        return Err("Write the choices, one per line.".into());
    }
    let n = o
        .number
        .max(1)
        .min(if o.flag { usize::MAX } else { items.len() });
    Ok(drawn(
        "Picked",
        pick(&items, n, o.flag, o.rng)
            .iter()
            .map(|s| s.to_string())
            .collect(),
    ))
}

fn shuffle(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut items = list(text);
    o.rng.shuffle(&mut items);
    Ok(super::list(items))
}

fn teams_of(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut people = list(text);
    let k = o.number.max(2);
    if people.len() < k {
        return Err(format!("There are fewer than {k} people."));
    }
    o.rng.shuffle(&mut people);
    let mut teams: Vec<Vec<&str>> = vec![Vec::new(); k];
    for (i, p) in people.iter().enumerate() {
        teams[i % k].push(p);
    }
    let names: Vec<String> = (1..=k).map(|i| format!("Team {i}")).collect();
    Ok(table(
        &teams
            .iter()
            .zip(&names)
            .map(|(t, name)| (name.as_str(), t.join(", ")))
            .collect::<Vec<_>>(),
    ))
}

fn secret_santa(text: &str, o: &mut Opts) -> Result<Output, String> {
    let people = list(text);
    if people.len() < 3 {
        return Err("Write at least three names.".into());
    }
    // A random cycle: nobody draws themselves, and nobody draws the person
    // who draws them.
    let mut order = people.clone();
    o.rng.shuffle(&mut order);
    Ok(super::grid(
        &["Giver", "Gives to"],
        (0..order.len()).map(|i| vec![order[i], order[(i + 1) % order.len()]]),
    ))
}

fn lottery(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = o.num(0)? as usize;
    let max = o.num(1)? as usize;
    if n == 0 || n > max {
        return Err("Draw at least one number, and no more than the pool.".into());
    }
    let mut pool: Vec<usize> = (1..=max).collect();
    o.rng.shuffle(&mut pool);
    let mut main = pool[..n].to_vec();
    main.sort();
    let mut out = vec![(
        "Numbers",
        main.iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join("  "),
    )];
    if let Some(b) = o.opt_num(2)?.filter(|b| *b >= 1.0) {
        out.push(("Bonus", (o.rng.below(b as usize) + 1).to_string()));
    }
    let odds: f64 = (0..n).map(|i| (max - i) as f64 / (n - i) as f64).product();
    Ok(table(&out).note(format!("Odds of all {n}: 1 in {}", fmt(odds.round(), 0))))
}

fn coin(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = o.number.max(1);
    let flips: Vec<bool> = (0..n).map(|_| o.rng.next() & 1 == 0).collect();
    let heads = flips.iter().filter(|f| **f).count();
    let shown: String = flips.iter().map(|f| if *f { 'H' } else { 'T' }).collect();
    Ok(table(&[
        ("Flips", shown),
        ("Heads", heads.to_string()),
        ("Tails", (n - heads).to_string()),
    ]))
}

fn bingo(_: &str, o: &mut Opts) -> Result<Output, String> {
    let cards = o.number.max(1);
    let mut out = Vec::new();
    for k in 0..cards {
        let cols: Vec<Vec<usize>> = (0..5)
            .map(|c| {
                let mut v: Vec<usize> = (c * 15 + 1..=c * 15 + 15).collect();
                o.rng.shuffle(&mut v);
                v.truncate(5);
                v
            })
            .collect();
        out.push(format!("Card {}\n  B   I   N   G   O", k + 1));
        for r in 0..5 {
            out.push(
                (0..5)
                    .map(|c| {
                        if r == 2 && c == 2 {
                            "  ★".to_owned()
                        } else {
                            format!("{:>3}", cols[c][r])
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
        out.push(String::new());
    }
    Ok(out.join("\n").trim_end().into())
}

fn bracket(text: &str, o: &mut Opts) -> Result<Output, String> {
    let mut teams = list(text);
    if teams.len() < 2 {
        return Err("Write at least two teams.".into());
    }
    if o.flag {
        o.rng.shuffle(&mut teams);
    }
    let size = teams.len().next_power_of_two();
    // Seeded pairs: 1 v N, 2 v N-1…; missing seats are byes.
    let mut out = Vec::new();
    for i in 0..size / 2 {
        let a = teams.get(i).copied().unwrap_or("bye");
        let b = teams.get(size - 1 - i).copied().unwrap_or("bye");
        out.push(format!("{a} vs {b}"));
    }
    Ok(Output::default()
        .note(format!(
            "Round 1 of {} ({} teams)",
            size.trailing_zeros(),
            teams.len()
        ))
        .list(out))
}

fn mock(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = o.number.max(1);
    let domains = [
        "example.com",
        "mail.test",
        "inbox.example",
        "post.example.org",
    ];
    let streets = [
        "Oak", "Maple", "Pine", "Cedar", "Elm", "Willow", "Lake", "Hill", "River", "Park",
    ];
    let mut rows = Vec::new();
    for i in 0..n {
        let female = o.rng.next() & 1 == 0;
        let first = if female {
            names::FEMALE[o.rng.below(names::FEMALE.len())]
        } else {
            names::MALE[o.rng.below(names::MALE.len())]
        };
        let last = names::SURNAMES[o.rng.below(names::SURNAMES.len())];
        let (country, city) = names::COUNTRIES[o.rng.below(names::COUNTRIES.len())];
        let email = format!(
            "{}.{}@{}",
            first.to_lowercase(),
            last.to_lowercase(),
            domains[o.rng.below(domains.len())]
        );
        let phone = format!("+1-555-{:03}-{:04}", o.rng.below(1000), o.rng.below(10000));
        let born = super::dates::iso(-7300 + o.rng.below(18000) as i64);
        let address = format!(
            "{} {} Street",
            1 + o.rng.below(999),
            streets[o.rng.below(streets.len())]
        );
        rows.push([
            (i + 1).to_string(),
            first.into(),
            last.into(),
            email,
            phone,
            born,
            address,
            city.into(),
            country.into(),
        ]);
    }
    const KEYS: [&str; 9] = [
        "id",
        "first_name",
        "last_name",
        "email",
        "phone",
        "birth_date",
        "address",
        "city",
        "country",
    ];
    Ok(super::snippet(match o.mode {
        "csv" => std::iter::once(KEYS.join(","))
            .chain(rows.iter().map(|r| {
                r.iter()
                    .map(|c| {
                        if c.contains(',') {
                            format!("\"{c}\"")
                        } else {
                            c.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            }))
            .collect::<Vec<_>>()
            .join("\n"),
        "sql" => rows
            .iter()
            .map(|r| {
                format!(
                    "INSERT INTO people ({}) VALUES ({}, {});",
                    KEYS.join(", "),
                    r[0],
                    r[1..]
                        .iter()
                        .map(|c| format!("'{}'", c.replace('\'', "''")))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => {
            let v: Vec<hypereffect::Value> = rows
                .iter()
                .map(|r| {
                    let mut m = hypereffect::Map::new();
                    for (k, c) in KEYS.iter().zip(r) {
                        m.insert(
                            k.to_string(),
                            if *k == "id" {
                                hypereffect::json!(c.parse::<u64>().unwrap())
                            } else {
                                hypereffect::json!(c)
                            },
                        );
                    }
                    hypereffect::Value::Object(m)
                })
                .collect();
            super::code::json_text(&hypereffect::Value::Array(v), 2)
        }
    }))
}

fn pet_name(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = o.number.max(1);
    Ok(drawn(
        "Name",
        pick(names::PETS, n.min(names::PETS.len()), false, o.rng)
            .iter()
            .map(|s| s.to_string())
            .collect(),
    ))
}

// Fun

const EIGHT_BALL: [&str; 20] = [
    "It is certain.",
    "It is decidedly so.",
    "Without a doubt.",
    "Yes, definitely.",
    "You may rely on it.",
    "As I see it, yes.",
    "Most likely.",
    "Outlook good.",
    "Yes.",
    "Signs point to yes.",
    "Reply hazy, try again.",
    "Ask again later.",
    "Better not tell you now.",
    "Cannot predict now.",
    "Concentrate and ask again.",
    "Don't count on it.",
    "My reply is no.",
    "My sources say no.",
    "Outlook not so good.",
    "Very doubtful.",
];

fn eight_ball(_: &str, o: &mut Opts) -> Result<Output, String> {
    Ok(table(&[(
        "The Magic 8 Ball says",
        EIGHT_BALL[o.rng.below(EIGHT_BALL.len())].into(),
    )]))
}

const FORTUNES: [&str; 16] = [
    "A small step today becomes a long road tomorrow.",
    "Your patience will be rewarded soon.",
    "An unexpected letter brings good news.",
    "Curiosity will lead you somewhere wonderful.",
    "Share your umbrella; you will be glad you did.",
    "The best time to start was yesterday; the next best is now.",
    "A friend's advice is worth taking this week.",
    "You will find what you lost when you stop looking.",
    "Kindness given returns doubled.",
    "Your next idea is better than your last.",
    "A quiet morning holds a loud success.",
    "Say yes to the next invitation.",
    "Fortune favours the well rested.",
    "The answer you seek is in a book you have not opened.",
    "Something old will feel new again.",
    "Today is a good day to learn a new word.",
];

fn fortune(_: &str, o: &mut Opts) -> Result<Output, String> {
    let lucky: Vec<String> = pick(
        &[
            "1", "4", "7", "8", "11", "13", "18", "21", "23", "27", "32", "36", "40", "42", "45",
        ],
        6,
        false,
        o.rng,
    )
    .iter()
    .map(|s| s.to_string())
    .collect();
    Ok(
        super::prose(format!("🥠 {}", FORTUNES[o.rng.below(FORTUNES.len())]))
            .facts(&[("Lucky numbers", lucky.join(" "))]),
    )
}

const MAJOR: [&str; 22] = [
    "The Fool",
    "The Magician",
    "The High Priestess",
    "The Empress",
    "The Emperor",
    "The Hierophant",
    "The Lovers",
    "The Chariot",
    "Strength",
    "The Hermit",
    "Wheel of Fortune",
    "Justice",
    "The Hanged Man",
    "Death",
    "Temperance",
    "The Devil",
    "The Tower",
    "The Star",
    "The Moon",
    "The Sun",
    "Judgement",
    "The World",
];

fn tarot(_: &str, o: &mut Opts) -> Result<Output, String> {
    let ranks = [
        "Ace", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten", "Page",
        "Knight", "Queen", "King",
    ];
    let suits = ["Wands", "Cups", "Swords", "Pentacles"];
    let mut deck: Vec<String> = MAJOR.iter().map(|s| s.to_string()).collect();
    for s in suits {
        for r in ranks {
            deck.push(format!("{r} of {s}"));
        }
    }
    o.rng.shuffle(&mut deck);
    let positions: &[&str] = match o.mode {
        "three" => &["Past", "Present", "Future"],
        "five" => &[
            "Situation",
            "Challenge",
            "Advice",
            "Hidden influence",
            "Outcome",
        ],
        _ => &["Card"],
    };
    let rows: Vec<(&str, String)> = positions
        .iter()
        .zip(&deck)
        .map(|(p, c)| {
            (
                *p,
                format!(
                    "{c}{}",
                    if o.rng.next() % 4 == 0 {
                        " (reversed)"
                    } else {
                        ""
                    }
                ),
            )
        })
        .collect();
    Ok(table(&rows).note("For fun only."))
}

const RUNES: [(&str, &str); 24] = [
    ("ᚠ Fehu", "wealth"),
    ("ᚢ Uruz", "strength"),
    ("ᚦ Thurisaz", "a challenge"),
    ("ᚨ Ansuz", "a message"),
    ("ᚱ Raidho", "a journey"),
    ("ᚲ Kenaz", "insight"),
    ("ᚷ Gebo", "a gift"),
    ("ᚹ Wunjo", "joy"),
    ("ᚺ Hagalaz", "disruption"),
    ("ᚾ Naudiz", "need"),
    ("ᛁ Isa", "stillness"),
    ("ᛃ Jera", "harvest"),
    ("ᛇ Eihwaz", "endurance"),
    ("ᛈ Perthro", "mystery"),
    ("ᛉ Algiz", "protection"),
    ("ᛊ Sowilo", "success"),
    ("ᛏ Tiwaz", "courage"),
    ("ᛒ Berkano", "growth"),
    ("ᛖ Ehwaz", "trust"),
    ("ᛗ Mannaz", "community"),
    ("ᛚ Laguz", "intuition"),
    ("ᛜ Ingwaz", "new beginnings"),
    ("ᛞ Dagaz", "a breakthrough"),
    ("ᛟ Othala", "home"),
];

fn runes(_: &str, o: &mut Opts) -> Result<Output, String> {
    let mut idx: Vec<usize> = (0..24).collect();
    o.rng.shuffle(&mut idx);
    Ok(table(
        &idx.iter()
            .take(o.number.max(1))
            .map(|i| (RUNES[*i].0, RUNES[*i].1.to_owned()))
            .collect::<Vec<_>>(),
    ))
}

fn reduce(mut n: u64) -> u64 {
    while n > 9 && n != 11 && n != 22 && n != 33 {
        n = n
            .to_string()
            .chars()
            .map(|c| c.to_digit(10).unwrap() as u64)
            .sum();
    }
    n
}

fn numerology(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (y, m, d) = super::dates::civil_from_days(o.date(0)?);
    let life = reduce(reduce(y as u64) + reduce(m as u64) + reduce(d as u64));
    let name = o.field(1).to_lowercase();
    let value = |c: char| {
        if c.is_ascii_lowercase() {
            (c as u64 - 'a' as u64) % 9 + 1
        } else {
            0
        }
    };
    let vowels: u64 = name
        .chars()
        .filter(|c| "aeiou".contains(*c))
        .map(value)
        .sum();
    let all: u64 = name.chars().map(value).sum();
    let mut rows = vec![("Life path number", life.to_string())];
    if !name.is_empty() {
        rows.push(("Expression number", reduce(all).to_string()));
        rows.push(("Soul urge number", reduce(vowels).to_string()));
        rows.push(("Personality number", reduce(all - vowels).to_string()));
    }
    rows.push(("", "For fun only.".into()));
    Ok(table(&rows))
}

fn biorhythm(_: &str, o: &mut Opts) -> Result<Output, String> {
    let days = (o.date(1)? - o.date(0)?) as f64;
    let wave = |p: f64| (2.0 * std::f64::consts::PI * days / p).sin() * 100.0;
    Ok(table(&[
        ("Physical (23 days)", format!("{:+.0}%", wave(23.0))),
        ("Emotional (28 days)", format!("{:+.0}%", wave(28.0))),
        ("Intellectual (33 days)", format!("{:+.0}%", wave(33.0))),
        ("", format!("Day {} of life; for fun only.", fmt(days, 0))),
    ]))
}

fn life_weeks(_: &str, o: &mut Opts) -> Result<Output, String> {
    let lived = ((o.date(1)? - o.date(0)?) / 7).max(0) as usize;
    let years = o.opt_num(2)?.unwrap_or(80.0).clamp(1.0, 120.0) as usize;
    let mut out = Vec::new();
    for y in 0..years {
        let row: String = (0..52)
            .map(|w| if y * 52 + w < lived { '■' } else { '□' })
            .collect();
        out.push(format!("{y:>3} {row}"));
    }
    Ok(table(&[
        ("Weeks lived", fmt(lived as f64, 0)),
        ("Of", fmt((years * 52) as f64, 0)),
        (
            "Share",
            format!("{}%", fmt(lived as f64 * 100.0 / (years * 52) as f64, 1)),
        ),
    ])
    .code(out.join("\n")))
}

fn t9(text: &str, o: &mut Opts) -> Result<Output, String> {
    const KEYS: [&str; 10] = [
        " ", "", "abc", "def", "ghi", "jkl", "mno", "pqrs", "tuv", "wxyz",
    ];
    if o.mode == "multitap" {
        return Ok(text
            .to_lowercase()
            .chars()
            .map(|c| {
                KEYS.iter()
                    .enumerate()
                    .find_map(|(k, s)| s.find(c).map(|i| k.to_string().repeat(i + 1)))
                    .unwrap_or_else(|| c.to_string())
            })
            .collect::<Vec<_>>()
            .join(" ")
            .into());
    }
    Ok(super::snippet(
        text.to_lowercase()
            .chars()
            .map(|c| {
                KEYS.iter()
                    .position(|s| s.contains(c))
                    .map_or(c, |k| (b'0' + k as u8) as char)
            })
            .collect(),
    ))
}

const PROMPTS: [&str; 16] = [
    "Write about a door that only opens once a year.",
    "A letter arrives addressed to someone who lived in your home a century ago.",
    "Describe your town from the point of view of a stray cat.",
    "Two strangers share an umbrella; one of them is lying.",
    "The last bookshop in the city is closing tonight.",
    "Write a recipe that is secretly a love story.",
    "A map shows a road that does not exist.",
    "Your character wakes up fluent in a language nobody speaks.",
    "Describe a meal that changed someone's mind.",
    "The lighthouse keeper sees a light out at sea.",
    "Write a conversation between a river and a bridge.",
    "A photograph shows you somewhere you have never been.",
    "The power goes out for a week.",
    "Someone keeps leaving flowers on the same bench.",
    "Write about the first day of a strange new job.",
    "A clock in the house starts running backwards.",
];

fn prompt(_: &str, o: &mut Opts) -> Result<Output, String> {
    Ok(super::prose(PROMPTS[o.rng.below(PROMPTS.len())]))
}

pub const RANDOM: &[Tool] = &[
    tool(
        "random-number",
        "Random number",
        "Random numbers in a range.",
        random_number,
    )
    .form()
    .fields(&[("Minimum", "1"), ("Maximum", "100"), ("Decimals", "0")])
    .checks(&["No repeats", "Sort them"])
    .slider("How many", 1.0, 100.0, 1.0),
    tool(
        "dice",
        "Dice roller",
        "Rolls dice in notation such as 2d6+1, d20, 4d6, d%.",
        dice,
    )
    .form()
    .fields(&[("Dice (comma separated)", "2d6, d20")]),
    tool("coin", "Coin flip", "Flips coins.", coin)
        .form()
        .slider("Flips", 1.0, 100.0, 1.0),
    tool(
        "random-choice",
        "Random choice",
        "Picks from a list: a decision, a winner, a spin of the wheel.",
        choice,
    )
    .text("Choices, one per line")
    .checks(&["Allow repeats"])
    .slider("How many", 1.0, 50.0, 1.0),
    tool(
        "shuffle",
        "List shuffler",
        "A list in random order: draft order, seating, turns.",
        shuffle,
    )
    .text("Items, one per line"),
    tool(
        "random-teams",
        "Team generator",
        "Splits people into random teams.",
        teams_of,
    )
    .text("Names, one per line")
    .slider("Teams", 2.0, 20.0, 2.0),
    tool(
        "secret-santa",
        "Secret Santa",
        "Who gives a present to whom.",
        secret_santa,
    )
    .text("Names, one per line"),
    tool(
        "lottery",
        "Lottery numbers",
        "A random draw of lottery numbers.",
        lottery,
    )
    .form()
    .fields(&[
        ("Numbers to draw", "6"),
        ("From 1 to", "49"),
        ("Bonus ball from 1 to (optional)", "10"),
    ]),
    tool("bingo", "Bingo cards", "Random 75-ball bingo cards.", bingo)
        .form()
        .slider("Cards", 1.0, 10.0, 1.0),
    tool(
        "bracket",
        "Tournament bracket",
        "First-round pairings for a knockout tournament.",
        bracket,
    )
    .text("Teams in seed order, one per line")
    .checks(&["Draw at random instead of by seed"]),
    tool(
        "mock-data",
        "Mock data generator",
        "Fake people records as JSON, CSV or SQL.",
        mock,
    )
    .form()
    .selects(&[(
        "Format",
        &[("json", "JSON"), ("csv", "CSV"), ("sql", "SQL inserts")],
    )])
    .slider("Records", 1.0, 100.0, 5.0),
    GENERATORS[0],
    GENERATORS[1],
    GENERATORS[2],
    GENERATORS[3],
    GENERATORS[4],
    GENERATORS[5],
    tool(
        "pet-name",
        "Pet name generator",
        "Random names for a pet.",
        pet_name,
    )
    .form()
    .slider("How many", 1.0, 30.0, 5.0),
];

pub const FUN: &[Tool] = &[
    tool(
        "eight-ball",
        "Magic 8-ball",
        "Ask a yes-or-no question, and shake.",
        eight_ball,
    )
    .form()
    .fields(&[("Your question", "Will it be sunny tomorrow?")]),
    tool(
        "fortune",
        "Fortune cookie",
        "A fortune and lucky numbers.",
        fortune,
    )
    .form(),
    tool("tarot", "Tarot reading", "Draws tarot cards.", tarot)
        .form()
        .selects(&[(
            "Spread",
            &[
                ("one", "One card"),
                ("three", "Past, present, future"),
                ("five", "Five cards"),
            ],
        )]),
    tool("runes", "Rune caster", "Casts Elder Futhark runes.", runes)
        .form()
        .slider("Runes", 1.0, 5.0, 3.0),
    tool(
        "numerology",
        "Numerology",
        "Life path and name numbers.",
        numerology,
    )
    .form()
    .fields(&[
        ("Birth date", "1990-05-15"),
        ("Full name (optional)", "Ada Lovelace"),
    ]),
    tool(
        "biorhythm",
        "Biorhythm",
        "Physical, emotional and intellectual cycles.",
        biorhythm,
    )
    .form()
    .fields(&[("Birth date", "1990-05-15"), ("On (empty: today)", "")]),
    tool(
        "life-weeks",
        "Life in weeks",
        "Your life as a grid of weeks.",
        life_weeks,
    )
    .form()
    .fields(&[
        ("Birth date", "1990-05-15"),
        ("On (empty: today)", ""),
        ("Expected years", "80"),
    ]),
    tool("t9", "T9 keypad", "Text as phone keypad digits.", t9).selects(&[(
        "Typing",
        &[("t9", "T9 (one press a letter)"), ("multitap", "Multi-tap")],
    )]),
    tool(
        "writing-prompt",
        "Writing prompt",
        "A random story idea.",
        prompt,
    )
    .form(),
];
