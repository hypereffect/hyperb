//! Everyday calculators: money, business, health, home, cooking, travel.

use super::Output;
use super::dates::{WEEKDAYS, clock, iso, weekday};
use super::{Opts, Tool, fmt, grid, prose, table, tool};

fn money(v: f64) -> String {
    fmt(v, 2)
}

fn pct(v: f64) -> String {
    format!("{}%", fmt(v, 2))
}

/// The payment of a loan of `p` at `rate` per period over `n` periods.
pub fn payment(p: f64, rate: f64, n: f64) -> f64 {
    if rate == 0.0 {
        p / n
    } else {
        p * rate / (1.0 - (1.0 + rate).powf(-n))
    }
}

/// The lines of a list as `name, number, number…`.
fn rows(text: &str, numbers: usize) -> Result<Vec<(String, Vec<f64>)>, String> {
    let mut out = Vec::new();
    for l in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let parts: Vec<&str> = l.split([',', ';', '\t', ':']).map(str::trim).collect();
        if parts.len() < numbers + 1 && !(numbers > 0 && parts.len() == numbers) {
            return Err(format!(
                "“{l}” needs a name and {numbers} number{}.",
                if numbers == 1 { "" } else { "s" }
            ));
        }
        let (name, nums) = if parts.len() == numbers {
            ("", &parts[..])
        } else {
            (parts[0], &parts[parts.len() - numbers..])
        };
        let nums: Vec<f64> = nums
            .iter()
            .map(|n| {
                super::parse_number(n).ok_or_else(|| format!("“{n}” is not a number in “{l}”."))
            })
            .collect::<Result<_, _>>()?;
        out.push((name.to_owned(), nums));
    }
    if out.is_empty() {
        return Err("The list is empty.".into());
    }
    Ok(out)
}

/// The numbers in a text, one or more a line.
pub fn numbers(text: &str) -> Result<Vec<f64>, String> {
    let v: Vec<f64> = text
        .split(|c: char| c.is_whitespace() || c == ';')
        .map(|w| w.trim_end_matches(','))
        .filter(|w| !w.is_empty())
        .map(|w| super::parse_number(w).ok_or_else(|| format!("“{w}” is not a number.")))
        .collect::<Result<_, _>>()?;
    if v.is_empty() {
        return Err("Write some numbers.".into());
    }
    Ok(v)
}

// Finance

fn loan(_: &str, o: &mut Opts) -> Result<Output, String> {
    let p = o.positive(0)?;
    let rate = o.num(1)? / 100.0 / 12.0;
    let n = (o.positive(2)? * 12.0).round();
    let m = payment(p, rate, n);
    Ok(table(&[
        ("Monthly payment", money(m)),
        ("Payments", fmt(n, 0)),
        ("Total paid", money(m * n)),
        ("Total interest", money(m * n - p)),
    ]))
}

fn mortgage(_: &str, o: &mut Opts) -> Result<Output, String> {
    let price = o.positive(0)?;
    let down = o.num(1)? / 100.0 * price;
    let rate = o.num(2)? / 100.0 / 12.0;
    let n = (o.positive(3)? * 12.0).round();
    let tax = o.opt_num(4)?.unwrap_or(0.0) / 12.0;
    let insurance = o.opt_num(5)?.unwrap_or(0.0) / 12.0;
    let principal = price - down;
    let m = payment(principal, rate, n);
    Ok(table(&[
        ("Loan", money(principal)),
        ("Principal and interest", money(m)),
        ("Monthly total", money(m + tax + insurance)),
        ("Total interest", money(m * n - principal)),
        ("Total cost", money(m * n + down + (tax + insurance) * n)),
    ]))
}

fn amortization(_: &str, o: &mut Opts) -> Result<Output, String> {
    let p = o.positive(0)?;
    let rate = o.num(1)? / 100.0 / 12.0;
    let n = (o.positive(2)? * 12.0).round() as usize;
    if n > 1200 {
        return Err("At most 100 years.".into());
    }
    let extra = o.opt_num(3)?.unwrap_or(0.0);
    let m = payment(p, rate, n as f64);
    let mut balance = p;
    let mut facts = vec![("Monthly payment", money(m))];
    if extra > 0.0 {
        facts.push(("Extra each month", money(extra)));
    }
    let yearly = o.mode != "monthly";
    let mut rows = Vec::new();
    let (mut yp, mut yi, mut total_interest) = (0.0, 0.0, 0.0);
    for k in 1..=n {
        let interest = balance * rate;
        let principal = (m + extra - interest).min(balance);
        balance -= principal;
        yp += principal;
        yi += interest;
        total_interest += interest;
        let done = balance < 0.005;
        if !yearly || k % 12 == 0 || done {
            rows.push(vec![
                (if yearly { k.div_ceil(12) } else { k }).to_string(),
                money(yp),
                money(yi),
                money(balance.max(0.0)),
            ]);
            yp = 0.0;
            yi = 0.0;
        }
        if done {
            facts.push(("Paid off after", format!("{k} months")));
            facts.push(("Total interest", money(total_interest)));
            break;
        }
    }
    Ok(table(&facts).table(
        &[
            if yearly { "Year" } else { "Month" },
            "Principal",
            "Interest",
            "Balance",
        ],
        rows,
    ))
}

fn compound(_: &str, o: &mut Opts) -> Result<Output, String> {
    let p = o.num(0)?;
    let r = o.num(1)? / 100.0;
    let years = o.positive(2)?;
    let monthly = o.opt_num(3)?.unwrap_or(0.0);
    let k: f64 = match o.mode {
        "daily" => 365.0,
        "monthly" => 12.0,
        "quarterly" => 4.0,
        _ => 1.0,
    };
    // Monthly contributions, compounding k times a year.
    let months = (years * 12.0).round() as usize;
    let mut balance = p;
    let mut lines = Vec::new();
    for mth in 1..=months {
        let months_per_period = 12.0 / k;
        balance += monthly;
        if k >= 12.0 {
            balance *= (1.0 + r / k).powf(k / 12.0);
        } else if (mth as f64 % months_per_period).abs() < 1e-9 {
            balance *= 1.0 + r / k;
        }
        if mth % 12 == 0 {
            lines.push(vec![(mth / 12).to_string(), money(balance)]);
        }
    }
    let contributed = p + monthly * months as f64;
    let out = table(&[
        ("Final balance", money(balance)),
        ("Contributed", money(contributed)),
        ("Interest earned", money(balance - contributed)),
        (
            "Effective annual rate",
            pct(((1.0 + r / k).powf(k) - 1.0) * 100.0),
        ),
    ]);
    Ok(if lines.len() > 1 {
        out.table(&["Year", "Balance"], lines)
    } else {
        out
    })
}

fn simple_interest(_: &str, o: &mut Opts) -> Result<Output, String> {
    let p = o.num(0)?;
    let r = o.num(1)? / 100.0;
    let t = o.num(2)?;
    let i = p * r * t;
    Ok(table(&[("Interest", money(i)), ("Total", money(p + i))]))
}

fn effective_rate(_: &str, o: &mut Opts) -> Result<Output, String> {
    let r = o.num(0)? / 100.0;
    let rows: Vec<(&str, String)> = [
        ("Yearly", 1.0),
        ("Quarterly", 4.0),
        ("Monthly", 12.0),
        ("Daily", 365.0),
    ]
    .iter()
    .map(|(n, k)| (*n, pct(((1.0 + r / k).powf(*k) - 1.0) * 100.0)))
    .chain(std::iter::once((
        "Continuous",
        pct((r.exp() - 1.0) * 100.0),
    )))
    .collect();
    Ok(prose(format!(
        "Effective annual rate (APY) of a {} nominal rate, compounded:",
        pct(r * 100.0)
    ))
    .then(table(&rows)))
}

fn savings_goal(_: &str, o: &mut Opts) -> Result<Output, String> {
    let goal = o.positive(0)?;
    let have = o.opt_num(1)?.unwrap_or(0.0);
    let r = o.opt_num(2)?.unwrap_or(0.0) / 100.0 / 12.0;
    let n = (o.positive(3)? * 12.0).round();
    let grown = have * (1.0 + r).powf(n);
    let need = if r == 0.0 {
        (goal - grown) / n
    } else {
        (goal - grown) * r / ((1.0 + r).powf(n) - 1.0)
    };
    Ok(table(&[
        ("Save each month", money(need.max(0.0))),
        ("Your savings grow to", money(grown)),
        ("Total deposits", money(need.max(0.0) * n)),
    ]))
}

fn retirement(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (age, retire) = (o.num(0)?, o.num(1)?);
    if retire <= age {
        return Err("The retirement age must be after the current age.".into());
    }
    let mut balance = o.num(2)?;
    let monthly = o.num(3)?;
    let r = o.num(4)? / 100.0 / 12.0;
    let withdrawal = o.opt_num(5)?.unwrap_or(4.0) / 100.0;
    for _ in 0..((retire - age) * 12.0).round() as usize {
        balance = balance * (1.0 + r) + monthly;
    }
    Ok(table(&[
        ("At retirement", money(balance)),
        ("Yearly income", money(balance * withdrawal)),
        ("Monthly income", money(balance * withdrawal / 12.0)),
        (
            "",
            format!(
                "At a {} withdrawal rate, in today's money only if the return is after inflation.",
                pct(withdrawal * 100.0)
            ),
        ),
    ]))
}

fn fire(_: &str, o: &mut Opts) -> Result<Output, String> {
    let spend = o.positive(0)?;
    let mut have = o.num(1)?;
    let save = o.num(2)?;
    let r = o.num(3)? / 100.0;
    let swr = o.opt_num(4)?.unwrap_or(4.0) / 100.0;
    let target = spend / swr;
    let mut years = 0;
    while have < target && years < 100 {
        have = have * (1.0 + r) + save;
        years += 1;
    }
    Ok(table(&[
        ("FI number", money(target)),
        (
            "Years to get there",
            if years >= 100 {
                "more than 100".into()
            } else {
                years.to_string()
            },
        ),
        ("Savings then", money(have)),
    ]))
}

fn debt_payoff(_: &str, o: &mut Opts) -> Result<Output, String> {
    let mut balance = o.positive(0)?;
    let r = o.num(1)? / 100.0 / 12.0;
    let pay = o.positive(2)?;
    if pay <= balance * r {
        return Err(format!(
            "The payment must be more than the monthly interest, {}.",
            money(balance * r)
        ));
    }
    let (mut months, mut interest) = (0, 0.0);
    while balance > 0.005 && months < 1200 {
        let i = balance * r;
        interest += i;
        balance = balance + i - pay;
        months += 1;
    }
    Ok(table(&[
        (
            "Months to pay off",
            format!(
                "{months} ({})",
                super::dates::duration_words(months as f64 * 30.44 * 86400.0)
            ),
        ),
        ("Total interest", money(interest)),
        ("Total paid", money(o.num(0)? + interest)),
    ]))
}

fn inflation(_: &str, o: &mut Opts) -> Result<Output, String> {
    let amount = o.num(0)?;
    let r = o.num(1)? / 100.0;
    let years = o.num(2)?;
    let f = (1.0 + r).powf(years);
    Ok(table(&[
        (
            "Prices then",
            format!("{} costs {}", money(amount), money(amount * f)),
        ),
        (
            "Buying power",
            format!("{} buys what {} buys now", money(amount), money(amount / f)),
        ),
        ("Total inflation", pct((f - 1.0) * 100.0)),
    ]))
}

fn npv(text: &str, o: &mut Opts) -> Result<Output, String> {
    let flows = numbers(text)?;
    let r = o.num(0)? / 100.0;
    let v: f64 = flows
        .iter()
        .enumerate()
        .map(|(t, c)| c / (1.0 + r).powi(t as i32))
        .sum();
    Ok(table(&[
        ("Net present value", money(v)),
        ("Cash flows", flows.len().to_string()),
    ]))
}

pub fn irr_of(flows: &[f64]) -> Option<f64> {
    let npv = |r: f64| {
        flows
            .iter()
            .enumerate()
            .map(|(t, c)| c / (1.0 + r).powi(t as i32))
            .sum::<f64>()
    };
    // Bisection between -99% and 1000%.
    let (mut lo, mut hi) = (-0.99, 10.0);
    if npv(lo).signum() == npv(hi).signum() {
        return None;
    }
    for _ in 0..200 {
        let mid = (lo + hi) / 2.0;
        if npv(lo).signum() == npv(mid).signum() {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some((lo + hi) / 2.0)
}

fn irr(text: &str, _: &mut Opts) -> Result<Output, String> {
    let flows = numbers(text)?;
    let r = irr_of(&flows).ok_or(
        "No rate makes the cash flows add up to zero: they need both payments and receipts.",
    )?;
    Ok(table(&[
        ("Internal rate of return", pct(r * 100.0)),
        ("Cash flows", flows.len().to_string()),
    ]))
}

fn roi(_: &str, o: &mut Opts) -> Result<Output, String> {
    let cost = o.positive(0)?;
    let value = o.num(1)?;
    let years = o.opt_num(2)?;
    let gain = value - cost;
    let mut r = vec![("Gain", money(gain)), ("ROI", pct(gain / cost * 100.0))];
    if let Some(y) = years.filter(|y| *y > 0.0) {
        r.push((
            "Annualized",
            pct(((value / cost).powf(1.0 / y) - 1.0) * 100.0),
        ));
    }
    Ok(table(&r))
}

fn break_even(_: &str, o: &mut Opts) -> Result<Output, String> {
    let fixed = o.num(0)?;
    let price = o.num(1)?;
    let var = o.num(2)?;
    if price <= var {
        return Err("The price must be above the variable cost, or no amount breaks even.".into());
    }
    let units = fixed / (price - var);
    Ok(table(&[
        ("Break-even units", fmt(units.ceil(), 0)),
        ("Break-even sales", money(units * price)),
        (
            "Contribution margin",
            format!(
                "{} a unit ({})",
                money(price - var),
                pct((price - var) / price * 100.0)
            ),
        ),
    ]))
}

fn margin(_: &str, o: &mut Opts) -> Result<Output, String> {
    let cost = o.num(0)?;
    let price = o.positive(1)?;
    let profit = price - cost;
    Ok(table(&[
        ("Profit", money(profit)),
        ("Gross margin", pct(profit / price * 100.0)),
        (
            "Markup",
            if cost > 0.0 {
                pct(profit / cost * 100.0)
            } else {
                "—".into()
            },
        ),
    ]))
}

fn discount(_: &str, o: &mut Opts) -> Result<Output, String> {
    let price = o.num(0)?;
    let d = o.num(1)? / 100.0;
    let extra = o.opt_num(2)?.unwrap_or(0.0) / 100.0;
    let tax = o.opt_num(3)?.unwrap_or(0.0) / 100.0;
    let after = price * (1.0 - d) * (1.0 - extra);
    Ok(table(&[
        ("Price after discounts", money(after)),
        (
            "You save",
            format!(
                "{} ({})",
                money(price - after),
                pct((1.0 - after / price) * 100.0)
            ),
        ),
        ("With tax", money(after * (1.0 + tax))),
    ]))
}

fn vat(_: &str, o: &mut Opts) -> Result<Output, String> {
    let amount = o.num(0)?;
    let r = o.num(1)? / 100.0;
    let (net, gross) = if o.mode == "remove" {
        (amount / (1.0 + r), amount)
    } else {
        (amount, amount * (1.0 + r))
    };
    Ok(table(&[
        ("Net", money(net)),
        ("Tax", money(gross - net)),
        ("Gross", money(gross)),
    ]))
}

fn salary(_: &str, o: &mut Opts) -> Result<Output, String> {
    let amount = o.positive(0)?;
    let hours = o.positive(1)?;
    let weeks = o.positive(2)?;
    let yearly = match o.mode {
        "hour" => amount * hours * weeks,
        "day" => amount * 5.0 * weeks,
        "week" => amount * weeks,
        "month" => amount * 12.0,
        _ => amount,
    };
    Ok(table(&[
        ("Hourly", money(yearly / weeks / hours)),
        ("Daily", money(yearly / weeks / 5.0)),
        ("Weekly", money(yearly / weeks)),
        ("Monthly", money(yearly / 12.0)),
        ("Yearly", money(yearly)),
    ]))
}

fn crypto_profit(_: &str, o: &mut Opts) -> Result<Output, String> {
    let invested = o.positive(0)?;
    let buy = o.positive(1)?;
    let sell = o.positive(2)?;
    let fee = o.opt_num(3)?.unwrap_or(0.0) / 100.0;
    let coins = invested * (1.0 - fee) / buy;
    let out = coins * sell * (1.0 - fee);
    Ok(table(&[
        ("Coins bought", fmt(coins, 8)),
        ("Value when sold", money(out)),
        ("Profit", money(out - invested)),
        ("Return", pct((out / invested - 1.0) * 100.0)),
    ]))
}

fn invoice(text: &str, o: &mut Opts) -> Result<Output, String> {
    let items = rows(text, 2)?;
    let tax = o.opt_num(0)?.unwrap_or(0.0) / 100.0;
    let disc = o.opt_num(1)?.unwrap_or(0.0) / 100.0;
    let mut lines = Vec::new();
    let mut sub = 0.0;
    for (name, n) in &items {
        let amount = n[0] * n[1];
        sub += amount;
        lines.push(vec![name.clone(), fmt(n[0], 2), money(n[1]), money(amount)]);
    }
    let d = sub * disc;
    let t = (sub - d) * tax;
    Ok(grid(&["Item", "Qty", "Price", "Amount"], lines).facts(&[
        ("Subtotal", money(sub)),
        ("Discount", money(-d)),
        ("Tax", money(t)),
        ("Total", money(sub - d + t)),
    ]))
}

fn subscriptions(text: &str, _: &mut Opts) -> Result<Output, String> {
    let mut monthly = 0.0;
    let mut out = Vec::new();
    for l in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let lower = l.to_lowercase();
        let per = if lower.contains("year") || lower.contains("annual") {
            1.0 / 12.0
        } else if lower.contains("week") {
            52.0 / 12.0
        } else {
            1.0
        };
        let price = lower
            .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == ','))
            .filter_map(super::parse_number)
            .next()
            .ok_or_else(|| format!("“{l}” has no price."))?;
        monthly += price * per;
        out.push(vec![
            l.split([',', ':']).next().unwrap_or(l).trim().to_owned(),
            money(price * per),
        ]);
    }
    Ok(grid(&["Subscription", "A month"], out).facts(&[
        ("Monthly", money(monthly)),
        ("Yearly", money(monthly * 12.0)),
        ("Daily", money(monthly * 12.0 / 365.0)),
    ]))
}

fn net_worth(text: &str, _: &mut Opts) -> Result<Output, String> {
    let items = rows(text, 1)?;
    let assets: f64 = items.iter().map(|i| i.1[0]).filter(|v| *v > 0.0).sum();
    let debts: f64 = -items
        .iter()
        .map(|i| i.1[0])
        .filter(|v| *v < 0.0)
        .sum::<f64>();
    Ok(table(&[
        ("Assets", money(assets)),
        ("Liabilities", money(debts)),
        ("Net worth", money(assets - debts)),
        (
            "Debt to assets",
            if assets > 0.0 {
                pct(debts / assets * 100.0)
            } else {
                "—".into()
            },
        ),
    ]))
}

fn emergency_fund(_: &str, o: &mut Opts) -> Result<Output, String> {
    let expenses = o.positive(0)?;
    let months = o.positive(1)?;
    let have = o.opt_num(2)?.unwrap_or(0.0);
    let save = o.opt_num(3)?.unwrap_or(0.0);
    let target = expenses * months;
    let gap = (target - have).max(0.0);
    let mut r = vec![
        ("Target", money(target)),
        ("Still needed", money(gap)),
        ("Covered now", format!("{} months", fmt(have / expenses, 1))),
    ];
    if save > 0.0 && gap > 0.0 {
        r.push(("Months to reach it", fmt((gap / save).ceil(), 0)));
    }
    Ok(table(&r))
}

fn rent_vs_buy(_: &str, o: &mut Opts) -> Result<Output, String> {
    let price = o.positive(0)?;
    let down = o.num(1)? / 100.0 * price;
    let rate = o.num(2)? / 100.0;
    let years = o.positive(3)?;
    let rent = o.positive(4)?;
    let growth = o.opt_num(5)?.unwrap_or(3.0) / 100.0;
    let invest = o.opt_num(6)?.unwrap_or(5.0) / 100.0;
    // Buying: mortgage over 30 years, 1.5% a year of taxes and upkeep, the
    // home growing with rents, 6% to sell. Renting: the down payment and
    // any monthly difference are invested.
    let n = 360.0;
    let m = payment(price - down, rate / 12.0, n);
    let (mut balance, mut renter, mut rent_now, mut buy_paid, mut rent_paid) =
        (price - down, down, rent, down, 0.0);
    for _ in 0..(years * 12.0) as usize {
        let own = m + price * 0.015 / 12.0;
        balance -= m - balance * rate / 12.0;
        buy_paid += own;
        rent_paid += rent_now;
        renter = renter * (1.0 + invest / 12.0) + (own - rent_now).max(0.0);
        rent_now *= (1.0 + growth).powf(1.0 / 12.0);
    }
    let home = price * (1.0 + growth).powf(years);
    let equity = home * 0.94 - balance.max(0.0);
    let verdict = if equity > renter {
        "Buying comes out ahead"
    } else {
        "Renting comes out ahead"
    };
    Ok(table(&[
        ("After", format!("{} years", fmt(years, 0))),
        ("Owner's equity after selling", money(equity)),
        ("Renter's investments", money(renter)),
        ("Paid by the owner", money(buy_paid)),
        ("Paid in rent", money(rent_paid)),
        (
            "",
            format!("{verdict}, by {}.", money((equity - renter).abs())),
        ),
    ]))
}

fn late_payment(_: &str, o: &mut Opts) -> Result<Output, String> {
    let amount = o.positive(0)?;
    let rate = o.num(1)? / 100.0;
    let days = o.num(2)?;
    let fee = o.opt_num(3)?.unwrap_or(0.0);
    let interest = amount * rate * days / 365.0;
    Ok(table(&[
        ("Interest", money(interest)),
        ("Fixed fee", money(fee)),
        ("Total due", money(amount + interest + fee)),
    ]))
}

fn currency_format(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.num(0)?;
    let symbol = o.field(1);
    let digits = o.opt_num(2)?.unwrap_or(2.0).clamp(0.0, 8.0) as usize;
    let raw = format!("{:.*}", digits, v.abs());
    let (whole, frac) = raw.split_once('.').unwrap_or((&raw, ""));
    let group3 = |sep: &str| crate::common::group(whole).replace(',', sep);
    let indian = || {
        if whole.len() <= 3 {
            return whole.to_owned();
        }
        let (head, last) = whole.split_at(whole.len() - 3);
        let mut parts: Vec<String> = Vec::new();
        let mut h = head;
        while h.len() > 2 {
            parts.insert(0, h[h.len() - 2..].to_owned());
            h = &h[..h.len() - 2];
        }
        if !h.is_empty() {
            parts.insert(0, h.to_owned());
        }
        format!("{},{last}", parts.join(","))
    };
    let (num, dec) = match o.mode {
        "eu" => (group3("."), ","),
        "fr" => (group3("\u{202f}"), ","),
        "ch" => (group3("’"), "."),
        "in" => (indian(), "."),
        _ => (group3(","), "."),
    };
    let n = if frac.is_empty() {
        num
    } else {
        format!("{num}{dec}{frac}")
    };
    let sign = if v < 0.0 { "-" } else { "" };
    Ok(match o.choice(1) {
        "after" => format!("{sign}{n} {symbol}"),
        "code" => format!("{sign}{n} {}", symbol.to_uppercase()),
        _ => format!("{sign}{symbol}{n}"),
    }
    .trim()
    .into())
}

fn tip(_: &str, o: &mut Opts) -> Result<Output, String> {
    let bill = o.positive(0)?;
    let t = o.num(1)? / 100.0;
    let people = o.opt_num(2)?.unwrap_or(1.0).max(1.0);
    let total = bill * (1.0 + t);
    let mut r = vec![("Tip", money(bill * t)), ("Total", money(total))];
    if people > 1.0 {
        r.push(("Each pays", money(total / people)));
        r.push(("Each, rounded up", money((total / people).ceil())));
    }
    Ok(table(&r))
}

// Business and real estate

fn cac(_: &str, o: &mut Opts) -> Result<Output, String> {
    let spend = o.num(0)? + o.opt_num(1)?.unwrap_or(0.0);
    let customers = o.positive(2)?;
    Ok(table(&[
        ("Customer acquisition cost", money(spend / customers)),
        ("Total spend", money(spend)),
    ]))
}

fn churn(_: &str, o: &mut Opts) -> Result<Output, String> {
    let start = o.positive(0)?;
    let end = o.num(1)?;
    let new = o.opt_num(2)?.unwrap_or(0.0);
    let lost = start + new - end;
    let rate = lost / start;
    Ok(table(&[
        ("Customers lost", fmt(lost, 0)),
        ("Churn rate", pct(rate * 100.0)),
        ("Retention rate", pct((1.0 - rate) * 100.0)),
        (
            "Average lifetime",
            if rate > 0.0 {
                format!("{} periods", fmt(1.0 / rate, 1))
            } else {
                "—".into()
            },
        ),
    ]))
}

fn conversion(_: &str, o: &mut Opts) -> Result<Output, String> {
    let visitors = o.positive(0)?;
    let conv = o.num(1)?;
    let value = o.opt_num(2)?;
    let mut r = vec![("Conversion rate", pct(conv / visitors * 100.0))];
    if let Some(v) = value {
        r.push(("Revenue", money(conv * v)));
        r.push(("Value per visitor", money(conv * v / visitors)));
    }
    Ok(table(&r))
}

fn ltv(_: &str, o: &mut Opts) -> Result<Output, String> {
    let arpu = o.positive(0)?;
    let margin = o.opt_num(1)?.unwrap_or(100.0) / 100.0;
    let churn = o.positive(2)? / 100.0;
    let value = arpu * margin / churn;
    let mut r = vec![
        ("Lifetime value", money(value)),
        (
            "Average lifetime",
            format!("{} months", fmt(1.0 / churn, 1)),
        ),
    ];
    if let Some(cac) = o.opt_num(3)?.filter(|c| *c > 0.0) {
        r.push((
            "LTV : CAC",
            format!("{}:1 (3:1 or more is healthy)", fmt(value / cac, 1)),
        ));
        r.push(("Months to recover CAC", fmt(cac / (arpu * margin), 1)));
    }
    Ok(table(&r))
}

fn unit_economics(_: &str, o: &mut Opts) -> Result<Output, String> {
    let price = o.positive(0)?;
    let cogs = o.num(1)?;
    let cac = o.num(2)?;
    let churn = o.positive(3)? / 100.0;
    let contrib = price - cogs;
    let ltv = contrib / churn;
    Ok(table(&[
        (
            "Contribution margin",
            format!("{} ({})", money(contrib), pct(contrib / price * 100.0)),
        ),
        ("Lifetime value", money(ltv)),
        ("LTV : CAC", format!("{}:1", fmt(ltv / cac.max(1e-9), 2))),
        ("Payback", format!("{} months", fmt(cac / contrib, 1))),
    ]))
}

fn freelance_rate(_: &str, o: &mut Opts) -> Result<Output, String> {
    let income = o.positive(0)?;
    let expenses = o.opt_num(1)?.unwrap_or(0.0);
    let tax = o.opt_num(2)?.unwrap_or(0.0) / 100.0;
    let hours = o.positive(3)?;
    let weeks = o.positive(4)?;
    let gross = (income / (1.0 - tax)) + expenses;
    let rate = gross / (hours * weeks);
    Ok(table(&[
        ("Hourly rate", money(rate)),
        ("Day rate (8 h)", money(rate * 8.0)),
        ("Revenue needed", money(gross)),
        ("Billable hours a year", fmt(hours * weeks, 0)),
    ]))
}

fn meeting_cost(_: &str, o: &mut Opts) -> Result<Output, String> {
    let people = o.positive(0)?;
    let salary = o.positive(1)?;
    let minutes = o.positive(2)?;
    let hourly = salary / 2080.0 * 1.3;
    let cost = people * hourly * minutes / 60.0;
    Ok(table(&[
        ("Cost of the meeting", money(cost)),
        ("Per minute", money(cost / minutes)),
        ("Held weekly, a year", money(cost * 52.0)),
        (
            "",
            "With 2,080 working hours a year and 30% on top of salary for benefits.".into(),
        ),
    ]))
}

fn quote(text: &str, o: &mut Opts) -> Result<Output, String> {
    let tasks = rows(text, 1)?;
    let rate = o.positive(0)?;
    let buffer = o.opt_num(1)?.unwrap_or(0.0) / 100.0;
    let mut out = Vec::new();
    let mut hours = 0.0;
    for (name, h) in &tasks {
        hours += h[0];
        out.push(vec![
            name.clone(),
            format!("{} h", fmt(h[0], 1)),
            money(h[0] * rate),
        ]);
    }
    Ok(grid(&["Task", "Hours", "Cost"], out).facts(&[
        ("Hours", fmt(hours, 1)),
        ("Subtotal", money(hours * rate)),
        ("Contingency", money(hours * rate * buffer)),
        ("Quote", money(hours * rate * (1.0 + buffer))),
    ]))
}

fn pert(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (a, m, b) = (o.num(0)?, o.num(1)?, o.num(2)?);
    if !(a <= m && m <= b) {
        return Err("Optimistic ≤ likely ≤ pessimistic.".into());
    }
    let e = (a + 4.0 * m + b) / 6.0;
    let sd = (b - a) / 6.0;
    Ok(table(&[
        ("Expected (PERT)", fmt(e, 2)),
        ("Standard deviation", fmt(sd, 2)),
        (
            "68% range",
            format!("{} – {}", fmt(e - sd, 2), fmt(e + sd, 2)),
        ),
        (
            "95% range",
            format!("{} – {}", fmt(e - 2.0 * sd, 2), fmt(e + 2.0 * sd, 2)),
        ),
    ]))
}

fn cap_rate(_: &str, o: &mut Opts) -> Result<Output, String> {
    let price = o.positive(0)?;
    let rent = o.num(1)? * 12.0;
    let expenses = o.opt_num(2)?.unwrap_or(0.0);
    let noi = rent - expenses;
    Ok(table(&[
        ("Net operating income", money(noi)),
        ("Cap rate", pct(noi / price * 100.0)),
    ]))
}

fn cash_flow(_: &str, o: &mut Opts) -> Result<Output, String> {
    let rent = o.positive(0)?;
    let mortgage = o.num(1)?;
    let fixed = o.opt_num(2)?.unwrap_or(0.0);
    let vacancy = o.opt_num(3)?.unwrap_or(5.0) / 100.0;
    let upkeep = o.opt_num(4)?.unwrap_or(10.0) / 100.0;
    let mgmt = o.opt_num(5)?.unwrap_or(0.0) / 100.0;
    let income = rent * (1.0 - vacancy);
    let costs = mortgage + fixed + rent * (upkeep + mgmt);
    Ok(table(&[
        ("Effective income", money(income)),
        ("Expenses", money(costs)),
        ("Monthly cash flow", money(income - costs)),
        ("Yearly cash flow", money((income - costs) * 12.0)),
    ]))
}

fn rental_yield(_: &str, o: &mut Opts) -> Result<Output, String> {
    let price = o.positive(0)?;
    let rent = o.positive(1)? * 12.0;
    let costs = o.opt_num(2)?.unwrap_or(0.0);
    Ok(table(&[
        ("Gross yield", pct(rent / price * 100.0)),
        ("Net yield", pct((rent - costs) / price * 100.0)),
        ("Price to rent", fmt(price / rent, 1)),
    ]))
}

fn affordability(_: &str, o: &mut Opts) -> Result<Output, String> {
    let income = o.positive(0)? / 12.0;
    let debts = o.opt_num(1)?.unwrap_or(0.0);
    let down = o.opt_num(2)?.unwrap_or(0.0);
    let rate = o.num(3)? / 100.0 / 12.0;
    let years = o.positive(4)?;
    let extra = o.opt_num(5)?.unwrap_or(1.5) / 100.0 / 12.0;
    // The 28/36 rule: housing at most 28% of income, all debt at most 36%.
    let budget = (income * 0.28).min(income * 0.36 - debts);
    if budget <= 0.0 {
        return Err("The debts already take 36% of the income.".into());
    }
    let n = years * 12.0;
    let factor = payment(1.0, rate, n);
    // The payment on the loan plus taxes and insurance on the price.
    let price = (budget + down * factor) / (factor + extra);
    Ok(table(&[
        ("Home price", money(price)),
        ("Loan", money(price - down)),
        ("Monthly housing budget", money(budget)),
        (
            "",
            "By the 28/36 rule, with taxes and insurance as a share of the price.".into(),
        ),
    ]))
}

fn flip(_: &str, o: &mut Opts) -> Result<Output, String> {
    let arv = o.positive(0)?;
    let repairs = o.num(1)?;
    let price = o.opt_num(2)?;
    let holding = o.opt_num(3)?.unwrap_or(0.0);
    let selling = o.opt_num(4)?.unwrap_or(8.0) / 100.0;
    let mut r = vec![("Most to offer (70% rule)", money(arv * 0.7 - repairs))];
    if let Some(p) = price {
        let profit = arv * (1.0 - selling) - p - repairs - holding;
        r.push(("Profit", money(profit)));
        r.push((
            "Return on cost",
            pct(profit / (p + repairs + holding) * 100.0),
        ));
    }
    Ok(table(&r))
}

fn closing_costs(_: &str, o: &mut Opts) -> Result<Output, String> {
    let price = o.positive(0)?;
    let down = o.opt_num(1)?.unwrap_or(20.0) / 100.0;
    let loan = price * (1.0 - down);
    Ok(table(&[
        ("Low (2%)", money(price * 0.02)),
        ("Typical (3.5%)", money(price * 0.035)),
        ("High (5%)", money(price * 0.05)),
        ("Loan origination (1%)", money(loan * 0.01)),
        (
            "Cash to close (typical)",
            money(price * down + price * 0.035),
        ),
    ]))
}

fn property_tax(_: &str, o: &mut Opts) -> Result<Output, String> {
    let value = o.positive(0)?;
    let rate = o.num(1)?;
    let exemption = o.opt_num(2)?.unwrap_or(0.0);
    let taxable = (value - exemption).max(0.0);
    let yearly = if o.mode == "mill" {
        taxable * rate / 1000.0
    } else {
        taxable * rate / 100.0
    };
    Ok(table(&[
        ("Yearly", money(yearly)),
        ("Monthly", money(yearly / 12.0)),
        ("Taxable value", money(taxable)),
    ]))
}

fn rent_increase(_: &str, o: &mut Opts) -> Result<Output, String> {
    let rent = o.positive(0)?;
    let v = o.num(1)?;
    let (new, p) = if o.mode == "amount" {
        (v, (v / rent - 1.0) * 100.0)
    } else {
        (rent * (1.0 + v / 100.0), v)
    };
    Ok(table(&[
        ("New rent", money(new)),
        ("Increase", format!("{} ({})", money(new - rent), pct(p))),
        ("More a year", money((new - rent) * 12.0)),
    ]))
}

fn rental_depreciation(_: &str, o: &mut Opts) -> Result<Output, String> {
    let basis = o.positive(0)? - o.opt_num(1)?.unwrap_or(0.0);
    let life = if o.mode == "commercial" { 39.0 } else { 27.5 };
    Ok(table(&[
        ("Depreciable basis", money(basis)),
        ("Recovery period", format!("{life} years")),
        ("Yearly depreciation", money(basis / life)),
        ("Monthly", money(basis / life / 12.0)),
    ]))
}

fn vacancy(_: &str, o: &mut Opts) -> Result<Output, String> {
    let total = o.positive(0)?;
    let vacant = o.num(1)?;
    let rent = o.opt_num(2)?;
    let rate = vacant / total;
    let mut r = vec![
        ("Vacancy rate", pct(rate * 100.0)),
        ("Occupancy rate", pct((1.0 - rate) * 100.0)),
    ];
    if let Some(rent) = rent {
        r.push(("Lost rent a month", money(vacant * rent)));
    }
    Ok(table(&r))
}

fn repair_replace(_: &str, o: &mut Opts) -> Result<Output, String> {
    let repair = o.positive(0)?;
    let replace = o.positive(1)?;
    let age = o.num(2)?;
    let life = o.positive(3)?;
    let score = repair * age;
    let verdict = if repair > replace * 0.5 || score > replace {
        "Replace it: the repair costs more than half a new one, or repair × age exceeds the price."
    } else {
        "Repair it."
    };
    Ok(table(&[
        ("Life used", pct(age / life * 100.0)),
        ("Repair ÷ replacement", pct(repair / replace * 100.0)),
        ("Repair × age", money(score)),
        ("", verdict.into()),
    ]))
}

fn brrrr(_: &str, o: &mut Opts) -> Result<Output, String> {
    let buy = o.positive(0)?;
    let rehab = o.num(1)?;
    let arv = o.positive(2)?;
    let ltv = o.opt_num(3)?.unwrap_or(75.0) / 100.0;
    let loan = arv * ltv;
    let invested = buy + rehab;
    Ok(table(&[
        ("Refinance loan", money(loan)),
        ("Cash invested", money(invested)),
        ("Cash left in the deal", money((invested - loan).max(0.0))),
        (
            "Cash pulled out beyond cost",
            money((loan - invested).max(0.0)),
        ),
        ("Equity", money(arv - loan)),
    ]))
}

fn renovation_roi(_: &str, o: &mut Opts) -> Result<Output, String> {
    let cost = o.positive(0)?;
    let added = o.num(1)?;
    Ok(table(&[
        ("Return", pct((added - cost) / cost * 100.0)),
        ("Recouped", pct(added / cost * 100.0)),
        ("Net", money(added - cost)),
    ]))
}

// Health

fn metric(o: &Opts, w: usize, h: usize) -> Result<(f64, f64), String> {
    let (wt, ht) = (o.positive(w)?, o.positive(h)?);
    Ok(if o.mode == "imperial" {
        (wt * 0.45359237, ht * 2.54)
    } else {
        (wt, ht)
    })
}

const UNITS: super::Menu = &[
    ("metric", "Metric (kg, cm)"),
    ("imperial", "Imperial (lb, in)"),
];
const SEX: super::Menu = &[("female", "Female"), ("male", "Male")];

fn bmi(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (kg, cm) = metric(o, 0, 1)?;
    let m = cm / 100.0;
    let b = kg / (m * m);
    let class = match b {
        b if b < 18.5 => "underweight",
        b if b < 25.0 => "normal weight",
        b if b < 30.0 => "overweight",
        _ => "obese",
    };
    Ok(table(&[
        ("BMI", format!("{b:.1} — {class}")),
        (
            "Normal range for this height",
            format!("{} – {} kg", fmt(18.5 * m * m, 1), fmt(24.9 * m * m, 1)),
        ),
        (
            "",
            "BMI is a rough screen, and says nothing of muscle or fat.".into(),
        ),
    ]))
}

fn mifflin(o: &Opts) -> Result<f64, String> {
    let (kg, cm) = metric(o, 1, 2)?;
    let age = o.positive(0)?;
    Ok(10.0 * kg + 6.25 * cm - 5.0 * age + if o.choice(1) == "male" { 5.0 } else { -161.0 })
}

const ACTIVITY: super::Menu = &[
    ("1.2", "Sedentary"),
    ("1.375", "Light exercise (1–3 days a week)"),
    ("1.55", "Moderate (3–5 days)"),
    ("1.725", "Hard (6–7 days)"),
    ("1.9", "Very hard, or a physical job"),
];

fn bmr(_: &str, o: &mut Opts) -> Result<Output, String> {
    let b = mifflin(o)?;
    Ok(table(&[
        ("BMR (Mifflin–St Jeor)", format!("{} kcal a day", fmt(b, 0))),
        ("", "The energy used at complete rest.".into()),
    ]))
}

fn tdee(_: &str, o: &mut Opts) -> Result<Output, String> {
    let b = mifflin(o)?;
    let f: f64 = o.choice(2).parse().unwrap_or(1.2);
    let t = b * f;
    Ok(table(&[
        ("BMR", format!("{} kcal", fmt(b, 0))),
        ("Maintenance (TDEE)", format!("{} kcal", fmt(t, 0))),
        ("Lose 0.5 kg a week", format!("{} kcal", fmt(t - 550.0, 0))),
        ("Lose 0.25 kg a week", format!("{} kcal", fmt(t - 275.0, 0))),
        ("Gain 0.25 kg a week", format!("{} kcal", fmt(t + 275.0, 0))),
    ]))
}

fn macros(_: &str, o: &mut Opts) -> Result<Output, String> {
    let kcal = o.positive(0)?;
    let (p, c, f) = match o.mode {
        "low-carb" => (0.3, 0.2, 0.5),
        "high-protein" => (0.4, 0.3, 0.3),
        "keto" => (0.2, 0.05, 0.75),
        _ => (0.25, 0.5, 0.25),
    };
    Ok(table(&[
        (
            "Protein",
            format!("{} g ({}%)", fmt(kcal * p / 4.0, 0), p * 100.0),
        ),
        (
            "Carbohydrates",
            format!("{} g ({}%)", fmt(kcal * c / 4.0, 0), c * 100.0),
        ),
        (
            "Fat",
            format!("{} g ({}%)", fmt(kcal * f / 9.0, 0), f * 100.0),
        ),
    ]))
}

fn body_fat(_: &str, o: &mut Opts) -> Result<Output, String> {
    let k = if o.mode == "imperial" { 2.54 } else { 1.0 };
    let height = o.positive(0)? * k;
    let neck = o.positive(1)? * k;
    let waist = o.positive(2)? * k;
    let male = o.choice(1) == "male";
    let bf = if male {
        495.0 / (1.0324 - 0.19077 * (waist - neck).log10() + 0.15456 * height.log10()) - 450.0
    } else {
        let hip = o.positive(3)? * k;
        495.0 / (1.29579 - 0.35004 * (waist + hip - neck).log10() + 0.22100 * height.log10())
            - 450.0
    };
    if !bf.is_finite() || bf <= 0.0 {
        return Err("Those measurements give no sensible result; check them.".into());
    }
    let class = match (male, bf) {
        (true, b) if b < 6.0 => "essential fat",
        (true, b) if b < 14.0 => "athletes",
        (true, b) if b < 18.0 => "fitness",
        (true, b) if b < 25.0 => "average",
        (false, b) if b < 14.0 => "essential fat",
        (false, b) if b < 21.0 => "athletes",
        (false, b) if b < 25.0 => "fitness",
        (false, b) if b < 32.0 => "average",
        _ => "obese",
    };
    Ok(table(&[(
        "Body fat (US Navy)",
        format!("{bf:.1}% — {class}"),
    )]))
}

fn ideal_weight(_: &str, o: &mut Opts) -> Result<Output, String> {
    let cm = o.positive(0)? * if o.mode == "imperial" { 2.54 } else { 1.0 };
    let over = (cm / 2.54 - 60.0).max(0.0);
    let male = o.choice(1) == "male";
    let f = |m: f64, w: f64, mi: f64, wi: f64| if male { m + mi * over } else { w + wi * over };
    let kg = |v: f64| format!("{} kg ({} lb)", fmt(v, 1), fmt(v / 0.45359237, 0));
    Ok(table(&[
        ("Devine", kg(f(50.0, 45.5, 2.3, 2.3))),
        ("Robinson", kg(f(52.0, 49.0, 1.9, 1.7))),
        ("Miller", kg(f(56.2, 53.1, 1.41, 1.36))),
        ("Hamwi", kg(f(48.0, 45.5, 2.7, 2.2))),
        (
            "BMI 18.5–24.9",
            format!(
                "{} – {} kg",
                fmt(18.5 * (cm / 100.0).powi(2), 1),
                fmt(24.9 * (cm / 100.0).powi(2), 1)
            ),
        ),
    ]))
}

fn bsa(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (kg, cm) = metric(o, 0, 1)?;
    Ok(table(&[
        (
            "Mosteller",
            format!("{} m²", fmt((kg * cm / 3600.0).sqrt(), 3)),
        ),
        (
            "Du Bois",
            format!("{} m²", fmt(0.007184 * kg.powf(0.425) * cm.powf(0.725), 3)),
        ),
        (
            "Haycock",
            format!(
                "{} m²",
                fmt(0.024265 * kg.powf(0.5378) * cm.powf(0.3964), 3)
            ),
        ),
    ]))
}

fn bac(_: &str, o: &mut Opts) -> Result<Output, String> {
    let drinks = o.num(0)?;
    let kg = o.positive(1)?
        * if o.mode == "imperial" {
            0.45359237
        } else {
            1.0
        };
    let hours = o.num(2)?;
    let r = if o.choice(1) == "male" { 0.68 } else { 0.55 };
    // 14 g of alcohol a standard drink; 0.015% eliminated an hour.
    let level = ((drinks * 14.0) / (kg * 1000.0 * r) * 100.0 - 0.015 * hours).max(0.0);
    Ok(table(&[
        ("Estimated BAC", format!("{:.3}%", level)),
        ("Sober in about", format!("{} hours", fmt(level / 0.015, 1))),
        (
            "",
            "An estimate only: never use it to decide whether to drive.".into(),
        ),
    ]))
}

fn heart_zones(_: &str, o: &mut Opts) -> Result<Output, String> {
    let age = o.positive(0)?;
    let rest = o.opt_num(1)?;
    let max = o.opt_num(2)?.unwrap_or(208.0 - 0.7 * age);
    let zone = |lo: f64, hi: f64| match rest {
        Some(r) => format!(
            "{} – {} bpm",
            fmt(r + (max - r) * lo, 0),
            fmt(r + (max - r) * hi, 0)
        ),
        None => format!("{} – {} bpm", fmt(max * lo, 0), fmt(max * hi, 0)),
    };
    Ok(table(&[
        ("Maximum heart rate", format!("{} bpm", fmt(max, 0))),
        ("Zone 1, recovery (50–60%)", zone(0.5, 0.6)),
        ("Zone 2, endurance (60–70%)", zone(0.6, 0.7)),
        ("Zone 3, tempo (70–80%)", zone(0.7, 0.8)),
        ("Zone 4, threshold (80–90%)", zone(0.8, 0.9)),
        ("Zone 5, maximum (90–100%)", zone(0.9, 1.0)),
        (
            "",
            if rest.is_some() {
                "By heart rate reserve (Karvonen)."
            } else {
                "Add a resting heart rate for the Karvonen method."
            }
            .into(),
        ),
    ]))
}

fn one_rep_max(_: &str, o: &mut Opts) -> Result<Output, String> {
    let w = o.positive(0)?;
    let reps = o.positive(1)?;
    if reps > 12.0 {
        return Err("The formulas hold up to about 12 repetitions.".into());
    }
    let epley = w * (1.0 + reps / 30.0);
    let brzycki = w * 36.0 / (37.0 - reps);
    let lombardi = w * reps.powf(0.1);
    let avg = (epley + brzycki + lombardi) / 3.0;
    let mut r = vec![
        ("Epley", fmt(epley, 1)),
        ("Brzycki", fmt(brzycki, 1)),
        ("Lombardi", fmt(lombardi, 1)),
        ("Average", fmt(avg, 1)),
        ("", String::new()),
    ];
    let shares = [
        ("95% (2 reps)", 0.95),
        ("90% (4 reps)", 0.9),
        ("85% (6 reps)", 0.85),
        ("80% (8 reps)", 0.8),
        ("75% (10 reps)", 0.75),
    ];
    r.extend(shares.iter().map(|(n, p)| (*n, fmt(avg * p, 1))));
    Ok(table(&r))
}

/// `h:mm:ss`, `mm:ss` or minutes as seconds.
fn parse_clock(s: &str) -> Option<f64> {
    let parts: Vec<f64> = s
        .trim()
        .split(':')
        .map(|p| p.trim().parse().ok())
        .collect::<Option<_>>()?;
    Some(match parts.as_slice() {
        [m] => m * 60.0,
        [m, s] => m * 60.0 + s,
        [h, m, s] => h * 3600.0 + m * 60.0 + s,
        _ => return None,
    })
}

fn hms(secs: f64) -> String {
    let t = secs.round() as i64;
    if t >= 3600 {
        format!("{}:{:02}:{:02}", t / 3600, t % 3600 / 60, t % 60)
    } else {
        format!("{}:{:02}", t / 60, t % 60)
    }
}

fn pace(_: &str, o: &mut Opts) -> Result<Output, String> {
    let km = o.positive(0)? * if o.mode == "mi" { 1.609344 } else { 1.0 };
    let time = parse_clock(o.field(1))
        .filter(|t| *t > 0.0)
        .ok_or("Write the time as h:mm:ss or mm:ss.")?;
    let per_km = time / km;
    Ok(table(&[
        (
            "Pace",
            format!("{} /km · {} /mi", hms(per_km), hms(per_km * 1.609344)),
        ),
        (
            "Speed",
            format!(
                "{} km/h · {} mph",
                fmt(km / time * 3600.0, 2),
                fmt(km / 1.609344 / time * 3600.0, 2)
            ),
        ),
        ("5 km at this pace", hms(per_km * 5.0)),
        ("10 km", hms(per_km * 10.0)),
        ("Half marathon", hms(per_km * 21.0975)),
        ("Marathon", hms(per_km * 42.195)),
    ]))
}

fn splits(_: &str, o: &mut Opts) -> Result<Output, String> {
    let total = o.positive(0)?;
    let time = parse_clock(o.field(1))
        .filter(|t| *t > 0.0)
        .ok_or("Write the goal time as h:mm:ss or mm:ss.")?;
    let split = o.positive(2)?;
    let n = (total / split).ceil() as usize;
    if n > 500 {
        return Err("That makes too many splits.".into());
    }
    let mut out = Vec::new();
    for i in 1..=n {
        let at = (i as f64 * split).min(total);
        let prev = ((i - 1) as f64 * split).min(total);
        out.push(vec![
            fmt(at, 2),
            hms((at - prev) / total * time),
            hms(at / total * time),
        ]);
    }
    Ok(grid(&["At", "Split", "Elapsed"], out))
}

fn due_date(_: &str, o: &mut Opts) -> Result<Output, String> {
    let lmp = o.date(0)?;
    let cycle = o.opt_num(1)?.unwrap_or(28.0);
    let today = o.date(2)?;
    let due = lmp + 280 + (cycle as i64 - 28);
    let along = today - lmp;
    Ok(table(&[
        (
            "Due date",
            format!("{} ({})", iso(due), WEEKDAYS[weekday(due)]),
        ),
        ("Now", format!("{} weeks and {} days", along / 7, along % 7)),
        (
            "Trimester",
            match along / 7 {
                w if w < 13 => "first",
                w if w < 27 => "second",
                _ => "third",
            }
            .into(),
        ),
        ("Conception, about", iso(lmp + 14 + (cycle as i64 - 28))),
        (
            "",
            "An estimate: few babies arrive on their due date.".into(),
        ),
    ]))
}

fn ovulation(_: &str, o: &mut Opts) -> Result<Output, String> {
    let lmp = o.date(0)?;
    let cycle = o.opt_num(1)?.unwrap_or(28.0) as i64;
    let mut out = Vec::new();
    for k in 0..3 {
        let start = lmp + k * cycle;
        let ov = start + cycle - 14;
        out.push(vec![
            (k + 1).to_string(),
            iso(start),
            format!("{} to {}", iso(ov - 5), iso(ov + 1)),
            iso(ov),
        ]);
    }
    Ok(
        grid(&["Cycle", "Period", "Fertile", "Ovulation, about"], out)
            .note("An estimate from the calendar method; cycles vary."),
    )
}

fn protein(_: &str, o: &mut Opts) -> Result<Output, String> {
    let kg = o.positive(0)? * if o.choice(1) == "lb" { 0.45359237 } else { 1.0 };
    let (lo, hi) = match o.mode {
        "active" => (1.2, 1.6),
        "athlete" => (1.6, 2.2),
        "older" => (1.0, 1.2),
        _ => (0.8, 1.0),
    };
    Ok(table(&[
        (
            "Protein a day",
            format!("{} – {} g", fmt(kg * lo, 0), fmt(kg * hi, 0)),
        ),
        ("Per kilogram", format!("{lo} – {hi} g")),
    ]))
}

fn water(_: &str, o: &mut Opts) -> Result<Output, String> {
    let kg = o.positive(0)? * if o.choice(0) == "lb" { 0.45359237 } else { 1.0 };
    let exercise = o.opt_num(1)?.unwrap_or(0.0);
    let litres = kg * 0.033 + exercise / 30.0 * 0.35;
    Ok(table(&[
        (
            "Water a day",
            format!("{} L ({} cups)", fmt(litres, 1), fmt(litres / 0.25, 0)),
        ),
        (
            "",
            "Food provides some of it; thirst is a good guide.".into(),
        ),
    ]))
}

fn sleep(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (hh, mm) = o.field(0).split_once(':').unwrap_or((o.field(0), "0"));
    let h = match (hh.trim().parse::<f64>(), mm.trim().parse::<f64>()) {
        (Ok(h), Ok(m)) if (0.0..24.0).contains(&h) && (0.0..60.0).contains(&m) => h + m / 60.0,
        _ => return Err("Write the time as hh:mm.".into()),
    };
    let fall = o.opt_num(1)?.unwrap_or(15.0) / 60.0;
    let cycles = [6.0, 5.0, 4.0, 3.0];
    let lines: Vec<Vec<String>> = cycles
        .iter()
        .map(|c| {
            let at = if o.mode == "wake" {
                h - c * 1.5 - fall
            } else {
                h + c * 1.5 + fall
            };
            vec![
                clock(at),
                format!("{c} cycles"),
                format!("{} hours", fmt(c * 1.5, 1)),
            ]
        })
        .collect();
    Ok(grid(
        &[
            if o.mode == "wake" {
                "Go to bed at"
            } else {
                "Wake up at"
            },
            "Sleep",
            "Hours",
        ],
        lines,
    ))
}

fn blood_pressure(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (s, d) = (o.positive(0)?, o.positive(1)?);
    let cat = if s > 180.0 || d > 120.0 {
        "Hypertensive crisis: get medical help now"
    } else if s >= 140.0 || d >= 90.0 {
        "High blood pressure, stage 2"
    } else if s >= 130.0 || d >= 80.0 {
        "High blood pressure, stage 1"
    } else if s >= 120.0 {
        "Elevated"
    } else {
        "Normal"
    };
    Ok(table(&[
        ("Category (AHA)", cat.into()),
        ("Pulse pressure", format!("{} mmHg", fmt(s - d, 0))),
        (
            "Mean arterial",
            format!("{} mmHg", fmt(d + (s - d) / 3.0, 0)),
        ),
    ]))
}

fn blood_sugar(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.positive(0)?;
    let mgdl = if o.mode == "mmol" { v * 18.0182 } else { v };
    let (state, note) = match mgdl {
        x if x < 70.0 => ("Low", "below 70 mg/dL"),
        x if x < 100.0 => ("Normal fasting", "70–99 mg/dL"),
        x if x < 126.0 => ("Prediabetes range when fasting", "100–125 mg/dL"),
        _ => ("Diabetes range when fasting", "126 mg/dL or more"),
    };
    Ok(table(&[
        ("mg/dL", fmt(mgdl, 0)),
        ("mmol/L", fmt(mgdl / 18.0182, 1)),
        ("If fasting", format!("{state} ({note})")),
        (
            "Matching HbA1c, as an average",
            format!("{}%", fmt((mgdl + 46.7) / 28.7, 1)),
        ),
    ]))
}

fn vo2(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = match o.mode {
        "cooper" => (o.positive(0)? - 504.9) / 44.73,
        _ => {
            let age = o.positive(0)?;
            let rest = o.positive(1)?;
            15.3 * (208.0 - 0.7 * age) / rest
        }
    };
    Ok(table(&[("VO₂ max", format!("{} ml/kg/min", fmt(v, 1)))]))
}

// Home, DIY and garden

fn paint(_: &str, o: &mut Opts) -> Result<Output, String> {
    let perimeter = o.positive(0)?;
    let height = o.positive(1)?;
    let doors = o.opt_num(2)?.unwrap_or(0.0);
    let windows = o.opt_num(3)?.unwrap_or(0.0);
    let coats = o.opt_num(4)?.unwrap_or(2.0);
    let coverage = o.opt_num(5)?.unwrap_or(10.0);
    let imperial = o.mode == "imperial";
    let (door, window) = if imperial { (21.0, 15.0) } else { (1.9, 1.5) };
    let area = perimeter * height - doors * door - windows * window;
    let paint = area * coats / coverage;
    Ok(table(&[
        (
            "Wall area",
            format!("{} {}", fmt(area, 1), if imperial { "sq ft" } else { "m²" }),
        ),
        (
            "Paint",
            format!(
                "{} {}",
                fmt(paint, 1),
                if imperial { "gallons" } else { "litres" }
            ),
        ),
    ]))
}

fn room_area(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (l, w) = (o.positive(0)?, o.positive(1)?);
    let h = o.opt_num(2)?;
    let u = if o.mode == "ft" { "ft" } else { "m" };
    let mut r = vec![
        ("Floor area", format!("{} {u}²", fmt(l * w, 2))),
        ("Perimeter", format!("{} {u}", fmt(2.0 * (l + w), 2))),
    ];
    if let Some(h) = h {
        r.push(("Wall area", format!("{} {u}²", fmt(2.0 * (l + w) * h, 2))));
        r.push(("Volume", format!("{} {u}³", fmt(l * w * h, 2))));
    }
    Ok(table(&r))
}

fn flooring(_: &str, o: &mut Opts) -> Result<Output, String> {
    let area = o.positive(0)? * o.positive(1)?;
    let waste = o.opt_num(2)?.unwrap_or(10.0) / 100.0;
    let box_cover = o.opt_num(3)?;
    let need = area * (1.0 + waste);
    let mut r = vec![("Area", fmt(area, 2)), ("With waste", fmt(need, 2))];
    if let Some(b) = box_cover.filter(|b| *b > 0.0) {
        r.push(("Boxes", fmt((need / b).ceil(), 0)));
    }
    Ok(table(&r))
}

fn tiles(_: &str, o: &mut Opts) -> Result<Output, String> {
    let area = o.positive(0)? * o.positive(1)?;
    let tile = o.positive(2)? * o.positive(3)? / 1e4;
    let gap = o.opt_num(4)?.unwrap_or(0.3) / 100.0;
    let waste = o.opt_num(5)?.unwrap_or(10.0) / 100.0;
    let per = (o.num(2)? / 100.0 + gap) * (o.num(3)? / 100.0 + gap);
    let count = area / per * (1.0 + waste);
    Ok(table(&[
        ("Area", format!("{} m²", fmt(area, 2))),
        ("Tile size", format!("{} m²", fmt(tile, 4))),
        ("Tiles", fmt(count.ceil(), 0)),
    ]))
}

fn grout(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (tl, tw) = (o.positive(0)?, o.positive(1)?);
    let thick = o.positive(2)?;
    let joint = o.positive(3)?;
    let area = o.positive(4)?;
    // kg = (L + W) / (L × W) × thickness × joint × 1.6 × area, sizes in mm.
    let kg = (tl + tw) / (tl * tw) * thick * joint * 1.6 * area;
    Ok(table(&[
        ("Grout", format!("{} kg", fmt(kg * 1.1, 2))),
        ("", "With 10% extra.".into()),
    ]))
}

fn wallpaper(_: &str, o: &mut Opts) -> Result<Output, String> {
    let perimeter = o.positive(0)?;
    let height = o.positive(1)?;
    let width = o.positive(2)?;
    let length = o.positive(3)?;
    let repeat = o.opt_num(4)?.unwrap_or(0.0) / 100.0;
    let drop = if repeat > 0.0 {
        (height / repeat).ceil() * repeat
    } else {
        height
    } + 0.1;
    let per_roll = (length / drop).floor();
    if per_roll < 1.0 {
        return Err("A roll is shorter than one drop.".into());
    }
    let strips = (perimeter / width).ceil();
    Ok(table(&[
        ("Strips", fmt(strips, 0)),
        ("Strips a roll", fmt(per_roll, 0)),
        ("Rolls", fmt((strips / per_roll).ceil(), 0)),
    ]))
}

fn drywall(_: &str, o: &mut Opts) -> Result<Output, String> {
    let area = o.positive(0)?;
    let sheet = o.positive(1)? * o.positive(2)?;
    let n = (area / sheet * 1.1).ceil();
    Ok(table(&[
        ("Sheets", fmt(n, 0)),
        ("Screws (about 32 a sheet)", fmt(n * 32.0, 0)),
        ("Joint compound", format!("{} L", fmt(n * 1.8, 1))),
        ("Tape", format!("{} m", fmt(n * 11.0, 0))),
    ]))
}

fn concrete(_: &str, o: &mut Opts) -> Result<Output, String> {
    let imperial = o.mode == "imperial";
    let (l, w, d) = (o.positive(0)?, o.positive(1)?, o.positive(2)?);
    let m3 = if imperial {
        l * w * d / 12.0 * 0.0283168
    } else {
        l * w * d / 100.0
    };
    Ok(table(&[
        (
            "Volume",
            format!("{} m³ · {} yd³", fmt(m3, 3), fmt(m3 / 0.764555, 2)),
        ),
        ("With 10% extra", format!("{} m³", fmt(m3 * 1.1, 3))),
        ("80 lb bags", fmt((m3 * 1.1 / 0.0170).ceil(), 0)),
        ("60 lb bags", fmt((m3 * 1.1 / 0.0127).ceil(), 0)),
        ("25 kg bags", fmt((m3 * 1.1 / 0.0118).ceil(), 0)),
    ]))
}

fn deck(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (len, width) = (o.positive(0)?, o.positive(1)?);
    let board = o.positive(2)? / 100.0;
    let gap = o.opt_num(3)?.unwrap_or(0.5) / 100.0;
    let board_len = o.positive(4)?;
    let rows = (width / (board + gap)).ceil();
    let per_row = (len / board_len).ceil();
    Ok(table(&[
        ("Rows of boards", fmt(rows, 0)),
        ("Boards", fmt((rows * per_row * 1.1).ceil(), 0)),
        ("Running length", format!("{} m", fmt(rows * len, 1))),
        ("Joists at 40 cm", fmt((len / 0.4).ceil() + 1.0, 0)),
    ]))
}

fn fence(_: &str, o: &mut Opts) -> Result<Output, String> {
    let len = o.positive(0)?;
    let spacing = o.positive(1)?;
    let rails = o.opt_num(2)?.unwrap_or(2.0);
    let picket = o.opt_num(3)?.unwrap_or(0.0) / 100.0;
    let gap = o.opt_num(4)?.unwrap_or(0.0) / 100.0;
    let sections = (len / spacing).ceil();
    let mut r = vec![
        ("Posts", fmt(sections + 1.0, 0)),
        ("Sections", fmt(sections, 0)),
        ("Rails", fmt(sections * rails, 0)),
    ];
    if picket > 0.0 {
        r.push(("Pickets", fmt((len / (picket + gap)).ceil(), 0)));
    }
    Ok(table(&r))
}

fn board_feet(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (t, w, l) = (o.positive(0)?, o.positive(1)?, o.positive(2)?);
    let n = o.opt_num(3)?.unwrap_or(1.0);
    let bf = t * w * l / 12.0 * n;
    let mut r = vec![
        ("Board feet", fmt(bf, 2)),
        ("Cubic metres", fmt(bf * 0.00235974, 4)),
    ];
    if let Some(p) = o.opt_num(4)? {
        r.push(("Cost", money(bf * p)));
    }
    Ok(table(&r))
}

fn mulch(_: &str, o: &mut Opts) -> Result<Output, String> {
    let imperial = o.mode == "imperial";
    let area = o.positive(0)? * o.positive(1)?;
    let depth = o.positive(2)?;
    let m3 = if imperial {
        area * depth / 12.0 * 0.0283168
    } else {
        area * depth / 100.0
    };
    Ok(table(&[
        (
            "Volume",
            format!("{} m³ · {} yd³", fmt(m3, 2), fmt(m3 / 0.764555, 2)),
        ),
        ("50 L bags", fmt((m3 * 1000.0 / 50.0).ceil(), 0)),
        ("2 cu ft bags", fmt((m3 / 0.0566337).ceil(), 0)),
    ]))
}

fn stairs(_: &str, o: &mut Opts) -> Result<Output, String> {
    let rise = o.positive(0)?;
    let ideal = o.opt_num(1)?.unwrap_or(18.0);
    let tread = o.opt_num(2)?.unwrap_or(28.0);
    let steps = (rise / ideal).round().max(1.0);
    let riser = rise / steps;
    let run = tread * (steps - 1.0);
    let rule = 2.0 * riser + tread;
    Ok(table(&[
        ("Risers", fmt(steps, 0)),
        ("Riser height", format!("{} cm", fmt(riser, 1))),
        ("Treads", fmt(steps - 1.0, 0)),
        ("Total run", format!("{} cm", fmt(run, 1))),
        (
            "Stringer length",
            format!("{} cm", fmt((rise * rise + run * run).sqrt(), 1)),
        ),
        (
            "Angle",
            format!("{}°", fmt((rise / run.max(1.0)).atan().to_degrees(), 1)),
        ),
        (
            "2 risers + tread",
            format!("{} cm (comfortable: 60–65)", fmt(rule, 1)),
        ),
    ]))
}

fn curtains(_: &str, o: &mut Opts) -> Result<Output, String> {
    let width = o.positive(0)?;
    let drop = o.positive(1)?;
    let fullness: f64 = o.mode.parse().unwrap_or(2.0);
    let fabric = o.opt_num(2)?.unwrap_or(140.0);
    let total = (width + 20.0) * fullness;
    let widths = (total / fabric).ceil();
    Ok(table(&[
        (
            "Curtain width",
            format!("{} cm ({} each of two)", fmt(total, 0), fmt(total / 2.0, 0)),
        ),
        ("Fabric widths", fmt(widths, 0)),
        (
            "Fabric length",
            format!("{} m", fmt(widths * (drop + 30.0) / 100.0, 2)),
        ),
    ]))
}

fn ac_size(_: &str, o: &mut Opts) -> Result<Output, String> {
    let sqft = o.positive(0)? * if o.choice(0) == "m2" { 10.7639 } else { 1.0 };
    let people = o.opt_num(1)?.unwrap_or(2.0);
    let mut btu = sqft * 20.0;
    btu *= match o.choice(1) {
        "sunny" => 1.1,
        "shaded" => 0.9,
        _ => 1.0,
    };
    btu += (people - 2.0).max(0.0) * 600.0;
    if o.flag {
        btu += 4000.0;
    }
    Ok(table(&[
        ("Cooling", format!("{} BTU/h", fmt(btu.round(), 0))),
        (
            "",
            format!(
                "{} kW, {} tons",
                fmt(btu * 0.000293071, 2),
                fmt(btu / 12000.0, 2)
            ),
        ),
    ]))
}

fn energy_cost(_: &str, o: &mut Opts) -> Result<Output, String> {
    let watts = o.positive(0)?;
    let hours = o.positive(1)?;
    let price = o.num(2)?;
    let kwh = watts * hours / 1000.0;
    Ok(table(&[
        ("Energy a day", format!("{} kWh", fmt(kwh, 3))),
        ("A day", money(kwh * price)),
        ("A month", money(kwh * price * 30.44)),
        ("A year", money(kwh * price * 365.0)),
    ]))
}

fn split_by(text: &str, total: f64, what: &str) -> Result<Output, String> {
    let people = rows(text, 1)?;
    let sum: f64 = people.iter().map(|p| p.1[0]).sum();
    if sum <= 0.0 {
        return Err(format!("The {what} add up to nothing."));
    }
    let mut out: Vec<(String, String)> = people
        .iter()
        .map(|(n, v)| {
            (
                n.clone(),
                format!(
                    "{} ({})",
                    money(total * v[0] / sum),
                    pct(v[0] / sum * 100.0)
                ),
            )
        })
        .collect();
    out.push((String::new(), format!("Split {} by {what}.", money(total))));
    Ok(table(
        &out.iter()
            .map(|(a, b)| (a.as_str(), b.clone()))
            .collect::<Vec<_>>(),
    ))
}

fn bill_split(text: &str, o: &mut Opts) -> Result<Output, String> {
    split_by(text, o.positive(0)?, "days at home")
}

fn rent_split(text: &str, o: &mut Opts) -> Result<Output, String> {
    let total = o.positive(0)?;
    let shared = o.opt_num(1)?.unwrap_or(30.0) / 100.0;
    let people = rows(text, 1)?;
    let sum: f64 = people.iter().map(|p| p.1[0]).sum();
    let n = people.len() as f64;
    let mut out: Vec<(String, String)> = people
        .iter()
        .map(|(name, v)| {
            (
                name.clone(),
                money(total * shared / n + total * (1.0 - shared) * v[0] / sum),
            )
        })
        .collect();
    out.push((
        String::new(),
        format!(
            "{} of the rent is shared equally, the rest by room size.",
            pct(shared * 100.0)
        ),
    ));
    Ok(table(
        &out.iter()
            .map(|(a, b)| (a.as_str(), b.clone()))
            .collect::<Vec<_>>(),
    ))
}

fn solar(_: &str, o: &mut Opts) -> Result<Output, String> {
    let kwh = o.positive(0)? * 12.0;
    let sun = o.positive(1)?;
    let panel = o.positive(2)?;
    let kw = kwh / 365.0 / sun / 0.8;
    let panels = (kw * 1000.0 / panel).ceil();
    let mut r = vec![
        ("System size", format!("{} kW", fmt(kw, 2))),
        ("Panels", fmt(panels, 0)),
        (
            "Roof area",
            format!("about {} m²", fmt(panels * panel / 200.0, 0)),
        ),
    ];
    if let Some(cost) = o.opt_num(3)? {
        r.push(("Cost", money(kw * 1000.0 * cost)));
    }
    Ok(table(&r))
}

fn tv_distance(_: &str, o: &mut Opts) -> Result<Output, String> {
    let d = o.positive(0)?;
    let (w, _) = (d * 16.0 / (337f64).sqrt(), d * 9.0 / (337f64).sqrt());
    // Distances for a 30° (SMPTE) and 40° (THX) field of view.
    let at = |deg: f64| w / 2.0 / (deg / 2.0).to_radians().tan();
    let both = |inch: f64| format!("{} m ({} ft)", fmt(inch * 0.0254, 2), fmt(inch / 12.0, 1));
    Ok(table(&[
        ("Screen width", format!("{} in", fmt(w, 1))),
        ("Cinematic (40°, THX)", both(at(40.0))),
        ("Comfortable (30°, SMPTE)", both(at(30.0))),
        ("Farthest for 4K detail", both(d * 1.5)),
    ]))
}

fn lighting(_: &str, o: &mut Opts) -> Result<Output, String> {
    let area = o.positive(0)? * if o.choice(1) == "ft2" { 0.092903 } else { 1.0 };
    let lux: f64 = o.mode.parse().unwrap_or(150.0);
    let lumens = area * lux;
    Ok(table(&[
        ("Light needed", format!("{} lumens", fmt(lumens, 0))),
        ("LED power, about", format!("{} W", fmt(lumens / 100.0, 0))),
        ("Bulbs of 800 lm", fmt((lumens / 800.0).ceil(), 0)),
    ]))
}

fn moving_boxes(_: &str, o: &mut Opts) -> Result<Output, String> {
    let rooms = o.positive(0)?;
    let people = o.opt_num(1)?.unwrap_or(1.0);
    let base = rooms * 10.0 + people * 10.0;
    let k = match o.mode {
        "light" => 0.8,
        "heavy" => 1.3,
        _ => 1.0,
    };
    let n = base * k;
    Ok(table(&[
        ("Small boxes", fmt((n * 0.35).ceil(), 0)),
        ("Medium boxes", fmt((n * 0.4).ceil(), 0)),
        ("Large boxes", fmt((n * 0.25).ceil(), 0)),
        (
            "Packing tape",
            format!("{} rolls", fmt((n / 12.0).ceil(), 0)),
        ),
    ]))
}

fn water_usage(_: &str, o: &mut Opts) -> Result<Output, String> {
    let people = o.positive(0)?;
    let shower = o.num(1)? * 9.0;
    let flushes = o.num(2)? * 6.0;
    let laundry = o.opt_num(3)?.unwrap_or(0.0) * 50.0 / 7.0;
    let dishes = o.opt_num(4)?.unwrap_or(0.0) * 12.0 / 7.0;
    let day = people * (shower + flushes + 10.0) + laundry + dishes;
    Ok(table(&[
        ("Litres a day", fmt(day, 0)),
        ("A month", format!("{} m³", fmt(day * 30.44 / 1000.0, 2))),
        ("A year", format!("{} m³", fmt(day * 365.0 / 1000.0, 1))),
    ]))
}

fn commute_co2(_: &str, o: &mut Opts) -> Result<Output, String> {
    let km = o.positive(0)? * 2.0;
    let days = o.opt_num(1)?.unwrap_or(220.0);
    let modes = [
        ("Petrol car", 0.17),
        ("Diesel car", 0.16),
        ("Electric car", 0.05),
        ("Bus", 0.10),
        ("Train", 0.035),
        ("Motorbike", 0.11),
        ("Bike or walk", 0.0),
    ];
    let mut r: Vec<(&str, String)> = modes
        .iter()
        .map(|(n, f)| (*n, format!("{} kg CO₂ a year", fmt(km * days * f, 0))))
        .collect();
    r.push(("", format!("For {} km a day, there and back.", fmt(km, 1))));
    Ok(table(&r))
}

fn trees(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = o.positive(0)?;
    let years = o.opt_num(1)?.unwrap_or(1.0);
    let kg = n * 22.0 * years;
    Ok(table(&[
        (
            "CO₂ absorbed",
            format!("{} kg ({} t)", fmt(kg, 0), fmt(kg / 1000.0, 2)),
        ),
        ("Like driving", format!("{} km less", fmt(kg / 0.17, 0))),
        (
            "",
            "About 22 kg a mature tree a year; young trees take far less.".into(),
        ),
    ]))
}

fn plant_spacing(_: &str, o: &mut Opts) -> Result<Output, String> {
    let area = o.positive(0)? * o.positive(1)? * 1e4;
    let s = o.positive(2)?;
    let n = if o.mode == "triangle" {
        area / (s * s * 0.866)
    } else {
        area / (s * s)
    };
    Ok(table(&[("Plants", fmt(n.floor(), 0))]))
}

fn lawn_seed(_: &str, o: &mut Opts) -> Result<Output, String> {
    let area = o.positive(0)?;
    let rate = o
        .opt_num(1)?
        .unwrap_or(if o.mode == "overseed" { 20.0 } else { 40.0 });
    Ok(table(&[
        ("Seed", format!("{} kg", fmt(area * rate / 1000.0, 2))),
        ("Rate", format!("{} g/m²", fmt(rate, 0))),
    ]))
}

fn compost(_: &str, o: &mut Opts) -> Result<Output, String> {
    let greens = o.num(0)?;
    let browns = o.num(1)?;
    // Typical C:N: greens 20:1, browns 60:1, as equal volumes.
    let ratio = (greens * 20.0 + browns * 60.0) / (greens + browns).max(1e-9);
    let verdict = match ratio {
        r if r < 25.0 => "Too much green: add browns (leaves, cardboard).",
        r if r > 35.0 => "Too much brown: add greens (scraps, grass).",
        _ => "Balanced: 25–35 is ideal.",
    };
    Ok(table(&[
        ("C:N ratio", format!("{}:1", fmt(ratio, 0))),
        ("", verdict.into()),
    ]))
}

fn fertilizer(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n_pct = o.positive(0)? / 100.0;
    let area = o.positive(1)?;
    let target = o.opt_num(2)?.unwrap_or(5.0);
    let grams = target * area / n_pct;
    Ok(table(&[
        (
            "Product",
            format!("{} g ({} kg)", fmt(grams, 0), fmt(grams / 1000.0, 2)),
        ),
        ("Nitrogen", format!("{} g/m²", fmt(target, 1))),
    ]))
}

fn harvest(_: &str, o: &mut Opts) -> Result<Output, String> {
    let sown = o.date(0)?;
    let days = o.int(1)?;
    Ok(table(&[
        ("Harvest from", iso(sown + days)),
        ("", format!("{} days after {}", days, iso(sown))),
    ]))
}

fn seed_start(_: &str, o: &mut Opts) -> Result<Output, String> {
    let frost = o.date(0)?;
    let weeks = o.num(1)?;
    let start = frost - (weeks * 7.0) as i64;
    Ok(table(&[
        ("Start indoors", iso(start)),
        ("Transplant after", iso(frost + 7)),
    ]))
}

fn aquarium(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (l, w, h) = (o.positive(0)?, o.positive(1)?, o.positive(2)?);
    let litres = if o.mode == "in" {
        l * w * h * 0.0163871
    } else {
        l * w * h / 1000.0
    };
    Ok(table(&[
        (
            "Volume",
            format!("{} L · {} US gal", fmt(litres, 1), fmt(litres / 3.78541, 1)),
        ),
        ("Water weight", format!("{} kg", fmt(litres, 0))),
        ("Filled to 90%", format!("{} L", fmt(litres * 0.9, 1))),
    ]))
}

fn pet_food(_: &str, o: &mut Opts) -> Result<Output, String> {
    let kg = o.positive(0)? * if o.choice(2) == "lb" { 0.45359237 } else { 1.0 };
    let rer = 70.0 * kg.powf(0.75);
    let factor: f64 = o.choice(1).parse().unwrap_or(1.6);
    let kcal = rer * factor;
    let mut r = vec![
        ("Resting energy", format!("{} kcal", fmt(rer, 0))),
        ("A day", format!("{} kcal", fmt(kcal, 0))),
    ];
    if let Some(d) = o.opt_num(1)?.filter(|d| *d > 0.0) {
        r.push(("Food", format!("{} g a day", fmt(kcal / d * 1000.0, 0))));
    }
    Ok(table(&r))
}

fn pet_age(_: &str, o: &mut Opts) -> Result<Output, String> {
    let y = o.positive(0)?;
    let human = match o.mode {
        "cat" => {
            if y <= 1.0 {
                15.0 * y
            } else if y <= 2.0 {
                15.0 + 9.0 * (y - 1.0)
            } else {
                24.0 + 4.0 * (y - 2.0)
            }
        }
        _ => {
            let late: f64 = match o.choice(1) {
                "small" => 4.0,
                "large" => 6.0,
                _ => 5.0,
            };
            if y <= 1.0 {
                15.0 * y
            } else if y <= 2.0 {
                15.0 + 9.0 * (y - 1.0)
            } else {
                24.0 + late * (y - 2.0)
            }
        }
    };
    Ok(table(&[("In human years", fmt(human, 0))]))
}

// Cooking

const COOKING_UNITS: &[(&str, f64)] = &[
    ("ml", 1.0),
    ("l", 1000.0),
    ("tsp", 4.92892),
    ("tbsp", 14.7868),
    ("floz", 29.5735),
    ("cup", 236.588),
    ("pint", 473.176),
    ("quart", 946.353),
    ("gallon", 3785.41),
    ("g", -1.0),
    ("kg", -1000.0),
    ("oz", -28.3495),
    ("lb", -453.592),
];

fn cooking_units(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.num(0)?;
    let density: f64 = o.choice(2).parse().unwrap_or(1.0);
    let unit = |id: &str| {
        COOKING_UNITS
            .iter()
            .find(|u| u.0 == id)
            .map_or(1.0, |u| u.1)
    };
    let (from, to) = (unit(o.mode), unit(o.choice(1)));
    // Volumes are positive millilitres, weights negative grams.
    let base_ml = if from > 0.0 {
        v * from
    } else {
        v * -from / density
    };
    let out = if to > 0.0 {
        base_ml / to
    } else {
        base_ml * density / -to
    };
    let ing = if (from > 0.0) != (to > 0.0) {
        " (by the ingredient's density)"
    } else {
        ""
    };
    Ok(table(&[(
        &format!("{} {}", fmt(v, 3), o.mode),
        format!("{} {}{ing}", fmt(out, 3), o.choice(1)),
    )]))
}

fn oven(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.num(0)?;
    let c = match o.mode {
        "f" => (v - 32.0) / 1.8,
        "gas" => {
            if v < 1.0 {
                120.0 + (v - 0.25) * 20.0
            } else {
                121.0 + v * 14.0
            }
        }
        _ => v,
    };
    let gas = if c < 135.0 {
        (((c - 115.0) / 20.0).max(0.0) * 4.0).round() / 4.0
    } else {
        ((c - 121.0) / 14.0).round()
    };
    Ok(table(&[
        ("Celsius", format!("{} °C", fmt(c, 0))),
        ("Fahrenheit", format!("{} °F", fmt(c * 1.8 + 32.0, 0))),
        ("Fan oven", format!("{} °C", fmt(c - 20.0, 0))),
        ("Gas mark", fmt(gas.clamp(0.25, 10.0), 2)),
    ]))
}

fn air_fryer(_: &str, o: &mut Opts) -> Result<Output, String> {
    let t = o.num(0)?;
    let minutes = o.positive(1)?;
    let f = o.mode == "f";
    let new_t = if f { t - 25.0 } else { t - 15.0 };
    Ok(table(&[
        (
            "Air fryer temperature",
            format!("{} °{}", fmt(new_t, 0), if f { "F" } else { "C" }),
        ),
        ("Time", format!("{} minutes", fmt(minutes * 0.8, 0))),
        ("", "Check early and shake the basket halfway.".into()),
    ]))
}

/// A quantity at the start of a line: `2`, `1.5`, `1/2`, `1 1/2`, `½`,
/// `1½`, and how many bytes it takes.
fn leading_amount(line: &str) -> Option<(f64, usize)> {
    let fractions = [
        ('½', 0.5),
        ('⅓', 1.0 / 3.0),
        ('⅔', 2.0 / 3.0),
        ('¼', 0.25),
        ('¾', 0.75),
        ('⅛', 0.125),
    ];
    let re = regex_lite::Regex::new(
        r"^\s*(?:(\d+)\s+(\d+)/(\d+)|(\d+)/(\d+)|(\d+(?:[.,]\d+)?))?\s*([½⅓⅔¼¾⅛])?",
    )
    .unwrap();
    let c = re.captures(line)?;
    let n = |i: usize| {
        c.get(i)
            .and_then(|m| m.as_str().replace(',', ".").parse::<f64>().ok())
    };
    let mut v = match (n(1), n(2), n(3), n(4), n(5), n(6)) {
        (Some(w), Some(a), Some(b), ..) if b > 0.0 => w + a / b,
        (_, _, _, Some(a), Some(b), _) if b > 0.0 => a / b,
        (.., Some(x)) => x,
        _ => 0.0,
    };
    if let Some(f) = c.get(7) {
        v += fractions
            .iter()
            .find(|x| f.as_str().starts_with(x.0))
            .map_or(0.0, |x| x.1);
    }
    let len = c[0].trim_end().len();
    (len > 0).then_some((v, len))
}

fn nice_amount(v: f64) -> String {
    let whole = v.floor();
    let frac = v - whole;
    for (f, s) in [
        (0.25, "¼"),
        (1.0 / 3.0, "⅓"),
        (0.5, "½"),
        (2.0 / 3.0, "⅔"),
        (0.75, "¾"),
    ] {
        if (frac - f).abs() < 0.02 {
            return if whole == 0.0 {
                s.into()
            } else {
                format!("{whole}{s}")
            };
        }
    }
    fmt(v, 2)
}

fn recipe_scale(text: &str, o: &mut Opts) -> Result<Output, String> {
    let k = o.positive(1)? / o.positive(0)?;
    Ok(text
        .lines()
        .map(|l| match leading_amount(l) {
            Some((v, len)) if len > 0 => format!("{}{}", nice_amount(v * k), &l[len..]),
            _ => l.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("\n")
        .into())
}

fn bakers(_: &str, o: &mut Opts) -> Result<Output, String> {
    let flour = o.positive(0)?;
    let water = o.num(1)? / 100.0;
    let salt = o.num(2)? / 100.0;
    let yeast = o.opt_num(3)?.unwrap_or(1.0) / 100.0;
    let total = flour * (1.0 + water + salt + yeast);
    Ok(table(&[
        ("Flour", format!("{} g (100%)", fmt(flour, 0))),
        ("Water", format!("{} g", fmt(flour * water, 0))),
        ("Salt", format!("{} g", fmt(flour * salt, 1))),
        ("Yeast", format!("{} g", fmt(flour * yeast, 1))),
        ("Dough", format!("{} g", fmt(total, 0))),
    ]))
}

fn sourdough(_: &str, o: &mut Opts) -> Result<Output, String> {
    let flour = o.positive(0)?;
    let water = o.num(1)?;
    let starter = o.num(2)?;
    let sh = o.opt_num(3)?.unwrap_or(100.0) / 100.0;
    let sf = starter / (1.0 + sh);
    let sw = starter - sf;
    Ok(table(&[
        ("Hydration", pct((water + sw) / (flour + sf) * 100.0)),
        ("Total flour", format!("{} g", fmt(flour + sf, 0))),
        ("Total water", format!("{} g", fmt(water + sw, 0))),
        ("Starter share", pct(starter / (flour + sf) * 100.0)),
    ]))
}

fn coffee(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.positive(0)?;
    let ratio: f64 = o.choice(1).parse().unwrap_or(16.0);
    Ok(if o.mode == "water" {
        table(&[
            ("Coffee", format!("{} g", fmt(v / ratio, 1))),
            ("Water", format!("{} g", fmt(v, 0))),
            ("Ratio", format!("1:{ratio}")),
        ])
    } else {
        table(&[
            ("Water", format!("{} g", fmt(v * ratio, 0))),
            ("Coffee", format!("{} g", fmt(v, 1))),
            ("Ratio", format!("1:{ratio}")),
        ])
    })
}

fn brine(_: &str, o: &mut Opts) -> Result<Output, String> {
    let water = o.positive(0)?;
    let salinity = o.positive(1)? / 100.0;
    let sugar = o.opt_num(2)?.unwrap_or(0.0) / 100.0;
    Ok(table(&[
        ("Salt", format!("{} g", fmt(water * 1000.0 * salinity, 0))),
        ("Sugar", format!("{} g", fmt(water * 1000.0 * sugar, 0))),
        (
            "",
            "For litres of water; 5–8% for meat, 2–3% for vegetable ferments.".into(),
        ),
    ]))
}

fn pan_area(kind: &str, a: f64, b: f64) -> f64 {
    match kind {
        "round" => std::f64::consts::PI * (a / 2.0).powi(2),
        "square" => a * a,
        _ => a * b,
    }
}

fn pan_size(_: &str, o: &mut Opts) -> Result<Output, String> {
    let from = pan_area(o.mode, o.positive(0)?, o.opt_num(1)?.unwrap_or(0.0));
    let to = pan_area(o.choice(1), o.positive(2)?, o.opt_num(3)?.unwrap_or(0.0));
    if from <= 0.0 || to <= 0.0 {
        return Err("A rectangular pan needs both sides.".into());
    }
    Ok(table(&[
        ("Scale the recipe by", fmt(to / from, 2)),
        ("Area", format!("{} → {}", fmt(from, 0), fmt(to, 0))),
    ]))
}

fn eggs(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = o.positive(0)?;
    let sizes = [
        ("small", "Small", 43.0),
        ("medium", "Medium", 50.0),
        ("large", "Large", 57.0),
        ("xl", "Extra large", 64.0),
        ("jumbo", "Jumbo", 71.0),
    ];
    let w = sizes.iter().find(|s| s.0 == o.mode).map_or(57.0, |s| s.2) * n;
    let mut r: Vec<(&str, String)> = sizes.iter().map(|s| (s.1, fmt(w / s.2, 2))).collect();
    r.push(("Weight without shells", format!("{} g", fmt(w, 0))));
    Ok(table(&r))
}

fn abv(_: &str, o: &mut Opts) -> Result<Output, String> {
    let og = o.positive(0)?;
    let fg = o.positive(1)?;
    let a = (og - fg) * 131.25;
    // Calories by the ASBC formula, from extracts in degrees Plato.
    let plato = |sg: f64| -668.962 + 1262.45 * sg - 776.43 * sg * sg + 182.94 * sg.powi(3);
    let real_extract = 0.1808 * plato(og) + 0.8192 * plato(fg);
    let abw = 0.79 * a / fg;
    let kcal = (6.9 * abw + 4.0 * (real_extract - 0.1)) * fg * 3.55;
    Ok(table(&[
        ("Alcohol by volume", pct(a)),
        ("Apparent attenuation", pct((og - fg) / (og - 1.0) * 100.0)),
        ("Calories per 355 ml", fmt(kcal, 0)),
    ]))
}

fn defrost(_: &str, o: &mut Opts) -> Result<Output, String> {
    let kg = o.positive(0)? * if o.choice(1) == "lb" { 0.45359237 } else { 1.0 };
    let hours = match o.mode {
        "water" => kg / 0.45359237 * 0.5,
        "microwave" => kg * 0.2,
        _ => kg / 2.27 * 24.0,
    };
    Ok(table(&[
        ("Thawing time", format!("about {} hours", fmt(hours, 1))),
        (
            "",
            "Cook food thawed in water or a microwave right away.".into(),
        ),
    ]))
}

const DONENESS: &str = "\
Food                       Safe internal temperature (USDA)
Beef, pork, lamb, veal     63 °C / 145 °F, then rest 3 minutes
Ground meat                71 °C / 160 °F
Poultry (all)              74 °C / 165 °F
Fish and shellfish         63 °C / 145 °F
Eggs and egg dishes        71 °C / 160 °F
Leftovers, casseroles      74 °C / 165 °F

Beef and lamb doneness      °C       °F
Rare                       52       125
Medium rare                57       135
Medium                     63       145
Medium well                68       155
Well done                  71+      160+";

fn doneness(_: &str, o: &mut Opts) -> Result<Output, String> {
    reference(DONENESS, o.field(0))
}

const SMOKE_POINTS: &str = "\
Oil                        °C      °F
Avocado oil (refined)      270     520
Safflower oil              265     510
Rice bran oil              254     490
Light olive oil            240     465
Peanut oil (refined)       232     450
Ghee                       230     450
Sunflower oil (refined)    227     440
Canola oil                 204     400
Vegetable oil              204     400
Coconut oil (refined)      204     400
Extra virgin olive oil     191     375
Lard                       188     370
Coconut oil (virgin)       177     350
Sesame oil (unrefined)     177     350
Butter                     150     302
Flaxseed oil               107     225";

fn smoke_points(_: &str, o: &mut Opts) -> Result<Output, String> {
    reference(SMOKE_POINTS, o.field(0))
}

const SUBSTITUTES: &str = "\
Instead of                Use
1 cup buttermilk          1 cup milk + 1 tbsp lemon juice or vinegar, rested 5 minutes
1 egg (baking)            1 tbsp ground flax + 3 tbsp water, or ¼ cup applesauce
1 cup butter              ⅞ cup oil, or 1 cup margarine
1 cup sugar               ¾ cup honey (less liquid elsewhere), or 1 cup brown sugar
1 cup brown sugar         1 cup sugar + 1 tbsp molasses
1 tsp baking powder       ¼ tsp baking soda + ½ tsp cream of tartar
1 cup cake flour          1 cup flour − 2 tbsp, + 2 tbsp cornstarch
1 cup self-rising flour   1 cup flour + 1½ tsp baking powder + ¼ tsp salt
1 tbsp cornstarch         2 tbsp flour (to thicken)
1 cup heavy cream         ¾ cup milk + ¼ cup melted butter (not for whipping)
1 cup sour cream          1 cup plain yogurt
1 cup wine                1 cup broth + 1 tbsp vinegar
1 tbsp fresh herbs        1 tsp dried herbs
1 clove garlic            ⅛ tsp garlic powder
1 cup milk                ½ cup evaporated milk + ½ cup water
1 oz chocolate            3 tbsp cocoa + 1 tbsp butter or oil
1 tsp lemon juice         ½ tsp vinegar
1 cup breadcrumbs         ¾ cup crushed crackers or oats";

fn substitutes(_: &str, o: &mut Opts) -> Result<Output, String> {
    reference(SUBSTITUTES, o.field(0))
}

/// The rows of a reference table that match `query`. The table is written
/// in columns at least two spaces apart, a header first; a blank line starts
/// another table, and a line without columns is a remark about its table.
pub fn reference(table: &str, query: &str) -> Result<Output, String> {
    let q = query.trim().to_lowercase();
    let cells = |l: &str| {
        l.split("  ")
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let mut out = Output::default();
    for section in table.split("\n\n") {
        let mut lines = section.lines();
        let head = cells(lines.next().unwrap_or(""));
        let (rows, notes): (Vec<&str>, Vec<&str>) = lines.partition(|l| l.contains("  "));
        let rows: Vec<Vec<String>> = rows
            .into_iter()
            .filter(|l| l.to_lowercase().contains(&q))
            .map(cells)
            .collect();
        if !rows.is_empty() {
            out = out.table(&head.iter().map(String::as_str).collect::<Vec<_>>(), rows);
            out = notes.into_iter().fold(out, Output::note);
        }
    }
    if out.blocks.is_empty() {
        return Err(format!("Nothing matches “{query}”."));
    }
    Ok(out)
}

// Automotive, travel and geo

fn fuel_cost(_: &str, o: &mut Opts) -> Result<Output, String> {
    let distance = o.positive(0)?;
    let eff = o.positive(1)?;
    let price = o.positive(2)?;
    let fuel = match o.mode {
        "mpg" => distance / eff,
        "kmpl" => distance / eff,
        _ => distance * eff / 100.0,
    };
    let unit = if o.mode == "mpg" { "gallons" } else { "litres" };
    let mut r = vec![
        ("Fuel", format!("{} {unit}", fmt(fuel, 2))),
        ("Cost", money(fuel * price)),
    ];
    if let Some(p) = o.opt_num(3)?.filter(|p| *p > 1.0) {
        r.push(("Each, split", money(fuel * price / p)));
    }
    Ok(table(&r))
}

fn car_loan(_: &str, o: &mut Opts) -> Result<Output, String> {
    let price = o.positive(0)?;
    let tax = o.opt_num(4)?.unwrap_or(0.0) / 100.0;
    let down = o.opt_num(1)?.unwrap_or(0.0);
    let trade = o.opt_num(5)?.unwrap_or(0.0);
    let principal = price * (1.0 + tax) - down - trade;
    let rate = o.num(2)? / 100.0 / 12.0;
    let n = o.positive(3)?;
    let m = payment(principal, rate, n);
    Ok(table(&[
        ("Loan", money(principal)),
        ("Monthly payment", money(m)),
        ("Total interest", money(m * n - principal)),
        ("Total cost", money(m * n + down + trade)),
    ]))
}

fn depreciation(_: &str, o: &mut Opts) -> Result<Output, String> {
    let price = o.positive(0)?;
    let years = o.positive(1)?;
    let first = o.opt_num(2)?.unwrap_or(20.0) / 100.0;
    let later = o.opt_num(3)?.unwrap_or(15.0) / 100.0;
    let mut v = price;
    let mut out = Vec::new();
    for y in 1..=(years.ceil() as usize).min(30) {
        v *= 1.0 - if y == 1 { first } else { later };
        out.push(vec![y.to_string(), money(v)]);
    }
    Ok(table(&[
        ("Value", money(v)),
        (
            "Lost",
            format!("{} ({})", money(price - v), pct((1.0 - v / price) * 100.0)),
        ),
    ])
    .table(&["Year", "Value"], out))
}

fn ev_range(_: &str, o: &mut Opts) -> Result<Output, String> {
    let kwh = o.positive(0)?;
    let eff = o.positive(1)?;
    let soc = o.opt_num(2)?.unwrap_or(100.0) / 100.0;
    let factor: f64 = o.mode.parse().unwrap_or(1.0);
    let km = kwh * soc / eff * 100.0 * factor;
    Ok(table(&[(
        "Range",
        format!("{} km · {} mi", fmt(km, 0), fmt(km / 1.609344, 0)),
    )]))
}

/// A tire size such as `225/45R17`: width mm, aspect %, rim inches.
fn tire(s: &str) -> Option<(f64, f64, f64)> {
    let re =
        regex_lite::Regex::new(r"(?i)(\d{3})\s*/\s*(\d{2})\s*[A-Z]*\s*(\d{2}(?:\.\d)?)").unwrap();
    let c = re.captures(s)?;
    Some((c[1].parse().ok()?, c[2].parse().ok()?, c[3].parse().ok()?))
}

fn tire_size(_: &str, o: &mut Opts) -> Result<Output, String> {
    let a = tire(o.field(0)).ok_or("Write a size such as 225/45R17.")?;
    let dia = |(w, r, rim): (f64, f64, f64)| rim * 25.4 + 2.0 * w * r / 100.0;
    let d = dia(a);
    let mut rows = vec![
        (
            "Diameter",
            format!("{} mm ({} in)", fmt(d, 1), fmt(d / 25.4, 2)),
        ),
        ("Sidewall", format!("{} mm", fmt(a.0 * a.1 / 100.0, 1))),
        (
            "Circumference",
            format!("{} mm", fmt(d * std::f64::consts::PI, 0)),
        ),
        (
            "Revolutions per km",
            fmt(1e6 / (d * std::f64::consts::PI), 0),
        ),
    ];
    if let Some(b) = tire(o.field(1)) {
        let d2 = dia(b);
        rows.push((
            "Second diameter",
            format!("{} mm ({:+.1}%)", fmt(d2, 1), (d2 / d - 1.0) * 100.0),
        ));
        rows.push((
            "Speedometer at 100",
            format!("you go {}", fmt(100.0 * d2 / d, 1)),
        ));
    }
    Ok(table(&rows))
}

fn towing(_: &str, o: &mut Opts) -> Result<Output, String> {
    let cap = o.positive(0)?;
    let trailer = o.positive(1)?;
    let cargo = o.opt_num(2)?.unwrap_or(0.0);
    let load = trailer + cargo;
    let share = load / cap;
    let verdict = match share {
        s if s > 1.0 => "✗ Over the towing capacity.",
        s if s > 0.8 => "⚠ Over 80% of capacity: allowed, but leave a margin.",
        _ => "✓ Within 80% of capacity.",
    };
    Ok(table(&[
        ("Load", fmt(load, 0)),
        ("Of capacity", pct(share * 100.0)),
        (
            "Tongue weight (10–15%)",
            format!("{} – {}", fmt(load * 0.1, 0), fmt(load * 0.15, 0)),
        ),
        ("", verdict.into()),
    ]))
}

fn luggage(text: &str, o: &mut Opts) -> Result<Output, String> {
    let items = rows(text, 1)?;
    let limit = o.positive(0)?;
    let lb = o.mode == "lb";
    let total: f64 = items.iter().map(|i| i.1[0]).sum();
    let (kg, pounds) = if lb {
        (total * 0.45359237, total)
    } else {
        (total, total / 0.45359237)
    };
    let verdict = if total > limit {
        format!("✗ {} over the limit.", fmt(total - limit, 2))
    } else {
        format!("✓ {} to spare.", fmt(limit - total, 2))
    };
    Ok(table(&[
        (
            "Total",
            format!("{} kg · {} lb", fmt(kg, 2), fmt(pounds, 2)),
        ),
        ("", verdict),
    ]))
}

fn travel_budget(_: &str, o: &mut Opts) -> Result<Output, String> {
    let days = o.positive(0)?;
    let people = o.opt_num(1)?.unwrap_or(1.0).max(1.0);
    let lodging = o.opt_num(2)?.unwrap_or(0.0);
    let food = o.opt_num(3)?.unwrap_or(0.0);
    let local = o.opt_num(4)?.unwrap_or(0.0);
    let fares = o.opt_num(5)?.unwrap_or(0.0);
    let total = lodging * days + (food + local) * days * people + fares * people;
    Ok(table(&[
        ("Total", money(total)),
        ("Per person", money(total / people)),
        ("Per day", money(total / days)),
        ("With 10% for surprises", money(total * 1.1)),
    ]))
}

pub fn haversine(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    6371.0088 * 2.0 * a.sqrt().atan2((1.0 - a).sqrt())
}

/// A coordinate in decimal degrees or degrees, minutes and seconds.
pub fn coordinate(s: &str) -> Option<f64> {
    let t = s.trim().to_uppercase();
    let neg = t.starts_with('-') || t.ends_with('S') || t.ends_with('W');
    let nums: Vec<f64> = t
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter(|p| !p.is_empty())
        .map(|p| p.parse().ok())
        .collect::<Option<_>>()?;
    let v = match nums.as_slice() {
        [d] => *d,
        [d, m] => d + m / 60.0,
        [d, m, s] => d + m / 60.0 + s / 3600.0,
        _ => return None,
    };
    Some(if neg { -v } else { v })
}

fn coords(o: &Opts, i: usize) -> Result<(f64, f64), String> {
    let lat = coordinate(o.field(i))
        .filter(|v| v.abs() <= 90.0)
        .ok_or("A latitude is between -90 and 90.")?;
    let lon = coordinate(o.field(i + 1))
        .filter(|v| v.abs() <= 180.0)
        .ok_or("A longitude is between -180 and 180.")?;
    Ok((lat, lon))
}

fn distance(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (a, b) = (coords(o, 0)?, coords(o, 2)?);
    let km = haversine(a.0, a.1, b.0, b.1);
    Ok(table(&[
        ("Great-circle distance", format!("{} km", fmt(km, 2))),
        (
            "",
            format!(
                "{} mi · {} nautical mi",
                fmt(km / 1.609344, 2),
                fmt(km / 1.852, 2)
            ),
        ),
        ("Initial bearing", format!("{}°", fmt(bearing_deg(a, b), 1))),
    ]))
}

fn bearing_deg(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (p1, p2) = (a.0.to_radians(), b.0.to_radians());
    let dl = (b.1 - a.1).to_radians();
    let y = dl.sin() * p2.cos();
    let x = p1.cos() * p2.sin() - p1.sin() * p2.cos() * dl.cos();
    (y.atan2(x).to_degrees() + 360.0) % 360.0
}

fn bearing(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (a, b) = (coords(o, 0)?, coords(o, 2)?);
    let deg = bearing_deg(a, b);
    let points = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
        "NW", "NNW",
    ];
    Ok(table(&[
        (
            "Initial bearing",
            format!(
                "{}° ({})",
                fmt(deg, 2),
                points[((deg / 22.5).round() as usize) % 16]
            ),
        ),
        (
            "Final bearing",
            format!("{}°", fmt((bearing_deg(b, a) + 180.0) % 360.0, 2)),
        ),
        (
            "Back bearing",
            format!("{}°", fmt((deg + 180.0) % 360.0, 2)),
        ),
    ]))
}

fn dms(v: f64, pos: char, neg: char) -> String {
    let a = v.abs();
    let d = a.floor();
    let m = ((a - d) * 60.0).floor();
    let s = (a - d - m / 60.0) * 3600.0;
    format!("{d}°{m:02}′{s:05.2}″{}", if v < 0.0 { neg } else { pos })
}

fn coordinate_convert(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (lat, lon) = coords(o, 0)?;
    Ok(table(&[
        ("Decimal degrees", format!("{lat:.6}, {lon:.6}")),
        (
            "Degrees, minutes, seconds",
            format!("{} {}", dms(lat, 'N', 'S'), dms(lon, 'E', 'W')),
        ),
        (
            "Degrees, decimal minutes",
            format!(
                "{}° {:.4}′ {}, {}° {:.4}′ {}",
                lat.abs().floor(),
                lat.abs().fract() * 60.0,
                if lat < 0.0 { 'S' } else { 'N' },
                lon.abs().floor(),
                lon.abs().fract() * 60.0,
                if lon < 0.0 { 'W' } else { 'E' }
            ),
        ),
    ]))
}

fn flight_time(_: &str, o: &mut Opts) -> Result<Output, String> {
    let km = match o.mode {
        "coords" => {
            let (a, b) = (coords(o, 1)?, coords(o, 3)?);
            haversine(a.0, a.1, b.0, b.1)
        }
        _ => o.positive(0)?,
    };
    let speed = 850.0;
    let hours = km / speed + 0.5;
    Ok(table(&[
        ("Distance", format!("{} km", fmt(km, 0))),
        (
            "Flight time",
            format!("about {}", super::dates::duration_words(hours * 3600.0)),
        ),
    ]))
}

// The menus

pub const FINANCE: &[Tool] = &[
    tool(
        "loan",
        "Loan calculator",
        "The monthly payment, total interest and total cost of a loan.",
        loan,
    )
    .form()
    .fields(&[
        ("Amount", "20000"),
        ("Yearly interest rate (%)", "6.5"),
        ("Years", "5"),
    ]),
    tool(
        "mortgage",
        "Mortgage calculator",
        "The monthly payment of a mortgage, with taxes and insurance.",
        mortgage,
    )
    .form()
    .fields(&[
        ("Home price", "400000"),
        ("Down payment (%)", "20"),
        ("Yearly interest rate (%)", "6.5"),
        ("Years", "30"),
        ("Property tax a year", "4000"),
        ("Insurance a year", "1200"),
    ]),
    tool(
        "amortization",
        "Amortization schedule",
        "How each payment splits into principal and interest.",
        amortization,
    )
    .form()
    .fields(&[
        ("Amount", "200000"),
        ("Yearly interest rate (%)", "6"),
        ("Years", "30"),
        ("Extra a month", ""),
    ])
    .selects(&[(
        "Show",
        &[("yearly", "Each year"), ("monthly", "Each month")],
    )]),
    tool(
        "compound",
        "Compound interest",
        "Growth with compounding and monthly contributions.",
        compound,
    )
    .form()
    .fields(&[
        ("Starting amount", "10000"),
        ("Yearly rate (%)", "7"),
        ("Years", "10"),
        ("Added each month", "200"),
    ])
    .selects(&[(
        "Compounded",
        &[
            ("monthly", "Monthly"),
            ("yearly", "Yearly"),
            ("quarterly", "Quarterly"),
            ("daily", "Daily"),
        ],
    )]),
    tool(
        "simple-interest",
        "Simple interest",
        "Interest without compounding.",
        simple_interest,
    )
    .form()
    .fields(&[
        ("Principal", "5000"),
        ("Yearly rate (%)", "4"),
        ("Years", "3"),
    ]),
    tool(
        "effective-rate",
        "Effective interest rate",
        "The APY of a nominal rate for each compounding.",
        effective_rate,
    )
    .form()
    .fields(&[("Nominal yearly rate (%)", "12")]),
    tool(
        "savings-goal",
        "Savings goal",
        "How much to save each month to reach a goal.",
        savings_goal,
    )
    .form()
    .fields(&[
        ("Goal", "25000"),
        ("Saved already", "2000"),
        ("Yearly return (%)", "4"),
        ("Years", "3"),
    ]),
    tool(
        "retirement",
        "Retirement calculator",
        "Savings at retirement, and the income they give.",
        retirement,
    )
    .form()
    .fields(&[
        ("Age", "30"),
        ("Retirement age", "65"),
        ("Savings", "20000"),
        ("Saved each month", "500"),
        ("Yearly return (%)", "6"),
        ("Withdrawal rate (%)", "4"),
    ]),
    tool(
        "fire",
        "FIRE calculator",
        "Your financial independence number, and the years to reach it.",
        fire,
    )
    .form()
    .fields(&[
        ("Yearly spending", "40000"),
        ("Invested now", "50000"),
        ("Saved each year", "30000"),
        ("Yearly return (%)", "5"),
        ("Safe withdrawal rate (%)", "4"),
    ]),
    tool(
        "debt-payoff",
        "Debt payoff",
        "How long a debt takes to pay off, and its interest.",
        debt_payoff,
    )
    .form()
    .fields(&[
        ("Balance", "8000"),
        ("Yearly rate (%)", "19.9"),
        ("Monthly payment", "300"),
    ]),
    tool(
        "inflation",
        "Inflation calculator",
        "What money will cost, and buy, after years of inflation.",
        inflation,
    )
    .form()
    .fields(&[
        ("Amount", "100"),
        ("Yearly inflation (%)", "3"),
        ("Years", "10"),
    ]),
    tool(
        "npv",
        "NPV calculator",
        "The net present value of cash flows.",
        npv,
    )
    .text("Cash flows, first one now (negative for money spent)")
    .fields(&[("Discount rate (%)", "8")]),
    tool(
        "irr",
        "IRR calculator",
        "The internal rate of return of cash flows.",
        irr,
    )
    .text("Cash flows, first one now (negative for money spent)"),
    tool(
        "roi",
        "ROI calculator",
        "The return on an investment, total and per year.",
        roi,
    )
    .form()
    .fields(&[
        ("Invested", "10000"),
        ("Worth now", "14000"),
        ("Years (optional)", "3"),
    ]),
    tool(
        "break-even",
        "Break-even point",
        "The units to sell to cover fixed costs.",
        break_even,
    )
    .form()
    .fields(&[
        ("Fixed costs", "50000"),
        ("Price per unit", "25"),
        ("Variable cost per unit", "10"),
    ]),
    tool(
        "margin",
        "Profit margin",
        "Margin and markup from cost and price.",
        margin,
    )
    .form()
    .fields(&[("Cost", "60"), ("Price", "100")]),
    tool(
        "discount",
        "Discount calculator",
        "A price after discounts and tax.",
        discount,
    )
    .form()
    .fields(&[
        ("Price", "80"),
        ("Discount (%)", "25"),
        ("Extra discount (%)", ""),
        ("Sales tax (%)", ""),
    ]),
    tool(
        "vat",
        "VAT calculator",
        "Adds or removes VAT or sales tax.",
        vat,
    )
    .form()
    .fields(&[("Amount", "100"), ("Rate (%)", "20")])
    .selects(&[(
        "The amount",
        &[
            ("add", "excludes tax: add it"),
            ("remove", "includes tax: remove it"),
        ],
    )]),
    tool(
        "tip",
        "Tip calculator",
        "The tip, the total and each person's share.",
        tip,
    )
    .form()
    .fields(&[("Bill", "84.50"), ("Tip (%)", "18"), ("People", "3")]),
    tool(
        "salary",
        "Salary converter",
        "Hourly, daily, weekly, monthly and yearly pay.",
        salary,
    )
    .form()
    .fields(&[
        ("Pay", "25"),
        ("Hours a week", "40"),
        ("Weeks a year", "52"),
    ])
    .selects(&[(
        "Pay is per",
        &[
            ("hour", "Hour"),
            ("day", "Day"),
            ("week", "Week"),
            ("month", "Month"),
            ("year", "Year"),
        ],
    )]),
    tool(
        "crypto-profit",
        "Crypto profit and loss",
        "Profit on a trade, with fees.",
        crypto_profit,
    )
    .form()
    .fields(&[
        ("Invested", "1000"),
        ("Buy price", "30000"),
        ("Sell price", "36000"),
        ("Fee per trade (%)", "0.1"),
    ]),
    tool(
        "invoice",
        "Invoice calculator",
        "Line totals, discount, tax and total.",
        invoice,
    )
    .text("Item, quantity, price on each line")
    .fields(&[("Tax (%)", "20"), ("Discount (%)", "")]),
    tool(
        "subscriptions",
        "Subscription costs",
        "What subscriptions cost a month and a year.",
        subscriptions,
    )
    .text("Name, price, and “yearly” or “weekly” if not monthly, on each line"),
    tool("net-worth", "Net worth", "Assets minus debts.", net_worth)
        .text("Name: value on each line, debts negative"),
    tool(
        "emergency-fund",
        "Emergency fund",
        "The savings to cover months of expenses.",
        emergency_fund,
    )
    .form()
    .fields(&[
        ("Monthly expenses", "2500"),
        ("Months to cover", "6"),
        ("Saved already", "3000"),
        ("Saving each month", "400"),
    ]),
    tool(
        "rent-vs-buy",
        "Rent or buy",
        "Compares owning a home with renting and investing the difference.",
        rent_vs_buy,
    )
    .form()
    .fields(&[
        ("Home price", "350000"),
        ("Down payment (%)", "20"),
        ("Mortgage rate (%)", "6.5"),
        ("Years", "10"),
        ("Monthly rent", "1600"),
        ("Price and rent growth (%)", "3"),
        ("Investment return (%)", "6"),
    ]),
    tool(
        "late-payment",
        "Late payment interest",
        "Interest and fees on an overdue invoice.",
        late_payment,
    )
    .form()
    .fields(&[
        ("Amount due", "5000"),
        ("Yearly interest (%)", "8"),
        ("Days late", "45"),
        ("Fixed fee", "40"),
    ]),
    tool(
        "currency-format",
        "Currency formatter",
        "An amount written the way different countries do.",
        currency_format,
    )
    .form()
    .fields(&[
        ("Amount", "1234567.891"),
        ("Symbol", "€"),
        ("Decimals", "2"),
    ])
    .selects(&[
        (
            "Style",
            &[
                ("us", "1,234.56"),
                ("eu", "1.234,56"),
                ("fr", "1 234,56"),
                ("ch", "1’234.56"),
                ("in", "12,34,567.89"),
            ],
        ),
        (
            "Symbol goes",
            &[
                ("before", "Before"),
                ("after", "After"),
                ("code", "After, as a code"),
            ],
        ),
    ]),
];

pub const BUSINESS: &[Tool] = &[
    tool(
        "cac",
        "Customer acquisition cost",
        "What winning a customer costs.",
        cac,
    )
    .form()
    .fields(&[
        ("Marketing spend", "20000"),
        ("Sales spend", "10000"),
        ("New customers", "150"),
    ]),
    tool(
        "churn",
        "Churn rate",
        "Customers lost over a period.",
        churn,
    )
    .form()
    .fields(&[
        ("Customers at the start", "1000"),
        ("Customers at the end", "950"),
        ("New customers", "80"),
    ]),
    tool(
        "conversion",
        "Conversion rate",
        "Conversions as a share of visitors.",
        conversion,
    )
    .form()
    .fields(&[
        ("Visitors", "12000"),
        ("Conversions", "360"),
        ("Value per conversion", "45"),
    ]),
    tool(
        "ltv",
        "Customer lifetime value",
        "LTV from revenue, margin and churn, and against CAC.",
        ltv,
    )
    .form()
    .fields(&[
        ("Revenue per customer a month", "50"),
        ("Gross margin (%)", "80"),
        ("Monthly churn (%)", "3"),
        ("CAC (optional)", "300"),
    ]),
    tool(
        "unit-economics",
        "Unit economics",
        "Contribution margin, LTV:CAC and payback.",
        unit_economics,
    )
    .form()
    .fields(&[
        ("Price a month", "49"),
        ("Cost to serve a month", "9"),
        ("CAC", "400"),
        ("Monthly churn (%)", "2.5"),
    ]),
    tool(
        "freelance-rate",
        "Freelance rate",
        "The hourly rate that pays a target income.",
        freelance_rate,
    )
    .form()
    .fields(&[
        ("Take-home income a year", "60000"),
        ("Business expenses a year", "8000"),
        ("Tax (%)", "25"),
        ("Billable hours a week", "25"),
        ("Working weeks a year", "46"),
    ]),
    tool(
        "meeting-cost",
        "Meeting cost",
        "What a meeting costs in salaries.",
        meeting_cost,
    )
    .form()
    .fields(&[
        ("Attendees", "8"),
        ("Average yearly salary", "75000"),
        ("Minutes", "60"),
    ]),
    tool(
        "quote",
        "Project quote",
        "A quote from tasks, hours and a rate.",
        quote,
    )
    .text("Task, hours on each line")
    .fields(&[("Hourly rate", "85"), ("Contingency (%)", "15")]),
    tool(
        "pert",
        "Task estimator",
        "A three-point (PERT) estimate with its uncertainty.",
        pert,
    )
    .form()
    .fields(&[
        ("Optimistic", "3"),
        ("Most likely", "5"),
        ("Pessimistic", "12"),
    ]),
    tool(
        "cap-rate",
        "Cap rate",
        "Net operating income as a share of a property's price.",
        cap_rate,
    )
    .form()
    .fields(&[
        ("Property price", "300000"),
        ("Monthly rent", "2200"),
        ("Yearly expenses", "7000"),
    ]),
    tool(
        "cash-flow",
        "Rental cash flow",
        "Monthly cash flow of a rental property.",
        cash_flow,
    )
    .form()
    .fields(&[
        ("Monthly rent", "2000"),
        ("Mortgage payment", "1100"),
        ("Taxes and insurance a month", "300"),
        ("Vacancy (%)", "5"),
        ("Maintenance (%)", "8"),
        ("Management (%)", "8"),
    ]),
    tool(
        "rental-yield",
        "Rental yield",
        "Gross and net yield of a rental.",
        rental_yield,
    )
    .form()
    .fields(&[
        ("Property price", "250000"),
        ("Monthly rent", "1500"),
        ("Yearly costs", "3000"),
    ]),
    tool(
        "affordability",
        "Home affordability",
        "The home price an income can carry.",
        affordability,
    )
    .form()
    .fields(&[
        ("Yearly income", "90000"),
        ("Monthly debt payments", "400"),
        ("Down payment", "60000"),
        ("Mortgage rate (%)", "6.5"),
        ("Years", "30"),
        ("Taxes and insurance (% a year)", "1.5"),
    ]),
    tool(
        "flip",
        "House flipping",
        "The 70% rule offer, and a flip's profit.",
        flip,
    )
    .form()
    .fields(&[
        ("After-repair value", "300000"),
        ("Repairs", "40000"),
        ("Purchase price (optional)", "160000"),
        ("Holding costs", "8000"),
        ("Selling costs (%)", "8"),
    ]),
    tool(
        "closing-costs",
        "Closing costs",
        "An estimate of closing costs and cash to close.",
        closing_costs,
    )
    .form()
    .fields(&[("Home price", "350000"), ("Down payment (%)", "20")]),
    tool(
        "property-tax",
        "Property tax",
        "Property tax from value and rate.",
        property_tax,
    )
    .form()
    .fields(&[
        ("Assessed value", "300000"),
        ("Rate", "1.1"),
        ("Exemption", ""),
    ])
    .selects(&[(
        "Rate is",
        &[("percent", "A percentage"), ("mill", "Mills (per 1,000)")],
    )]),
    tool(
        "rent-increase",
        "Rent increase",
        "A new rent from a percentage, or the percentage of a new rent.",
        rent_increase,
    )
    .form()
    .fields(&[("Current rent", "1500"), ("Increase", "4")])
    .selects(&[(
        "Increase is",
        &[("percent", "A percentage"), ("amount", "The new rent")],
    )]),
    tool(
        "rental-depreciation",
        "Rental depreciation",
        "Straight-line depreciation of a rental building.",
        rental_depreciation,
    )
    .form()
    .fields(&[("Cost of the property", "300000"), ("Land value", "60000")])
    .selects(&[(
        "Property",
        &[
            ("residential", "Residential (27.5 years)"),
            ("commercial", "Commercial (39 years)"),
        ],
    )]),
    tool(
        "vacancy",
        "Vacancy rate",
        "Vacancy and occupancy, and rent lost.",
        vacancy,
    )
    .form()
    .fields(&[
        ("Units (or days)", "40"),
        ("Vacant", "3"),
        ("Rent per unit", "1200"),
    ]),
    tool(
        "repair-replace",
        "Repair or replace",
        "The 50% rule and repair × age rule.",
        repair_replace,
    )
    .form()
    .fields(&[
        ("Repair cost", "400"),
        ("Replacement cost", "1200"),
        ("Age (years)", "8"),
        ("Expected lifespan (years)", "12"),
    ]),
    tool(
        "brrrr",
        "BRRRR calculator",
        "Buy, rehab, rent, refinance: the cash left in the deal.",
        brrrr,
    )
    .form()
    .fields(&[
        ("Purchase price", "150000"),
        ("Rehab cost", "40000"),
        ("After-repair value", "250000"),
        ("Refinance loan-to-value (%)", "75"),
    ]),
    tool(
        "renovation-roi",
        "Renovation ROI",
        "How much of a renovation's cost it adds in value.",
        renovation_roi,
    )
    .form()
    .fields(&[("Cost", "25000"), ("Value added", "20000")]),
];

pub const HEALTH: &[Tool] = &[
    tool(
        "bmi",
        "BMI calculator",
        "Body mass index and the normal range for a height.",
        bmi,
    )
    .form()
    .fields(&[("Weight", "70"), ("Height", "175")])
    .selects(&[("Units", UNITS)]),
    tool(
        "bmr",
        "BMR calculator",
        "Calories burned at rest (Mifflin–St Jeor).",
        bmr,
    )
    .form()
    .fields(&[("Age", "35"), ("Weight", "70"), ("Height", "175")])
    .selects(&[("Units", UNITS), ("Sex", SEX)]),
    tool(
        "tdee",
        "Calorie calculator (TDEE)",
        "Daily calories to keep, lose or gain weight.",
        tdee,
    )
    .form()
    .fields(&[("Age", "35"), ("Weight", "70"), ("Height", "175")])
    .selects(&[("Units", UNITS), ("Sex", SEX), ("Activity", ACTIVITY)]),
    tool(
        "macros",
        "Macro calculator",
        "Grams of protein, carbohydrates and fat for a calorie target.",
        macros,
    )
    .form()
    .fields(&[("Calories a day", "2200")])
    .selects(&[(
        "Diet",
        &[
            ("balanced", "Balanced"),
            ("high-protein", "High protein"),
            ("low-carb", "Low carb"),
            ("keto", "Keto"),
        ],
    )]),
    tool(
        "body-fat",
        "Body fat (US Navy)",
        "Body fat from height, neck, waist and hip.",
        body_fat,
    )
    .form()
    .fields(&[
        ("Height", "175"),
        ("Neck", "38"),
        ("Waist (at the navel)", "85"),
        ("Hip (women)", "95"),
    ])
    .selects(&[
        (
            "Units",
            &[("metric", "Centimetres"), ("imperial", "Inches")],
        ),
        ("Sex", SEX),
    ]),
    tool(
        "ideal-weight",
        "Ideal weight",
        "Ideal weight by the Devine, Robinson, Miller and Hamwi formulas.",
        ideal_weight,
    )
    .form()
    .fields(&[("Height", "175")])
    .selects(&[
        (
            "Units",
            &[("metric", "Centimetres"), ("imperial", "Inches")],
        ),
        ("Sex", SEX),
    ]),
    tool(
        "bsa",
        "Body surface area",
        "BSA by the Mosteller, Du Bois and Haycock formulas.",
        bsa,
    )
    .form()
    .fields(&[("Weight", "70"), ("Height", "175")])
    .selects(&[("Units", UNITS)]),
    tool(
        "bac",
        "Blood alcohol estimate",
        "An estimate of blood alcohol (Widmark).",
        bac,
    )
    .form()
    .fields(&[
        ("Standard drinks", "3"),
        ("Weight", "70"),
        ("Hours since the first", "2"),
    ])
    .selects(&[
        ("Units", &[("metric", "Kilograms"), ("imperial", "Pounds")]),
        ("Sex", SEX),
    ]),
    tool(
        "heart-zones",
        "Heart rate zones",
        "Five training zones from age and resting heart rate.",
        heart_zones,
    )
    .form()
    .fields(&[
        ("Age", "35"),
        ("Resting heart rate", "60"),
        ("Maximum (if known)", ""),
    ]),
    tool(
        "one-rep-max",
        "One-rep max",
        "The most you could lift once, and training loads.",
        one_rep_max,
    )
    .form()
    .fields(&[("Weight lifted", "80"), ("Repetitions", "6")]),
    tool(
        "pace",
        "Pace calculator",
        "Pace and speed from distance and time, with race predictions.",
        pace,
    )
    .form()
    .fields(&[("Distance", "10"), ("Time (h:mm:ss)", "52:30")])
    .selects(&[("Distance in", &[("km", "Kilometres"), ("mi", "Miles")])]),
    tool(
        "splits",
        "Split times",
        "Even splits for a goal time.",
        splits,
    )
    .form()
    .fields(&[
        ("Total distance", "10"),
        ("Goal time (h:mm:ss)", "50:00"),
        ("Split every", "1"),
    ]),
    tool(
        "due-date",
        "Pregnancy due date",
        "The due date and how far along a pregnancy is.",
        due_date,
    )
    .form()
    .fields(&[
        ("First day of the last period", "2024-01-10"),
        ("Cycle length (days)", "28"),
        ("On (empty: today)", ""),
    ]),
    tool(
        "ovulation",
        "Ovulation calculator",
        "Fertile windows for the next three cycles.",
        ovulation,
    )
    .form()
    .fields(&[
        ("First day of the last period", "2024-01-10"),
        ("Cycle length (days)", "28"),
    ]),
    tool(
        "protein",
        "Protein intake",
        "Daily protein for a weight and activity.",
        protein,
    )
    .form()
    .fields(&[("Weight", "70")])
    .selects(&[
        (
            "Activity",
            &[
                ("sedentary", "Sedentary"),
                ("active", "Active"),
                ("athlete", "Athlete or building muscle"),
                ("older", "Over 65"),
            ],
        ),
        ("Weight in", &[("kg", "Kilograms"), ("lb", "Pounds")]),
    ]),
    tool("water", "Water intake", "How much to drink a day.", water)
        .form()
        .fields(&[("Weight", "70"), ("Exercise (minutes a day)", "30")])
        .selects(&[("Weight in", &[("kg", "Kilograms"), ("lb", "Pounds")])]),
    tool(
        "sleep",
        "Sleep calculator",
        "Bedtimes or wake-up times in whole 90-minute cycles.",
        sleep,
    )
    .form()
    .fields(&[("Time (hh:mm)", "07:00"), ("Minutes to fall asleep", "15")])
    .selects(&[(
        "The time is when I",
        &[("wake", "Want to wake up"), ("bed", "Go to bed")],
    )]),
    tool(
        "blood-pressure",
        "Blood pressure",
        "The category of a blood pressure reading.",
        blood_pressure,
    )
    .form()
    .fields(&[("Systolic", "128"), ("Diastolic", "82")]),
    tool(
        "blood-sugar",
        "Blood sugar converter",
        "mg/dL and mmol/L, with fasting ranges.",
        blood_sugar,
    )
    .form()
    .fields(&[("Value", "105")])
    .selects(&[("Unit", &[("mgdl", "mg/dL"), ("mmol", "mmol/L")])]),
    tool(
        "vo2max",
        "VO₂ max",
        "Aerobic fitness from the Cooper test or heart rates.",
        vo2,
    )
    .form()
    .fields(&[
        ("Metres in 12 minutes, or age", "2400"),
        ("Resting heart rate (heart rate method)", "60"),
    ])
    .selects(&[(
        "Method",
        &[
            ("cooper", "Cooper 12-minute run"),
            ("heart", "Heart rate ratio (age, resting)"),
        ],
    )]),
];

pub const HOME: &[Tool] = &[
    tool(
        "paint",
        "Paint calculator",
        "Paint for a room's walls.",
        paint,
    )
    .form()
    .fields(&[
        ("Wall length all round", "16"),
        ("Wall height", "2.5"),
        ("Doors", "1"),
        ("Windows", "2"),
        ("Coats", "2"),
        ("Coverage per litre or gallon", "10"),
    ])
    .selects(&[(
        "Units",
        &[
            ("metric", "Metres, litres"),
            ("imperial", "Feet, gallons (350 sq ft each)"),
        ],
    )]),
    tool(
        "room-area",
        "Room area",
        "Floor area, wall area and volume of a room.",
        room_area,
    )
    .form()
    .fields(&[
        ("Length", "5"),
        ("Width", "4"),
        ("Height (optional)", "2.5"),
    ])
    .selects(&[("Units", &[("m", "Metres"), ("ft", "Feet")])]),
    tool(
        "flooring",
        "Flooring calculator",
        "Flooring with waste, and boxes.",
        flooring,
    )
    .form()
    .fields(&[
        ("Length", "5"),
        ("Width", "4"),
        ("Waste (%)", "10"),
        ("Coverage per box", "2.2"),
    ]),
    tool("tiles", "Tile calculator", "Tiles for an area.", tiles)
        .form()
        .fields(&[
            ("Area length (m)", "3"),
            ("Area width (m)", "2"),
            ("Tile length (cm)", "30"),
            ("Tile width (cm)", "30"),
            ("Joint (mm)", "3"),
            ("Waste (%)", "10"),
        ]),
    tool("grout", "Grout calculator", "Grout for tiles.", grout)
        .form()
        .fields(&[
            ("Tile length (mm)", "300"),
            ("Tile width (mm)", "300"),
            ("Tile thickness (mm)", "8"),
            ("Joint width (mm)", "3"),
            ("Area (m²)", "6"),
        ]),
    tool(
        "wallpaper",
        "Wallpaper calculator",
        "Rolls of wallpaper for a room.",
        wallpaper,
    )
    .form()
    .fields(&[
        ("Wall length all round (m)", "14"),
        ("Height (m)", "2.5"),
        ("Roll width (m)", "0.53"),
        ("Roll length (m)", "10.05"),
        ("Pattern repeat (cm)", "0"),
    ]),
    tool(
        "drywall",
        "Drywall calculator",
        "Sheets, screws, compound and tape.",
        drywall,
    )
    .form()
    .fields(&[
        ("Wall area", "40"),
        ("Sheet length", "2.4"),
        ("Sheet width", "1.2"),
    ]),
    tool(
        "concrete",
        "Concrete calculator",
        "Concrete for a slab, and bags.",
        concrete,
    )
    .form()
    .fields(&[("Length", "3"), ("Width", "3"), ("Depth", "10")])
    .selects(&[(
        "Units",
        &[
            ("metric", "Metres, depth in cm"),
            ("imperial", "Feet, depth in inches"),
        ],
    )]),
    tool("deck", "Deck boards", "Boards and joists for a deck.", deck)
        .form()
        .fields(&[
            ("Deck length (m)", "4"),
            ("Deck width (m)", "3"),
            ("Board width (cm)", "14"),
            ("Gap (cm)", "0.5"),
            ("Board length (m)", "4"),
        ]),
    tool(
        "fence",
        "Fence calculator",
        "Posts, rails and pickets for a fence.",
        fence,
    )
    .form()
    .fields(&[
        ("Fence length", "30"),
        ("Post spacing", "2.4"),
        ("Rails per section", "2"),
        ("Picket width (cm)", "9"),
        ("Gap (cm)", "3"),
    ]),
    tool(
        "board-feet",
        "Lumber calculator",
        "Board feet of lumber, and cost.",
        board_feet,
    )
    .form()
    .fields(&[
        ("Thickness (in)", "2"),
        ("Width (in)", "6"),
        ("Length (ft)", "8"),
        ("Pieces", "10"),
        ("Price per board foot", ""),
    ]),
    tool(
        "mulch",
        "Mulch and soil",
        "Mulch, soil or gravel for a bed.",
        mulch,
    )
    .form()
    .fields(&[("Length", "4"), ("Width", "2"), ("Depth", "7")])
    .selects(&[(
        "Units",
        &[
            ("metric", "Metres, depth in cm"),
            ("imperial", "Feet, depth in inches"),
        ],
    )]),
    tool(
        "stairs",
        "Staircase calculator",
        "Risers, treads, run and angle of stairs.",
        stairs,
    )
    .form()
    .fields(&[
        ("Total rise (cm)", "270"),
        ("Ideal riser (cm)", "18"),
        ("Tread depth (cm)", "28"),
    ]),
    tool(
        "curtains",
        "Curtain size",
        "Curtain width and fabric for a window.",
        curtains,
    )
    .form()
    .fields(&[
        ("Rail width (cm)", "160"),
        ("Drop (cm)", "220"),
        ("Fabric width (cm)", "140"),
    ])
    .selects(&[(
        "Fullness",
        &[
            ("2", "Double (gathered)"),
            ("2.5", "Two and a half (pleated)"),
            ("1.5", "One and a half (flat)"),
            ("3", "Triple (sheer)"),
        ],
    )]),
    tool(
        "ac-size",
        "Air conditioner size",
        "The cooling power a room needs.",
        ac_size,
    )
    .form()
    .fields(&[("Room area", "25"), ("People usually in it", "2")])
    .selects(&[
        ("Area in", &[("m2", "m²"), ("ft2", "sq ft")]),
        (
            "Sun",
            &[
                ("normal", "Normal"),
                ("sunny", "Very sunny"),
                ("shaded", "Shaded"),
            ],
        ),
    ])
    .checks(&["It is a kitchen"]),
    tool(
        "energy-cost",
        "Energy cost",
        "What an appliance costs to run.",
        energy_cost,
    )
    .form()
    .fields(&[
        ("Power (W)", "150"),
        ("Hours a day", "8"),
        ("Price per kWh", "0.25"),
    ]),
    tool(
        "bill-split",
        "Utility bill splitter",
        "A bill split by the days each person was home.",
        bill_split,
    )
    .text("Name, days on each line")
    .fields(&[("Bill", "240")]),
    tool(
        "rent-split",
        "Rent splitter",
        "Rent split by room size, with shared space split equally.",
        rent_split,
    )
    .text("Name, room size on each line")
    .fields(&[("Rent", "2400"), ("Share split equally (%)", "30")]),
    tool(
        "solar",
        "Solar panel estimator",
        "The panels and system size for your use.",
        solar,
    )
    .form()
    .fields(&[
        ("Monthly use (kWh)", "600"),
        ("Peak sun hours a day", "4.5"),
        ("Panel power (W)", "400"),
        ("Cost per watt", "2.8"),
    ]),
    tool(
        "tv-distance",
        "TV size and distance",
        "How far to sit from a screen.",
        tv_distance,
    )
    .form()
    .fields(&[("Diagonal (inches)", "65")]),
    tool(
        "lighting",
        "Room lighting",
        "The lumens a room needs.",
        lighting,
    )
    .form()
    .fields(&[("Floor area", "20")])
    .selects(&[
        (
            "Room",
            &[
                ("150", "Living room"),
                ("300", "Kitchen"),
                ("100", "Bedroom"),
                ("400", "Office or workshop"),
                ("200", "Bathroom"),
                ("75", "Hallway"),
            ],
        ),
        ("Area in", &[("m2", "m²"), ("ft2", "sq ft")]),
    ]),
    tool(
        "moving-boxes",
        "Moving boxes",
        "Roughly how many boxes a move takes.",
        moving_boxes,
    )
    .form()
    .fields(&[("Rooms", "4"), ("People", "2")])
    .selects(&[(
        "Belongings",
        &[("average", "Average"), ("light", "Few"), ("heavy", "Many")],
    )]),
    tool(
        "water-usage",
        "Water usage",
        "A household's daily water use.",
        water_usage,
    )
    .form()
    .fields(&[
        ("People", "3"),
        ("Shower minutes a person a day", "8"),
        ("Flushes a person a day", "5"),
        ("Laundry loads a week", "4"),
        ("Dishwasher loads a week", "5"),
    ]),
    tool(
        "commute-co2",
        "Commute emissions",
        "The CO₂ of a commute by each way of travelling.",
        commute_co2,
    )
    .form()
    .fields(&[
        ("One-way distance (km)", "15"),
        ("Commuting days a year", "220"),
    ]),
    tool(
        "trees",
        "Tree planting impact",
        "The CO₂ trees absorb.",
        trees,
    )
    .form()
    .fields(&[("Trees", "10"), ("Years", "10")]),
    tool(
        "plant-spacing",
        "Plant spacing",
        "How many plants fit a bed.",
        plant_spacing,
    )
    .form()
    .fields(&[
        ("Bed length (m)", "3"),
        ("Bed width (m)", "1.2"),
        ("Spacing (cm)", "30"),
    ])
    .selects(&[(
        "Pattern",
        &[
            ("square", "Square grid"),
            ("triangle", "Staggered (triangles)"),
        ],
    )]),
    tool(
        "lawn-seed",
        "Lawn seed",
        "Grass seed for an area.",
        lawn_seed,
    )
    .form()
    .fields(&[("Area (m²)", "100"), ("Rate (g/m², optional)", "")])
    .selects(&[(
        "Sowing",
        &[("new", "A new lawn"), ("overseed", "Overseeding")],
    )]),
    tool(
        "compost",
        "Compost ratio",
        "The carbon to nitrogen balance of a compost pile.",
        compost,
    )
    .form()
    .fields(&[
        ("Green parts (volumes)", "1"),
        ("Brown parts (volumes)", "2"),
    ]),
    tool(
        "fertilizer",
        "Fertilizer calculator",
        "Product for a nitrogen target, from its N-P-K.",
        fertilizer,
    )
    .form()
    .fields(&[
        ("Nitrogen in the product (%)", "10"),
        ("Area (m²)", "50"),
        ("Nitrogen wanted (g/m²)", "5"),
    ]),
    tool("harvest", "Harvest date", "When a crop is ready.", harvest)
        .form()
        .fields(&[("Sowing date", "2025-04-15"), ("Days to maturity", "75")]),
    tool(
        "seed-start",
        "Seed starting date",
        "When to sow indoors before the last frost.",
        seed_start,
    )
    .form()
    .fields(&[("Last frost date", "2025-04-20"), ("Weeks before", "6")]),
    tool(
        "aquarium",
        "Aquarium volume",
        "Volume and weight of a tank.",
        aquarium,
    )
    .form()
    .fields(&[("Length", "100"), ("Width", "40"), ("Height", "50")])
    .selects(&[("Units", &[("cm", "Centimetres"), ("in", "Inches")])]),
    tool(
        "pet-food",
        "Pet food calculator",
        "Daily calories for a dog or cat, and grams of food.",
        pet_food,
    )
    .form()
    .fields(&[
        ("Weight", "20"),
        ("Food energy (kcal per kg, optional)", "3600"),
    ])
    .selects(&[
        ("Pet", &[("dog", "Dog"), ("cat", "Cat")]),
        (
            "Life stage",
            &[
                ("1.6", "Neutered adult"),
                ("1.8", "Intact adult"),
                ("1.2", "Inactive or overweight"),
                ("2.0", "Active"),
                ("3.0", "Puppy or kitten"),
            ],
        ),
        ("Weight in", &[("kg", "Kilograms"), ("lb", "Pounds")]),
    ]),
    tool(
        "pet-age",
        "Pet age",
        "A dog's or cat's age in human years.",
        pet_age,
    )
    .form()
    .fields(&[("Age (years)", "5")])
    .selects(&[
        ("Pet", &[("dog", "Dog"), ("cat", "Cat")]),
        (
            "Dog size",
            &[("medium", "Medium"), ("small", "Small"), ("large", "Large")],
        ),
    ]),
];

const UNIT_MENU: super::Menu = &[
    ("cup", "Cups"),
    ("tbsp", "Tablespoons"),
    ("tsp", "Teaspoons"),
    ("ml", "Millilitres"),
    ("l", "Litres"),
    ("floz", "Fluid ounces"),
    ("pint", "Pints"),
    ("quart", "Quarts"),
    ("gallon", "Gallons"),
    ("g", "Grams"),
    ("kg", "Kilograms"),
    ("oz", "Ounces"),
    ("lb", "Pounds"),
];

pub const COOKING: &[Tool] = &[
    tool(
        "cooking-units",
        "Cooking units",
        "Cups, spoons, millilitres, grams and ounces, by ingredient.",
        cooking_units,
    )
    .form()
    .fields(&[("Amount", "2")])
    .selects(&[
        ("From", UNIT_MENU),
        (
            "To",
            &[
                ("g", "Grams"),
                ("ml", "Millilitres"),
                ("cup", "Cups"),
                ("tbsp", "Tablespoons"),
                ("tsp", "Teaspoons"),
                ("oz", "Ounces"),
                ("lb", "Pounds"),
                ("l", "Litres"),
                ("kg", "Kilograms"),
                ("floz", "Fluid ounces"),
            ],
        ),
        (
            "Ingredient",
            &[
                ("0.507", "Flour"),
                ("0.845", "Sugar"),
                ("0.93", "Brown sugar"),
                ("0.96", "Butter"),
                ("1.0", "Water"),
                ("1.03", "Milk"),
                ("0.92", "Oil"),
                ("1.42", "Honey"),
                ("0.78", "Rice"),
                ("0.42", "Oats"),
                ("1.2", "Salt"),
                ("0.53", "Cocoa"),
            ],
        ),
    ]),
    tool(
        "oven",
        "Oven temperature",
        "Celsius, Fahrenheit, fan and gas mark.",
        oven,
    )
    .form()
    .fields(&[("Temperature", "350")])
    .selects(&[(
        "In",
        &[("f", "Fahrenheit"), ("c", "Celsius"), ("gas", "Gas mark")],
    )]),
    tool(
        "air-fryer",
        "Air fryer converter",
        "An oven recipe's temperature and time for an air fryer.",
        air_fryer,
    )
    .form()
    .fields(&[("Oven temperature", "400"), ("Oven time (minutes)", "25")])
    .selects(&[("In", &[("f", "Fahrenheit"), ("c", "Celsius")])]),
    tool(
        "recipe-scale",
        "Recipe scaler",
        "Scales the amounts at the start of each ingredient line.",
        recipe_scale,
    )
    .text("Ingredients, one per line")
    .fields(&[("Servings now", "4"), ("Servings wanted", "6")]),
    tool(
        "bakers",
        "Baker's percentages",
        "Dough ingredients from flour weight and percentages.",
        bakers,
    )
    .form()
    .fields(&[
        ("Flour (g)", "1000"),
        ("Water (%)", "70"),
        ("Salt (%)", "2"),
        ("Yeast (%)", "1"),
    ]),
    tool(
        "sourdough",
        "Sourdough hydration",
        "The true hydration of a dough with starter.",
        sourdough,
    )
    .form()
    .fields(&[
        ("Flour (g)", "450"),
        ("Water (g)", "330"),
        ("Starter (g)", "100"),
        ("Starter hydration (%)", "100"),
    ]),
    tool(
        "coffee",
        "Coffee ratio",
        "Coffee and water for a brew ratio.",
        coffee,
    )
    .form()
    .fields(&[("Amount (g)", "500")])
    .selects(&[
        ("The amount is", &[("water", "Water"), ("coffee", "Coffee")]),
        (
            "Ratio",
            &[
                ("16", "1:16 (pour-over)"),
                ("15", "1:15 (strong)"),
                ("17", "1:17 (light)"),
                ("12", "1:12 (French press)"),
                ("2", "1:2 (espresso)"),
                ("8", "1:8 (cold brew concentrate)"),
            ],
        ),
    ]),
    tool(
        "brine",
        "Brine calculator",
        "Salt and sugar for a brine.",
        brine,
    )
    .form()
    .fields(&[
        ("Water (litres)", "2"),
        ("Salt (%)", "6"),
        ("Sugar (%)", "3"),
    ]),
    tool(
        "pan-size",
        "Pan size converter",
        "How much to scale a recipe for another pan.",
        pan_size,
    )
    .form()
    .fields(&[
        ("Recipe pan size", "23"),
        ("Recipe pan, other side (rectangles)", ""),
        ("Your pan size", "20"),
        ("Your pan, other side (rectangles)", ""),
    ])
    .selects(&[
        (
            "Recipe pan",
            &[
                ("round", "Round (diameter)"),
                ("square", "Square"),
                ("rect", "Rectangle"),
            ],
        ),
        (
            "Your pan",
            &[
                ("square", "Square"),
                ("round", "Round (diameter)"),
                ("rect", "Rectangle"),
            ],
        ),
    ]),
    tool(
        "eggs",
        "Egg size converter",
        "Eggs of one size as eggs of another.",
        eggs,
    )
    .form()
    .fields(&[("Eggs", "3")])
    .selects(&[(
        "Size",
        &[
            ("large", "Large"),
            ("small", "Small"),
            ("medium", "Medium"),
            ("xl", "Extra large"),
            ("jumbo", "Jumbo"),
        ],
    )]),
    tool(
        "abv",
        "Homebrew ABV",
        "Alcohol from original and final gravity.",
        abv,
    )
    .form()
    .fields(&[("Original gravity", "1.050"), ("Final gravity", "1.010")]),
    tool(
        "defrost",
        "Defrost time",
        "How long frozen food takes to thaw.",
        defrost,
    )
    .form()
    .fields(&[("Weight", "2")])
    .selects(&[
        (
            "Method",
            &[
                ("fridge", "Fridge"),
                ("water", "Cold water"),
                ("microwave", "Microwave"),
            ],
        ),
        ("Weight in", &[("kg", "Kilograms"), ("lb", "Pounds")]),
    ]),
    tool(
        "doneness",
        "Meat doneness guide",
        "Safe internal temperatures and doneness.",
        doneness,
    )
    .form()
    .fields(&[("Search", "")]),
    tool(
        "smoke-points",
        "Smoke points",
        "The smoke points of cooking oils.",
        smoke_points,
    )
    .form()
    .fields(&[("Search", "")]),
    tool(
        "substitutes",
        "Ingredient substitutions",
        "What to use instead of a missing ingredient.",
        substitutes,
    )
    .form()
    .fields(&[("Search", "")]),
];

pub const AUTO: &[Tool] = &[
    tool(
        "fuel-cost",
        "Fuel cost",
        "The fuel and money a trip takes.",
        fuel_cost,
    )
    .form()
    .fields(&[
        ("Distance", "450"),
        ("Consumption", "6.5"),
        ("Fuel price", "1.80"),
        ("People sharing", ""),
    ])
    .selects(&[(
        "Consumption in",
        &[
            ("l100", "L/100 km (distance in km)"),
            ("mpg", "Miles per gallon (distance in miles)"),
            ("kmpl", "km per litre"),
        ],
    )]),
    tool(
        "car-loan",
        "Car loan",
        "The monthly payment and interest of a car loan.",
        car_loan,
    )
    .form()
    .fields(&[
        ("Price", "30000"),
        ("Down payment", "5000"),
        ("Yearly rate (%)", "7"),
        ("Months", "60"),
        ("Sales tax (%)", "6"),
        ("Trade-in value", ""),
    ]),
    tool(
        "depreciation",
        "Car depreciation",
        "A car's value over the years.",
        depreciation,
    )
    .form()
    .fields(&[
        ("Price new", "35000"),
        ("Years", "5"),
        ("First-year loss (%)", "20"),
        ("Later yearly loss (%)", "15"),
    ]),
    tool(
        "ev-range",
        "EV range",
        "An electric car's range from battery and consumption.",
        ev_range,
    )
    .form()
    .fields(&[
        ("Battery (kWh)", "75"),
        ("Consumption (kWh/100 km)", "17"),
        ("Charge (%)", "90"),
    ])
    .selects(&[(
        "Conditions",
        &[
            ("1.0", "Mild"),
            ("0.85", "Highway"),
            ("0.7", "Winter"),
            ("1.1", "City, mild"),
        ],
    )]),
    tool(
        "tire-size",
        "Tire size",
        "Diameter and circumference of a tire, and a second one compared.",
        tire_size,
    )
    .form()
    .fields(&[
        ("Tire size", "225/45R17"),
        ("Compare with (optional)", "225/40R18"),
    ]),
    tool(
        "towing",
        "Towing capacity check",
        "Whether a trailer is within a vehicle's capacity.",
        towing,
    )
    .form()
    .fields(&[
        ("Towing capacity", "3500"),
        ("Trailer weight", "2400"),
        ("Cargo in the trailer", "300"),
    ]),
    tool(
        "luggage",
        "Luggage weight",
        "The weight of what you pack, against a limit.",
        luggage,
    )
    .text("Item, weight on each line")
    .fields(&[("Limit", "23")])
    .selects(&[("Weights in", &[("kg", "Kilograms"), ("lb", "Pounds")])]),
    tool(
        "travel-budget",
        "Travel budget",
        "The cost of a trip, per person and per day.",
        travel_budget,
    )
    .form()
    .fields(&[
        ("Days", "7"),
        ("People", "2"),
        ("Lodging a night", "120"),
        ("Food a person a day", "45"),
        ("Local transport a person a day", "10"),
        ("Fares a person", "350"),
    ]),
    tool(
        "distance",
        "Distance between coordinates",
        "The great-circle distance between two points.",
        distance,
    )
    .form()
    .fields(&[
        ("From latitude", "48.8566"),
        ("From longitude", "2.3522"),
        ("To latitude", "40.7128"),
        ("To longitude", "-74.0060"),
    ]),
    tool(
        "bearing",
        "Bearing",
        "The compass bearing from one point to another.",
        bearing,
    )
    .form()
    .fields(&[
        ("From latitude", "51.5074"),
        ("From longitude", "-0.1278"),
        ("To latitude", "55.7558"),
        ("To longitude", "37.6173"),
    ]),
    tool(
        "coordinates",
        "Coordinate converter",
        "Decimal degrees and degrees, minutes, seconds.",
        coordinate_convert,
    )
    .form()
    .fields(&[("Latitude", "40°26′46″N"), ("Longitude", "79°58′56″W")]),
    tool(
        "flight-time",
        "Flight time",
        "About how long a flight takes.",
        flight_time,
    )
    .form()
    .fields(&[
        ("Distance (km)", "5800"),
        ("From latitude", "48.8566"),
        ("From longitude", "2.3522"),
        ("To latitude", "40.7128"),
        ("To longitude", "-74.0060"),
    ])
    .selects(&[(
        "From",
        &[("km", "The distance"), ("coords", "The coordinates")],
    )]),
];

#[cfg(test)]
mod tests {
    use super::super::Rng;
    use super::*;

    fn run(f: super::super::Run, fields: &[&str], choices: &[&str]) -> String {
        let mut rng = Rng(1);
        let mut o = Opts::new("", &mut rng).with(fields, choices, &[]);
        f("", &mut o).unwrap().plain()
    }

    #[test]
    fn money_math() {
        assert!(run(loan, &["20000", "6", "5"], &[]).contains("386.66"));
        assert_eq!(payment(1000.0, 0.0, 10.0), 100.0);
        assert!((irr_of(&[-100.0, 110.0]).unwrap() - 0.1).abs() < 1e-6);
        assert!(run(vat, &["120", "20"], &["remove"]).contains("Net:   100"));
    }

    #[test]
    fn recipes() {
        assert_eq!(leading_amount("1 1/2 cups flour").map(|a| a.0), Some(1.5));
        assert_eq!(leading_amount("½ tsp salt").map(|a| a.0), Some(0.5));
        assert_eq!(leading_amount("1/4 cup").map(|a| a.0), Some(0.25));
        let mut rng = Rng(1);
        let mut o = Opts::new("", &mut rng).with(&["2", "3"], &[], &[]);
        assert_eq!(
            recipe_scale("1 cup milk\n1/2 tsp salt\nPinch", &mut o)
                .unwrap()
                .plain(),
            "1½ cup milk\n¾ tsp salt\nPinch"
        );
    }

    #[test]
    fn geo() {
        assert!((haversine(48.8566, 2.3522, 51.5074, -0.1278) - 343.5).abs() < 1.0);
        assert_eq!(
            coordinate("40°26′46″N").map(|v| (v * 1000.0).round()),
            Some(40446.0)
        );
        assert_eq!(tire("225/45R17"), Some((225.0, 45.0, 17.0)));
    }
}
