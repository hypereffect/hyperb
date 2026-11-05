//! Physics, chemistry, astronomy, electronics, 3D printing, photography and
//! music.

use super::Output;
use super::calc::reference;
use super::math::gcd;
use super::{Opts, Tool, fmt, grid, table, tool};

fn sig(v: f64) -> String {
    if v != 0.0 && (v.abs() >= 1e9 || v.abs() < 1e-4) {
        format!("{v:.6e}")
    } else {
        fmt(v, 6)
    }
}

/// The one empty field of `n`, and the others' values.
fn solve_for(o: &Opts, n: usize) -> Result<(usize, Vec<f64>), String> {
    let vals: Vec<Option<f64>> = (0..n).map(|i| o.opt_num(i)).collect::<Result<_, _>>()?;
    let empty: Vec<usize> = (0..n).filter(|i| vals[*i].is_none()).collect();
    if empty.len() != 1 {
        return Err("Leave exactly one field empty: the one to find.".into());
    }
    Ok((
        empty[0],
        vals.into_iter().map(|v| v.unwrap_or(0.0)).collect(),
    ))
}

// Physics

const G: f64 = 6.674_30e-11;

fn kinematics(_: &str, o: &mut Opts) -> Result<Output, String> {
    let u = o.num(0)?;
    let a = o.num(1)?;
    let t = o.num(2)?;
    Ok(table(&[
        ("Final velocity", format!("{} m/s", sig(u + a * t))),
        ("Distance", format!("{} m", sig(u * t + a * t * t / 2.0))),
        ("Average velocity", format!("{} m/s", sig(u + a * t / 2.0))),
    ]))
}

fn projectile(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.positive(0)?;
    let angle = o.num(1)?.to_radians();
    let h = o.opt_num(2)?.unwrap_or(0.0);
    let g = o.opt_num(3)?.unwrap_or(9.80665);
    let (vx, vy) = (v * angle.cos(), v * angle.sin());
    let t = (vy + (vy * vy + 2.0 * g * h).sqrt()) / g;
    Ok(table(&[
        ("Time of flight", format!("{} s", sig(t))),
        ("Range", format!("{} m", sig(vx * t))),
        (
            "Maximum height",
            format!("{} m", sig(h + vy * vy / (2.0 * g))),
        ),
        (
            "Impact speed",
            format!("{} m/s", sig((vx * vx + (vy - g * t).powi(2)).sqrt())),
        ),
    ]))
}

fn newton(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (i, v) = solve_for(o, 3)?;
    let (m, a, f) = (v[0], v[1], v[2]);
    Ok(table(&[match i {
        0 => ("Mass", format!("{} kg", sig(f / a))),
        1 => ("Acceleration", format!("{} m/s²", sig(f / m))),
        _ => ("Force", format!("{} N", sig(m * a))),
    }]))
}

fn energy(_: &str, o: &mut Opts) -> Result<Output, String> {
    let m = o.positive(0)?;
    let v = o.opt_num(1)?.unwrap_or(0.0);
    let h = o.opt_num(2)?.unwrap_or(0.0);
    let g = 9.80665;
    let (k, p) = (0.5 * m * v * v, m * g * h);
    Ok(table(&[
        ("Kinetic energy", format!("{} J", sig(k))),
        ("Potential energy", format!("{} J", sig(p))),
        ("Total", format!("{} J", sig(k + p))),
        (
            "Speed if it falls",
            format!("{} m/s", sig((v * v + 2.0 * g * h).sqrt())),
        ),
    ]))
}

fn momentum(_: &str, o: &mut Opts) -> Result<Output, String> {
    let m = o.positive(0)?;
    let v = o.num(1)?;
    let t = o.opt_num(2)?;
    let mut r = vec![("Momentum", format!("{} kg·m/s", sig(m * v)))];
    if let Some(t) = t.filter(|t| *t > 0.0) {
        r.push((
            "Force to stop it in that time",
            format!("{} N", sig(m * v / t)),
        ));
    }
    Ok(table(&r))
}

fn gravity(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (m1, m2, r) = (o.positive(0)?, o.positive(1)?, o.positive(2)?);
    let f = G * m1 * m2 / (r * r);
    Ok(table(&[
        ("Force", format!("{} N", sig(f))),
        (
            "Acceleration of the second",
            format!("{} m/s²", sig(f / m2)),
        ),
    ]))
}

fn electric_field(_: &str, o: &mut Opts) -> Result<Output, String> {
    let q = o.num(0)?;
    let r = o.positive(1)?;
    let k = 8.987_551_792_3e9;
    let e = k * q / (r * r);
    let mut rows = vec![
        ("Field", format!("{} N/C", sig(e))),
        ("Potential", format!("{} V", sig(k * q / r))),
    ];
    if let Some(q2) = o.opt_num(2)? {
        rows.push(("Force on the second charge", format!("{} N", sig(e * q2))));
    }
    Ok(table(&rows))
}

fn wave(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (i, v) = solve_for(o, 3)?;
    let (f, l, s) = (v[0], v[1], v[2]);
    let (f, l, s) = match i {
        0 => (s / l, l, s),
        1 => (f, s / f, s),
        _ => (f, l, f * l),
    };
    Ok(table(&[
        ("Frequency", format!("{} Hz", sig(f))),
        ("Wavelength", format!("{} m", sig(l))),
        ("Speed", format!("{} m/s", sig(s))),
        ("Period", format!("{} s", sig(1.0 / f))),
    ]))
}

fn work(_: &str, o: &mut Opts) -> Result<Output, String> {
    let f = o.num(0)?;
    let d = o.num(1)?;
    let angle = o.opt_num(2)?.unwrap_or(0.0).to_radians();
    let w = f * d * angle.cos();
    let mut r = vec![("Work", format!("{} J", sig(w)))];
    if let Some(t) = o.opt_num(3)?.filter(|t| *t > 0.0) {
        r.push(("Power", format!("{} W", sig(w / t))));
    }
    Ok(table(&r))
}

// Chemistry

/// Symbol, name and standard atomic weight, by atomic number.
const ELEMENTS: [(&str, &str, f64); 118] = [
    ("H", "Hydrogen", 1.008),
    ("He", "Helium", 4.0026),
    ("Li", "Lithium", 6.94),
    ("Be", "Beryllium", 9.0122),
    ("B", "Boron", 10.81),
    ("C", "Carbon", 12.011),
    ("N", "Nitrogen", 14.007),
    ("O", "Oxygen", 15.999),
    ("F", "Fluorine", 18.998),
    ("Ne", "Neon", 20.180),
    ("Na", "Sodium", 22.990),
    ("Mg", "Magnesium", 24.305),
    ("Al", "Aluminium", 26.982),
    ("Si", "Silicon", 28.085),
    ("P", "Phosphorus", 30.974),
    ("S", "Sulfur", 32.06),
    ("Cl", "Chlorine", 35.45),
    ("Ar", "Argon", 39.948),
    ("K", "Potassium", 39.098),
    ("Ca", "Calcium", 40.078),
    ("Sc", "Scandium", 44.956),
    ("Ti", "Titanium", 47.867),
    ("V", "Vanadium", 50.942),
    ("Cr", "Chromium", 51.996),
    ("Mn", "Manganese", 54.938),
    ("Fe", "Iron", 55.845),
    ("Co", "Cobalt", 58.933),
    ("Ni", "Nickel", 58.693),
    ("Cu", "Copper", 63.546),
    ("Zn", "Zinc", 65.38),
    ("Ga", "Gallium", 69.723),
    ("Ge", "Germanium", 72.630),
    ("As", "Arsenic", 74.922),
    ("Se", "Selenium", 78.971),
    ("Br", "Bromine", 79.904),
    ("Kr", "Krypton", 83.798),
    ("Rb", "Rubidium", 85.468),
    ("Sr", "Strontium", 87.62),
    ("Y", "Yttrium", 88.906),
    ("Zr", "Zirconium", 91.224),
    ("Nb", "Niobium", 92.906),
    ("Mo", "Molybdenum", 95.95),
    ("Tc", "Technetium", 98.0),
    ("Ru", "Ruthenium", 101.07),
    ("Rh", "Rhodium", 102.91),
    ("Pd", "Palladium", 106.42),
    ("Ag", "Silver", 107.87),
    ("Cd", "Cadmium", 112.41),
    ("In", "Indium", 114.82),
    ("Sn", "Tin", 118.71),
    ("Sb", "Antimony", 121.76),
    ("Te", "Tellurium", 127.60),
    ("I", "Iodine", 126.90),
    ("Xe", "Xenon", 131.29),
    ("Cs", "Caesium", 132.91),
    ("Ba", "Barium", 137.33),
    ("La", "Lanthanum", 138.91),
    ("Ce", "Cerium", 140.12),
    ("Pr", "Praseodymium", 140.91),
    ("Nd", "Neodymium", 144.24),
    ("Pm", "Promethium", 145.0),
    ("Sm", "Samarium", 150.36),
    ("Eu", "Europium", 151.96),
    ("Gd", "Gadolinium", 157.25),
    ("Tb", "Terbium", 158.93),
    ("Dy", "Dysprosium", 162.50),
    ("Ho", "Holmium", 164.93),
    ("Er", "Erbium", 167.26),
    ("Tm", "Thulium", 168.93),
    ("Yb", "Ytterbium", 173.05),
    ("Lu", "Lutetium", 174.97),
    ("Hf", "Hafnium", 178.49),
    ("Ta", "Tantalum", 180.95),
    ("W", "Tungsten", 183.84),
    ("Re", "Rhenium", 186.21),
    ("Os", "Osmium", 190.23),
    ("Ir", "Iridium", 192.22),
    ("Pt", "Platinum", 195.08),
    ("Au", "Gold", 196.97),
    ("Hg", "Mercury", 200.59),
    ("Tl", "Thallium", 204.38),
    ("Pb", "Lead", 207.2),
    ("Bi", "Bismuth", 208.98),
    ("Po", "Polonium", 209.0),
    ("At", "Astatine", 210.0),
    ("Rn", "Radon", 222.0),
    ("Fr", "Francium", 223.0),
    ("Ra", "Radium", 226.0),
    ("Ac", "Actinium", 227.0),
    ("Th", "Thorium", 232.04),
    ("Pa", "Protactinium", 231.04),
    ("U", "Uranium", 238.03),
    ("Np", "Neptunium", 237.0),
    ("Pu", "Plutonium", 244.0),
    ("Am", "Americium", 243.0),
    ("Cm", "Curium", 247.0),
    ("Bk", "Berkelium", 247.0),
    ("Cf", "Californium", 251.0),
    ("Es", "Einsteinium", 252.0),
    ("Fm", "Fermium", 257.0),
    ("Md", "Mendelevium", 258.0),
    ("No", "Nobelium", 259.0),
    ("Lr", "Lawrencium", 266.0),
    ("Rf", "Rutherfordium", 267.0),
    ("Db", "Dubnium", 268.0),
    ("Sg", "Seaborgium", 269.0),
    ("Bh", "Bohrium", 270.0),
    ("Hs", "Hassium", 277.0),
    ("Mt", "Meitnerium", 278.0),
    ("Ds", "Darmstadtium", 281.0),
    ("Rg", "Roentgenium", 282.0),
    ("Cn", "Copernicium", 285.0),
    ("Nh", "Nihonium", 286.0),
    ("Fl", "Flerovium", 289.0),
    ("Mc", "Moscovium", 290.0),
    ("Lv", "Livermorium", 293.0),
    ("Ts", "Tennessine", 294.0),
    ("Og", "Oganesson", 294.0),
];

fn element(symbol: &str) -> Option<(usize, f64)> {
    ELEMENTS
        .iter()
        .position(|e| e.0 == symbol)
        .map(|i| (i + 1, ELEMENTS[i].2))
}

/// The atoms of a formula such as `Ca(OH)2`, `CuSO4·5H2O` or `[Fe(CN)6]3-`,
/// in order of appearance.
fn parse_formula(s: &str) -> Result<Vec<(String, f64)>, String> {
    fn group(c: &[char], i: &mut usize, out: &mut Vec<(String, f64)>) -> Result<(), String> {
        while *i < c.len() {
            match c[*i] {
                '(' | '[' => {
                    *i += 1;
                    let mut inner = Vec::new();
                    group(c, i, &mut inner)?;
                    if *i >= c.len() || !matches!(c[*i], ')' | ']') {
                        return Err("A bracket is not closed.".into());
                    }
                    *i += 1;
                    let n = count(c, i);
                    for (e, k) in inner {
                        add(out, &e, k * n);
                    }
                }
                ')' | ']' => return Ok(()),
                'A'..='Z' => {
                    let mut sym = c[*i].to_string();
                    *i += 1;
                    if *i < c.len() && c[*i].is_ascii_lowercase() {
                        sym.push(c[*i]);
                        *i += 1;
                    }
                    if element(&sym).is_none() {
                        return Err(format!("“{sym}” is not an element."));
                    }
                    let n = count(c, i);
                    add(out, &sym, n);
                }
                '+' | '-' if *i + 1 >= c.len() => *i += 1,
                c2 => return Err(format!("Unexpected “{c2}” in the formula.")),
            }
        }
        Ok(())
    }
    fn count(c: &[char], i: &mut usize) -> f64 {
        let start = *i;
        while *i < c.len() && c[*i].is_ascii_digit() {
            *i += 1;
        }
        c[start..*i]
            .iter()
            .collect::<String>()
            .parse()
            .unwrap_or(1.0)
    }
    fn add(out: &mut Vec<(String, f64)>, e: &str, n: f64) {
        match out.iter_mut().find(|x| x.0 == e) {
            Some(x) => x.1 += n,
            None => out.push((e.to_owned(), n)),
        }
    }
    let s: String = s.trim().chars().filter(|c| !c.is_whitespace()).collect();
    let sub = "₀₁₂₃₄₅₆₇₈₉";
    let s: String = s
        .chars()
        .map(|c| {
            sub.chars()
                .position(|d| d == c)
                .map_or(c, |p| (b'0' + p as u8) as char)
        })
        .collect();
    let mut out = Vec::new();
    for part in s.split(['·', '*', '.']) {
        let lead: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
        let k: f64 = lead.parse().unwrap_or(1.0);
        let chars: Vec<char> = part[lead.len()..].chars().collect();
        let mut inner = Vec::new();
        let mut i = 0;
        group(&chars, &mut i, &mut inner)?;
        if i < chars.len() {
            return Err("A closing bracket has no opening one.".into());
        }
        for (e, n) in inner {
            add(&mut out, &e, n * k);
        }
    }
    if out.is_empty() {
        return Err("Write a formula such as H2O.".into());
    }
    Ok(out)
}

pub fn molar_mass(formula: &str) -> Result<f64, String> {
    Ok(parse_formula(formula)?
        .iter()
        .map(|(e, n)| element(e).unwrap().1 * n)
        .sum())
}

fn molecular_weight(_: &str, o: &mut Opts) -> Result<Output, String> {
    let atoms = parse_formula(o.field(0))?;
    let total: f64 = atoms.iter().map(|(e, n)| element(e).unwrap().1 * n).sum();
    let rows = atoms.iter().map(|(e, n)| {
        let m = element(e).unwrap().1 * n;
        vec![
            e.clone(),
            fmt(*n, 2),
            fmt(m, 4),
            format!("{:.3}%", m / total * 100.0),
        ]
    });
    Ok(table(&[("Molar mass", format!("{} g/mol", fmt(total, 4)))])
        .table(&["Element", "Atoms", "Mass", "Share"], rows))
}

fn empirical(text: &str, _: &mut Opts) -> Result<Output, String> {
    let mut moles = Vec::new();
    for part in text
        .split([',', ';', '\n'])
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        let (sym, pct): (String, String) = part.chars().partition(|c| c.is_alphabetic());
        let pct = super::parse_number(&pct)
            .ok_or_else(|| format!("“{part}” needs an element and a percentage, such as C 40."))?;
        let (_, mass) = element(&sym).ok_or_else(|| format!("“{sym}” is not an element."))?;
        moles.push((sym, pct / mass));
    }
    if moles.is_empty() {
        return Err("Write elements and percentages, such as C 40, H 6.7, O 53.3.".into());
    }
    let min = moles.iter().map(|m| m.1).fold(f64::INFINITY, f64::min);
    let ratios: Vec<f64> = moles.iter().map(|m| m.1 / min).collect();
    // The smallest multiplier that makes every ratio whole.
    let k = (1..=12)
        .find(|k| {
            ratios
                .iter()
                .all(|r| ((r * *k as f64) - (r * *k as f64).round()).abs() < 0.1)
        })
        .unwrap_or(1);
    let formula: String = moles
        .iter()
        .zip(&ratios)
        .map(|((s, _), r)| {
            let n = (r * k as f64).round() as u32;
            if n == 1 { s.clone() } else { format!("{s}{n}") }
        })
        .collect();
    Ok(table(&[
        ("Empirical formula", formula.clone()),
        (
            "Its molar mass",
            format!("{} g/mol", fmt(molar_mass(&formula)?, 3)),
        ),
    ]))
}

/// Integer coefficients that balance a reaction, by exact elimination.
fn balance_coefficients(sides: &[Vec<Vec<(String, f64)>>; 2]) -> Result<Vec<i128>, String> {
    let species: Vec<(&Vec<(String, f64)>, i128)> = sides[0]
        .iter()
        .map(|s| (s, 1))
        .chain(sides[1].iter().map(|s| (s, -1)))
        .collect();
    let mut elements: Vec<&str> = Vec::new();
    for (s, _) in &species {
        for (e, _) in s.iter() {
            if !elements.contains(&e.as_str()) {
                elements.push(e);
            }
        }
    }
    let n = species.len();
    // Rows: elements; columns: species. Fractions as (num, den).
    let mut m: Vec<Vec<(i128, i128)>> = elements
        .iter()
        .map(|e| {
            species
                .iter()
                .map(|(s, sign)| {
                    (
                        s.iter().find(|x| x.0 == *e).map_or(0, |x| x.1 as i128) * sign,
                        1,
                    )
                })
                .collect()
        })
        .collect();
    let norm = |(a, b): (i128, i128)| {
        let g = gcd(a.unsigned_abs(), b.unsigned_abs()).max(1) as i128;
        let s = if b < 0 { -1 } else { 1 };
        (s * a / g, s * b / g)
    };
    let mut pivots = Vec::new();
    let mut r = 0;
    for c in 0..n {
        let Some(p) = (r..m.len()).find(|&i| m[i][c].0 != 0) else {
            continue;
        };
        m.swap(r, p);
        let pv = m[r][c];
        for j in 0..n {
            m[r][j] = norm((m[r][j].0 * pv.1, m[r][j].1 * pv.0));
        }
        for i in 0..m.len() {
            if i != r && m[i][c].0 != 0 {
                let f = m[i][c];
                for j in 0..n {
                    let (a, b) = m[r][j];
                    let sub = norm((f.0 * a, f.1 * b));
                    let (x, y) = m[i][j];
                    m[i][j] = norm((
                        x.checked_mul(sub.1).ok_or("Too large to balance.")? - sub.0 * y,
                        y * sub.1,
                    ));
                }
            }
        }
        pivots.push(c);
        r += 1;
    }
    let free: Vec<usize> = (0..n).filter(|c| !pivots.contains(c)).collect();
    if free.len() != 1 {
        return Err(if free.is_empty() {
            "That reaction cannot be balanced.".into()
        } else {
            "That reaction balances in more than one independent way.".into()
        });
    }
    let f = free[0];
    // The free species is 1; each pivot species is minus its row's free entry.
    let mut coef: Vec<(i128, i128)> = vec![(0, 1); n];
    coef[f] = (1, 1);
    for (row, &c) in pivots.iter().enumerate() {
        coef[c] = norm((-m[row][f].0, m[row][f].1));
    }
    let lcm = coef.iter().fold(1i128, |l, (_, d)| {
        l / gcd(l as u128, *d as u128) as i128 * d
    });
    let mut ints: Vec<i128> = coef.iter().map(|(a, b)| a * (lcm / b)).collect();
    let g = ints.iter().fold(0u128, |g, x| gcd(g, x.unsigned_abs())) as i128;
    for x in ints.iter_mut() {
        *x /= g;
    }
    if ints.iter().all(|x| *x < 0) {
        ints.iter_mut().for_each(|x| *x = -*x);
    }
    if ints.iter().any(|x| *x <= 0) {
        return Err("That reaction cannot be balanced with these species on these sides.".into());
    }
    Ok(ints)
}

fn balance(_: &str, o: &mut Opts) -> Result<Output, String> {
    let eq = o
        .field(0)
        .replace("<=>", "=")
        .replace("->", "=")
        .replace('→', "=")
        .replace("⟶", "=");
    let (l, r) = eq
        .split_once('=')
        .ok_or("Write a reaction such as H2 + O2 = H2O.")?;
    let names: [Vec<String>; 2] = [l, r].map(|side| {
        side.split(" + ")
            .map(|s| {
                s.trim()
                    .trim_start_matches(|c: char| c.is_ascii_digit())
                    .to_owned()
            })
            .filter(|s| !s.is_empty())
            .collect()
    });
    let parsed: [Vec<Vec<(String, f64)>>; 2] = [
        names[0]
            .iter()
            .map(|s| parse_formula(s))
            .collect::<Result<_, _>>()?,
        names[1]
            .iter()
            .map(|s| parse_formula(s))
            .collect::<Result<_, _>>()?,
    ];
    let k = balance_coefficients(&parsed)?;
    let side = |names: &[String], k: &[i128]| {
        names
            .iter()
            .zip(k)
            .map(|(n, k)| {
                if *k == 1 {
                    n.clone()
                } else {
                    format!("{k}{n}")
                }
            })
            .collect::<Vec<_>>()
            .join(" + ")
    };
    let n0 = names[0].len();
    Ok(table(&[(
        "Balanced",
        format!(
            "{} → {}",
            side(&names[0], &k[..n0]),
            side(&names[1], &k[n0..])
        ),
    )]))
}

fn molarity(_: &str, o: &mut Opts) -> Result<Output, String> {
    let mass = o.positive(0)?;
    let mm = match super::parse_number(o.field(1)) {
        Some(v) => v,
        None => molar_mass(o.field(1))?,
    };
    let litres = o.positive(2)?;
    let mol = mass / mm;
    Ok(table(&[
        ("Molarity", format!("{} mol/L", sig(mol / litres))),
        ("Moles", sig(mol)),
        ("Molar mass", format!("{} g/mol", fmt(mm, 3))),
    ]))
}

fn dilution(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (i, v) = solve_for(o, 4)?;
    let (c1, v1, c2, v2) = (v[0], v[1], v[2], v[3]);
    let (label, value) = match i {
        0 => ("C₁", c2 * v2 / v1),
        1 => ("V₁", c2 * v2 / c1),
        2 => ("C₂", c1 * v1 / v2),
        _ => ("V₂", c1 * v1 / c2),
    };
    let mut rows = vec![(label, sig(value))];
    if i == 1 {
        rows.push(("Solvent to add", sig(v2 - value)));
    }
    Ok(table(&rows))
}

fn gas_law(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (i, v) = solve_for(o, 4)?;
    let r = 0.082057366;
    let (p, vol, n, t) = (v[0], v[1], v[2], v[3]);
    Ok(table(&[match i {
        0 => ("Pressure", format!("{} atm", sig(n * r * t / vol))),
        1 => ("Volume", format!("{} L", sig(n * r * t / p))),
        2 => ("Amount", format!("{} mol", sig(p * vol / (r * t)))),
        _ => (
            "Temperature",
            format!(
                "{} K ({} °C)",
                sig(p * vol / (n * r)),
                fmt(p * vol / (n * r) - 273.15, 2)
            ),
        ),
    }]))
}

fn ph(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.positive(0)?;
    let ph = match o.mode {
        "h" => -v.log10(),
        "oh" => 14.0 + v.log10(),
        "poh" => 14.0 - v,
        _ => v,
    };
    let kind = match ph {
        p if p < 7.0 => "acidic",
        p if p > 7.0 => "basic",
        _ => "neutral",
    };
    Ok(table(&[
        ("pH", format!("{} ({kind})", fmt(ph, 4))),
        ("pOH", fmt(14.0 - ph, 4)),
        ("[H⁺]", format!("{} mol/L", sig(10f64.powf(-ph)))),
        ("[OH⁻]", format!("{} mol/L", sig(10f64.powf(ph - 14.0)))),
    ]))
}

fn buffer(_: &str, o: &mut Opts) -> Result<Output, String> {
    let pka = o.num(0)?;
    let base = o.positive(1)?;
    let acid = o.positive(2)?;
    Ok(table(&[
        ("pH", fmt(pka + (base / acid).log10(), 4)),
        ("Base to acid ratio", fmt(base / acid, 4)),
    ]))
}

fn electron_config(z: usize) -> String {
    const ORDER: [(u32, char, u32); 19] = [
        (1, 's', 2),
        (2, 's', 2),
        (2, 'p', 6),
        (3, 's', 2),
        (3, 'p', 6),
        (4, 's', 2),
        (3, 'd', 10),
        (4, 'p', 6),
        (5, 's', 2),
        (4, 'd', 10),
        (5, 'p', 6),
        (6, 's', 2),
        (4, 'f', 14),
        (5, 'd', 10),
        (6, 'p', 6),
        (7, 's', 2),
        (5, 'f', 14),
        (6, 'd', 10),
        (7, 'p', 6),
    ];
    let mut left = z as u32;
    let mut shells: Vec<(u32, char, u32)> = Vec::new();
    for (n, l, cap) in ORDER {
        if left == 0 {
            break;
        }
        let e = left.min(cap);
        shells.push((n, l, e));
        left -= e;
    }
    // The common exceptions: an s electron (or two) moves to d or f.
    let moves: &[(u32, char, i32)] = match z {
        24 | 29 => &[(4, 's', -1), (3, 'd', 1)],
        41 | 42 | 44 | 45 | 47 => &[(5, 's', -1), (4, 'd', 1)],
        46 => &[(5, 's', -2), (4, 'd', 2)],
        78 | 79 => &[(6, 's', -1), (5, 'd', 1)],
        57 | 89 => &[
            (if z == 57 { 4 } else { 5 }, 'f', -1),
            (if z == 57 { 5 } else { 6 }, 'd', 1),
        ],
        58 | 64 | 91 | 92 | 93 | 96 => &[
            (if z < 90 { 4 } else { 5 }, 'f', -1),
            (if z < 90 { 5 } else { 6 }, 'd', 1),
        ],
        90 => &[(5, 'f', -2), (6, 'd', 2)],
        _ => &[],
    };
    for (n, l, d) in moves {
        match shells.iter_mut().find(|s| s.0 == *n && s.1 == *l) {
            Some(s) => s.2 = (s.2 as i32 + d) as u32,
            None => shells.push((*n, *l, *d as u32)),
        }
    }
    shells.retain(|s| s.2 > 0);
    shells.sort_by_key(|s| (s.0, "spdf".find(s.1)));
    let sup = |n: u32| {
        n.to_string()
            .chars()
            .map(|c| "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().nth(c as usize - '0' as usize).unwrap())
            .collect::<String>()
    };
    shells
        .iter()
        .map(|(n, l, e)| format!("{n}{l}{}", sup(*e)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn find_element(q: &str) -> Result<usize, String> {
    let q = q.trim();
    if let Ok(z) = q.parse::<usize>() {
        return if (1..=118).contains(&z) {
            Ok(z)
        } else {
            Err("Atomic numbers go from 1 to 118.".into())
        };
    }
    ELEMENTS
        .iter()
        .position(|e| {
            e.0.eq_ignore_ascii_case(q)
                || e.1.eq_ignore_ascii_case(q)
                || (q.eq_ignore_ascii_case("aluminum") && e.0 == "Al")
                || (q.eq_ignore_ascii_case("cesium") && e.0 == "Cs")
        })
        .map(|i| i + 1)
        .ok_or_else(|| format!("“{q}” is not an element's name, symbol or number."))
}

/// Period, group (none for lanthanides and actinides) and category.
fn place(z: usize) -> (usize, Option<usize>, &'static str) {
    let starts = [1, 3, 11, 19, 37, 55, 87, 119];
    let period = starts.iter().rposition(|s| z >= *s).unwrap() + 1;
    let first = starts[period - 1];
    let pos = z - first;
    let group = match period {
        1 => Some(if z == 1 { 1 } else { 18 }),
        2 | 3 => Some(if pos < 2 { pos + 1 } else { pos + 11 }),
        4 | 5 => Some(pos + 1),
        _ => match pos {
            0 | 1 => Some(pos + 1),
            2..=16 if pos > 2 => None,
            2 => Some(3),
            p => Some(p - 13),
        },
    };
    let category = match z {
        1 | 6 | 7 | 8 | 15 | 16 | 34 => "reactive nonmetal",
        2 | 10 | 18 | 36 | 54 | 86 => "noble gas",
        9 | 17 | 35 | 53 => "halogen",
        3 | 11 | 19 | 37 | 55 | 87 => "alkali metal",
        4 | 12 | 20 | 38 | 56 | 88 => "alkaline earth metal",
        5 | 14 | 32 | 33 | 51 | 52 => "metalloid",
        57..=71 => "lanthanide",
        89..=103 => "actinide",
        13 | 31 | 49 | 50 | 81 | 82 | 83 | 84 | 85 => "post-transition metal",
        104.. => "transactinide",
        _ => "transition metal",
    };
    (period, group, category)
}

fn periodic(_: &str, o: &mut Opts) -> Result<Output, String> {
    let z = find_element(o.field(0))?;
    let (sym, name, mass) = ELEMENTS[z - 1];
    let (period, group, category) = place(z);
    Ok(table(&[
        ("Element", format!("{name} ({sym})")),
        ("Atomic number", z.to_string()),
        ("Atomic weight", fmt(mass, 4)),
        ("Period", period.to_string()),
        (
            "Group",
            group.map_or("— (f-block)".into(), |g| g.to_string()),
        ),
        ("Category", category.into()),
        ("Electron configuration", electron_config(z)),
    ]))
}

fn electrons(_: &str, o: &mut Opts) -> Result<Output, String> {
    let z = find_element(o.field(0))?;
    let config = electron_config(z);
    let mut valence = 0;
    let max_n = config
        .split(' ')
        .filter_map(|s| s.chars().next()?.to_digit(10))
        .max()
        .unwrap_or(1);
    for s in config.split(' ') {
        if s.starts_with(&max_n.to_string()) {
            valence += s
                .chars()
                .skip(2)
                .map(|c| "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().position(|d| d == c).unwrap_or(0))
                .fold(0, |a, d| a * 10 + d);
        }
    }
    Ok(table(&[
        ("Element", ELEMENTS[z - 1].1.into()),
        ("Configuration", config),
        ("Outer shell electrons", valence.to_string()),
    ]))
}

// Astronomy

fn angular_size(_: &str, o: &mut Opts) -> Result<Output, String> {
    let size = o.positive(0)?;
    let d = o.positive(1)?;
    let rad = 2.0 * (size / (2.0 * d)).atan();
    let deg = rad.to_degrees();
    Ok(table(&[
        ("Angular size", format!("{}°", sig(deg))),
        (
            "",
            format!(
                "{} arcminutes, {} arcseconds",
                sig(deg * 60.0),
                sig(deg * 3600.0)
            ),
        ),
        (
            "Compared with the full moon",
            format!("{}×", sig(deg / 0.5181)),
        ),
    ]))
}

fn magnitude(_: &str, o: &mut Opts) -> Result<Output, String> {
    let m_abs = o.num(0)?;
    let d = o.positive(1)? * if o.mode == "ly" { 0.306601 } else { 1.0 };
    let m = m_abs + 5.0 * (d.log10() - 1.0);
    Ok(table(&[
        ("Apparent magnitude", fmt(m, 3)),
        ("Distance modulus", fmt(m - m_abs, 3)),
        (
            "Visible to the naked eye",
            if m <= 6.0 {
                "yes, under dark skies"
            } else {
                "no"
            }
            .into(),
        ),
    ]))
}

fn drake(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n: f64 = (0..7)
        .map(|i| o.num(i))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .product();
    Ok(table(&[("Civilizations we could detect", sig(n))]))
}

const BODIES: &[(&str, &str, f64, f64)] = &[
    ("earth", "Earth", 5.972e24, 6.371e6),
    ("moon", "the Moon", 7.342e22, 1.7374e6),
    ("mars", "Mars", 6.417e23, 3.3895e6),
    ("venus", "Venus", 4.867e24, 6.0518e6),
    ("mercury", "Mercury", 3.301e23, 2.4397e6),
    ("jupiter", "Jupiter", 1.898e27, 6.9911e7),
    ("saturn", "Saturn", 5.683e26, 5.8232e7),
    ("uranus", "Uranus", 8.681e25, 2.5362e7),
    ("neptune", "Neptune", 1.024e26, 2.4622e7),
    ("sun", "the Sun", 1.989e30, 6.957e8),
    ("custom", "your body", 0.0, 0.0),
];

fn escape(_: &str, o: &mut Opts) -> Result<Output, String> {
    let b = BODIES.iter().find(|b| b.0 == o.mode).unwrap_or(&BODIES[0]);
    let (m, r) = if b.0 == "custom" {
        (o.positive(0)?, o.positive(1)?)
    } else {
        (b.2, b.3)
    };
    let v = (2.0 * G * m / r).sqrt();
    Ok(table(&[
        ("Escape velocity", format!("{} km/s", fmt(v / 1000.0, 3))),
        (
            "Orbital velocity at the surface",
            format!("{} km/s", fmt(v / 2f64.sqrt() / 1000.0, 3)),
        ),
        (
            "Surface gravity",
            format!("{} m/s²", fmt(G * m / (r * r), 3)),
        ),
    ]))
}

fn orbit(_: &str, o: &mut Opts) -> Result<Output, String> {
    let a = o.positive(0)?
        * if o.choice(1) == "au" {
            1.495978707e11
        } else {
            1000.0
        };
    let b = BODIES.iter().find(|b| b.0 == o.mode).unwrap_or(&BODIES[0]);
    let m = if b.0 == "custom" { o.positive(1)? } else { b.2 };
    let t = 2.0 * std::f64::consts::PI * (a.powi(3) / (G * m)).sqrt();
    Ok(table(&[
        ("Orbital period", super::dates::duration_words(t)),
        ("In days", fmt(t / 86400.0, 4)),
        (
            "Orbital speed",
            format!("{} km/s", fmt((G * m / a).sqrt() / 1000.0, 3)),
        ),
    ]))
}

fn planet_age(_: &str, o: &mut Opts) -> Result<Output, String> {
    let born = o.date(0)?;
    let days = (o.date(1)? - born) as f64;
    let years = [
        ("Mercury", 87.969),
        ("Venus", 224.701),
        ("Earth", 365.256),
        ("Mars", 686.980),
        ("Jupiter", 4332.59),
        ("Saturn", 10759.22),
        ("Uranus", 30688.5),
        ("Neptune", 60182.0),
    ];
    let rows: Vec<(&str, String)> = years
        .iter()
        .map(|(p, y)| (*p, format!("{} years", fmt(days / y, 2))))
        .collect();
    Ok(table(&rows))
}

fn redshift(_: &str, o: &mut Opts) -> Result<Output, String> {
    let z = match o.mode {
        "wavelengths" => o.positive(1)? / o.positive(0)? - 1.0,
        _ => o.num(0)?,
    };
    let c = 299792.458;
    let rel = ((1.0 + z).powi(2) - 1.0) / ((1.0 + z).powi(2) + 1.0);
    Ok(table(&[
        ("Redshift z", fmt(z, 6)),
        (
            "Velocity (relativistic)",
            format!("{} km/s ({}c)", fmt(rel * c, 1), fmt(rel, 4)),
        ),
        (
            "Velocity (z × c, small z)",
            format!("{} km/s", fmt(z * c, 1)),
        ),
        (
            "Distance by Hubble's law",
            format!("{} million light years", fmt(rel * c / 70.0 * 3.26156, 1)),
        ),
    ]))
}

fn space_travel(_: &str, o: &mut Opts) -> Result<Output, String> {
    let ly = 9.4607304725808e12;
    let d = o.positive(0)?
        * if o.choice(1) == "au" {
            1.495978707e8
        } else {
            ly
        };
    let speed = o.positive(1)?;
    let v = if o.mode == "c" {
        speed * 299792.458
    } else {
        speed
    };
    if v >= 299792.458 {
        return Err("Nothing with mass reaches the speed of light.".into());
    }
    let t = d / v;
    let gamma = 1.0 / (1.0 - (v / 299792.458).powi(2)).sqrt();
    Ok(table(&[
        ("Time on Earth", super::dates::duration_words(t)),
        ("Time aboard", super::dates::duration_words(t / gamma)),
        (
            "Speed",
            format!("{} km/s ({}c)", fmt(v, 2), sig(v / 299792.458)),
        ),
    ]))
}

fn telescope(_: &str, o: &mut Opts) -> Result<Output, String> {
    let scope = o.positive(0)?;
    let eyepiece = o.positive(1)?;
    let afov = o.opt_num(2)?.unwrap_or(52.0);
    let aperture = o.opt_num(3)?;
    let mag = scope / eyepiece;
    let mut r = vec![
        ("Magnification", format!("{}×", fmt(mag, 1))),
        (
            "True field of view",
            format!(
                "{}° ({} arcmin)",
                fmt(afov / mag, 3),
                fmt(afov / mag * 60.0, 1)
            ),
        ),
    ];
    if let Some(a) = aperture {
        r.push(("Focal ratio", format!("f/{}", fmt(scope / a, 1))));
        r.push(("Exit pupil", format!("{} mm", fmt(a / mag, 2))));
        r.push((
            "Highest useful magnification",
            format!("{}×", fmt(a * 2.0, 0)),
        ));
        r.push(("Limiting magnitude", fmt(2.7 + 5.0 * a.log10(), 1)));
    }
    Ok(table(&r))
}

const SHOWERS: &str = "\
Shower            Peak          Rate/hour  Parent
Quadrantids       Jan 3–4       110        2003 EH1
Lyrids            Apr 22        18         C/1861 G1 Thatcher
Eta Aquariids     May 6         50         1P/Halley
Delta Aquariids   Jul 30        25         96P/Machholz
Perseids          Aug 12–13     100        109P/Swift–Tuttle
Draconids         Oct 8         10         21P/Giacobini–Zinner
Orionids          Oct 21        20         1P/Halley
Taurids           Nov 5–12      5          2P/Encke
Leonids           Nov 17        15         55P/Tempel–Tuttle
Geminids          Dec 13–14     150        3200 Phaethon
Ursids            Dec 22        10         8P/Tuttle";

fn showers(_: &str, o: &mut Opts) -> Result<Output, String> {
    reference(SHOWERS, o.field(0))
}

// Electronics and engineering

fn ohm(_: &str, o: &mut Opts) -> Result<Output, String> {
    let vals: Vec<Option<f64>> = (0..4).map(|i| o.opt_num(i)).collect::<Result<_, _>>()?;
    let known: Vec<usize> = (0..4).filter(|i| vals[*i].is_some()).collect();
    if known.len() != 2 {
        return Err("Fill in exactly two of voltage, current, resistance and power.".into());
    }
    let (mut v, mut i, mut r, mut p) = (vals[0], vals[1], vals[2], vals[3]);
    match (v, i, r, p) {
        (Some(vv), Some(ii), _, _) => {
            r = Some(vv / ii);
            p = Some(vv * ii);
        }
        (Some(vv), _, Some(rr), _) => {
            i = Some(vv / rr);
            p = Some(vv * vv / rr);
        }
        (Some(vv), _, _, Some(pp)) => {
            i = Some(pp / vv);
            r = Some(vv * vv / pp);
        }
        (_, Some(ii), Some(rr), _) => {
            v = Some(ii * rr);
            p = Some(ii * ii * rr);
        }
        (_, Some(ii), _, Some(pp)) => {
            v = Some(pp / ii);
            r = Some(pp / (ii * ii));
        }
        (_, _, Some(rr), Some(pp)) => {
            v = Some((pp * rr).sqrt());
            i = Some((pp / rr).sqrt());
        }
        _ => {}
    }
    Ok(table(&[
        ("Voltage", format!("{} V", sig(v.unwrap()))),
        ("Current", format!("{} A", sig(i.unwrap()))),
        ("Resistance", format!("{} Ω", sig(r.unwrap()))),
        ("Power", format!("{} W", sig(p.unwrap()))),
    ]))
}

pub fn si(v: f64, unit: &str) -> String {
    let prefixes = [
        (1e9, "G"),
        (1e6, "M"),
        (1e3, "k"),
        (1.0, ""),
        (1e-3, "m"),
        (1e-6, "µ"),
        (1e-9, "n"),
        (1e-12, "p"),
    ];
    let (k, p) = prefixes
        .iter()
        .find(|(k, _)| v.abs() >= *k * 0.9999)
        .copied()
        .unwrap_or((1e-12, "p"));
    format!("{} {p}{unit}", fmt(v / k, 3))
}

const COLORS: [&str; 10] = [
    "black", "brown", "red", "orange", "yellow", "green", "blue", "violet", "grey", "white",
];

fn resistor(_: &str, o: &mut Opts) -> Result<Output, String> {
    let tolerance = |c: &str| match c {
        "brown" => Some("±1%"),
        "red" => Some("±2%"),
        "green" => Some("±0.5%"),
        "blue" => Some("±0.25%"),
        "violet" => Some("±0.1%"),
        "grey" | "gray" => Some("±0.05%"),
        "gold" => Some("±5%"),
        "silver" => Some("±10%"),
        _ => None,
    };
    if o.mode == "value" {
        let v = parse_si(o.field(0)).ok_or("Write a resistance such as 4.7k or 220.")?;
        if v < 0.1 {
            return Err("Too small for a color code.".into());
        }
        let five = o.flag;
        let digits = if five { 3 } else { 2 };
        let exp = v.log10().floor() as i32 - (digits - 1);
        let sig_digits = (v / 10f64.powi(exp)).round() as u64;
        let ds: Vec<usize> = sig_digits
            .to_string()
            .chars()
            .map(|c| c as usize - '0' as usize)
            .collect();
        let mult = match exp {
            -2 => "silver".to_owned(),
            -1 => "gold".to_owned(),
            e if (0..=9).contains(&e) => COLORS[e as usize].to_owned(),
            _ => return Err("That is outside the color code.".into()),
        };
        let bands: Vec<String> = ds
            .iter()
            .map(|d| COLORS[*d].to_owned())
            .chain([mult, "gold (±5%)".into()])
            .collect();
        return Ok(table(&[("Bands", bands.join(" – "))]));
    }
    let bands: Vec<String> = o
        .field(0)
        .to_lowercase()
        .replace([',', '-'], " ")
        .split_whitespace()
        .map(|b| b.replace("gray", "grey"))
        .collect();
    if !(3..=6).contains(&bands.len()) {
        return Err("Write three to six band colors, such as yellow violet red gold.".into());
    }
    let digit_count = if bands.len() >= 5 { 3 } else { 2 };
    let digit = |b: &str| {
        COLORS
            .iter()
            .position(|c| *c == b)
            .ok_or_else(|| format!("“{b}” is not a digit color."))
    };
    let mut value = 0.0;
    for b in &bands[..digit_count] {
        value = value * 10.0 + digit(b)? as f64;
    }
    let mult = match bands[digit_count].as_str() {
        "gold" => 0.1,
        "silver" => 0.01,
        b => 10f64.powi(digit(b)? as i32),
    };
    let tol = bands
        .get(digit_count + 1)
        .map_or("±20%", |b| tolerance(b).unwrap_or("?"));
    let mut rows = vec![("Resistance", format!("{} {tol}", si(value * mult, "Ω")))];
    if bands.len() == 6 {
        let ppm = ["250", "100", "50", "15", "25", "20", "10", "5", "1", "?"];
        rows.push((
            "Temperature coefficient",
            format!("{} ppm/K", ppm[digit(&bands[5]).unwrap_or(9)]),
        ));
    }
    Ok(table(&rows))
}

/// A value with an SI prefix or as written in schematics: `4.7k`, `4k7`,
/// `2M2`, `100n`, `0.1u`.
pub fn parse_si(s: &str) -> Option<f64> {
    let t = s
        .trim()
        .trim_end_matches(['Ω', 'F', 'H', 'f'])
        .trim()
        .replace("ohm", "")
        .replace('µ', "u");
    let mult = |c: char| match c {
        'p' => Some(1e12),
        'n' => Some(1e9),
        'u' => Some(1e6),
        'm' => Some(1e3),
        'k' | 'K' => Some(1e-3),
        'M' => Some(1e-6),
        'G' => Some(1e-9),
        'R' | 'r' => Some(1.0),
        _ => None,
    };
    if let Some(i) = t.find(|c: char| mult(c).is_some()) {
        let k = mult(t[i..].chars().next()?)?;
        let num = format!("{}.{}", &t[..i], &t[i + 1..]);
        let num = num.trim_end_matches('.');
        let v: f64 = if num.starts_with('.') {
            format!("0{num}").parse().ok()?
        } else {
            num.parse().ok()?
        };
        return Some(v / k);
    }
    t.parse().ok()
}

fn smd(_: &str, o: &mut Opts) -> Result<Output, String> {
    let code = o.field(0).to_uppercase();
    const EIA96: [u32; 96] = [
        100, 102, 105, 107, 110, 113, 115, 118, 121, 124, 127, 130, 133, 137, 140, 143, 147, 150,
        154, 158, 162, 165, 169, 174, 178, 182, 187, 191, 196, 200, 205, 210, 215, 221, 226, 232,
        237, 243, 249, 255, 261, 267, 274, 280, 287, 294, 301, 309, 316, 324, 332, 340, 348, 357,
        365, 374, 383, 392, 402, 412, 422, 432, 442, 453, 464, 475, 487, 499, 511, 523, 536, 549,
        562, 576, 590, 604, 619, 634, 649, 665, 681, 698, 715, 732, 750, 768, 787, 806, 825, 845,
        866, 887, 909, 931, 953, 976,
    ];
    let v = if code.contains('R') {
        code.replace('R', ".")
            .parse::<f64>()
            .map_err(|_| "Not an SMD code.")?
    } else if code.len() == 3
        && code[..2].chars().all(|c| c.is_ascii_digit())
        && code.ends_with(|c: char| c.is_ascii_alphabetic())
    {
        let i: usize = code[..2].parse().map_err(|_| "Not an SMD code.")?;
        let base = *EIA96
            .get(i.wrapping_sub(1))
            .ok_or("EIA-96 codes go from 01 to 96.")? as f64;
        let m = match code.chars().last().unwrap() {
            'Z' => 0.001,
            'Y' | 'R' => 0.01,
            'X' | 'S' => 0.1,
            'A' => 1.0,
            'B' | 'H' => 10.0,
            'C' => 100.0,
            'D' => 1000.0,
            'E' => 10000.0,
            'F' => 100000.0,
            _ => return Err("Unknown EIA-96 multiplier letter.".into()),
        };
        base * m
    } else if (3..=4).contains(&code.len()) && code.chars().all(|c| c.is_ascii_digit()) {
        let (digits, e) = code.split_at(code.len() - 1);
        digits.parse::<f64>().unwrap() * 10f64.powi(e.parse::<i32>().unwrap())
    } else {
        return Err("Write a code such as 472, 4R7, 1002 or 01C.".into());
    };
    Ok(table(&[("Resistance", si(v, "Ω"))]))
}

fn capacitor(_: &str, o: &mut Opts) -> Result<Output, String> {
    let code = o.field(0).trim().to_string();
    let pf = if code.len() == 3 && code.chars().all(|c| c.is_ascii_digit()) {
        let (d, e) = code.split_at(2);
        let e: i32 = e.parse().unwrap();
        d.parse::<f64>().unwrap()
            * if e == 9 {
                0.1
            } else if e == 8 {
                0.01
            } else {
                10f64.powi(e)
            }
    } else {
        parse_si(&code)
            .map(|v| {
                if code.chars().any(|c| c.is_alphabetic()) {
                    v * 1e12
                } else {
                    v
                }
            })
            .ok_or("Write a code such as 104, 4n7 or 0.1u.")?
    };
    let farads = pf * 1e-12;
    Ok(table(&[
        ("Capacitance", si(farads, "F")),
        ("Picofarads", fmt(pf, 3)),
        ("Nanofarads", fmt(pf / 1000.0, 4)),
        ("Microfarads", fmt(pf / 1e6, 6)),
        ("Three-digit code", {
            let e = (pf.log10().floor() as i32 - 1).max(0);
            format!("{:02}{}", (pf / 10f64.powi(e)).round() as u64, e)
        }),
    ]))
}

const E12: [f64; 12] = [1.0, 1.2, 1.5, 1.8, 2.2, 2.7, 3.3, 3.9, 4.7, 5.6, 6.8, 8.2];

fn nearest_e12(v: f64, up: bool) -> f64 {
    let decade = 10f64.powf(v.log10().floor());
    let mut options: Vec<f64> = E12
        .iter()
        .map(|e| e * decade)
        .chain(std::iter::once(10.0 * decade))
        .collect();
    if up {
        options.retain(|x| *x >= v * 0.999);
        options[0]
    } else {
        *options
            .iter()
            .min_by(|a, b| (*a - v).abs().total_cmp(&(*b - v).abs()))
            .unwrap()
    }
}

fn led(_: &str, o: &mut Opts) -> Result<Output, String> {
    let supply = o.positive(0)?;
    let vf = o.positive(1)?;
    let ma = o.positive(2)?;
    let n = o.opt_num(3)?.unwrap_or(1.0).max(1.0);
    let drop = supply - vf * n;
    if drop <= 0.0 {
        return Err("The supply must be above the LEDs' total forward voltage.".into());
    }
    let r = drop / (ma / 1000.0);
    let standard = nearest_e12(r, true);
    Ok(table(&[
        ("Resistor", si(r, "Ω")),
        ("Nearest E12 at or above", si(standard, "Ω")),
        (
            "Actual current",
            format!("{} mA", fmt(drop / standard * 1000.0, 2)),
        ),
        (
            "Resistor power",
            format!(
                "{} W (use a {} W part)",
                fmt(drop * drop / standard, 3),
                if drop * drop / standard < 0.125 {
                    "⅛"
                } else if drop * drop / standard < 0.25 {
                    "¼"
                } else if drop * drop / standard < 0.5 {
                    "½"
                } else {
                    "1"
                }
            ),
        ),
    ]))
}

fn divider(_: &str, o: &mut Opts) -> Result<Output, String> {
    let vin = o.positive(0)?;
    let r1 = parse_si(o.field(1)).ok_or("Write R1 such as 10k.")?;
    match parse_si(o.field(2)) {
        Some(r2) => Ok(table(&[
            (
                "Output voltage",
                format!("{} V", fmt(vin * r2 / (r1 + r2), 4)),
            ),
            ("Current", si(vin / (r1 + r2), "A")),
            ("Ratio", fmt(r2 / (r1 + r2), 4)),
        ])),
        None => {
            let vout = o.num(3)?;
            if vout >= vin {
                return Err("The output must be below the input.".into());
            }
            let r2 = r1 * vout / (vin - vout);
            Ok(table(&[
                ("R2", si(r2, "Ω")),
                ("Nearest E12", si(nearest_e12(r2, false), "Ω")),
            ]))
        }
    }
}

fn timer555(_: &str, o: &mut Opts) -> Result<Output, String> {
    let c = parse_si(o.field(2)).ok_or("Write C such as 10u or 100n.")?;
    let r1 = parse_si(o.field(0)).ok_or("Write R1 such as 1k.")?;
    if o.mode == "mono" {
        let t = 1.1 * r1 * c;
        return Ok(table(&[("Pulse length", format!("{} s", sig(t)))]));
    }
    let r2 = parse_si(o.field(1)).ok_or("Write R2 such as 10k.")?;
    let high = 0.693 * (r1 + r2) * c;
    let low = 0.693 * r2 * c;
    Ok(table(&[
        ("Frequency", si(1.0 / (high + low), "Hz")),
        ("Period", format!("{} s", sig(high + low))),
        ("High time", format!("{} s", sig(high))),
        ("Low time", format!("{} s", sig(low))),
        (
            "Duty cycle",
            format!("{}%", fmt(high / (high + low) * 100.0, 2)),
        ),
    ]))
}

fn awg(_: &str, o: &mut Opts) -> Result<Output, String> {
    let gauge = if o.mode == "mm2" {
        let a = o.positive(0)?;
        let d = (a * 4.0 / std::f64::consts::PI).sqrt();
        36.0 - 39.0 * (d / 0.127).log(92.0)
    } else {
        let g = o.field(0);
        match g {
            "0000" | "4/0" => -3.0,
            "000" | "3/0" => -2.0,
            "00" | "2/0" => -1.0,
            _ => o.num(0)?,
        }
    };
    let d = 0.127 * 92f64.powf((36.0 - gauge) / 39.0);
    let area = std::f64::consts::PI * d * d / 4.0;
    Ok(table(&[
        ("AWG", fmt(gauge, 2)),
        (
            "Diameter",
            format!("{} mm ({} in)", fmt(d, 4), fmt(d / 25.4, 5)),
        ),
        ("Area", format!("{} mm²", fmt(area, 4))),
        (
            "Copper resistance",
            format!("{} Ω/km", fmt(17.24 / area, 3)),
        ),
    ]))
}

fn gear(_: &str, o: &mut Opts) -> Result<Output, String> {
    let a = o.positive(0)?;
    let b = o.positive(1)?;
    let rpm = o.opt_num(2)?;
    let torque = o.opt_num(3)?;
    let ratio = b / a;
    let mut r = vec![("Ratio", format!("{}:1", fmt(ratio, 4)))];
    if let Some(rpm) = rpm {
        r.push(("Output speed", format!("{} rpm", fmt(rpm / ratio, 2))));
    }
    if let Some(t) = torque {
        r.push((
            "Output torque",
            format!("{} (before losses)", fmt(t * ratio, 2)),
        ));
    }
    Ok(table(&r))
}

fn belt(_: &str, o: &mut Opts) -> Result<Output, String> {
    let d1 = o.positive(0)?;
    let d2 = o.positive(1)?;
    let c = o.positive(2)?;
    let len = 2.0 * c + std::f64::consts::PI * (d1 + d2) / 2.0 + (d1 - d2).powi(2) / (4.0 * c);
    let wrap = 180.0 - 2.0 * ((d1 - d2).abs() / (2.0 * c)).asin().to_degrees();
    Ok(table(&[
        ("Belt length", fmt(len, 2)),
        ("Wrap on the small pulley", format!("{}°", fmt(wrap, 1))),
        ("Speed ratio", fmt(d1.max(d2) / d1.min(d2), 3)),
    ]))
}

fn rotation(_: &str, o: &mut Opts) -> Result<Output, String> {
    let d = o.positive(0)?;
    let v = o.positive(1)?;
    Ok(if o.mode == "rpm" {
        table(&[(
            "Surface speed",
            format!("{} m/min", fmt(std::f64::consts::PI * d / 1000.0 * v, 2)),
        )])
    } else {
        table(&[(
            "Spindle speed",
            format!("{} rpm", fmt(v * 1000.0 / (std::f64::consts::PI * d), 0)),
        )])
    })
}

fn thread(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.positive(0)?;
    let (pitch, tpi) = if o.mode == "tpi" {
        (25.4 / v, v)
    } else {
        (v, 25.4 / v)
    };
    let d = o.opt_num(1)?;
    let mut r = vec![
        ("Pitch", format!("{} mm", fmt(pitch, 4))),
        ("Threads per inch", fmt(tpi, 3)),
    ];
    if let Some(d) = d {
        r.push((
            "Tap drill (75% thread)",
            format!("{} mm", fmt(d - pitch, 2)),
        ));
    }
    Ok(table(&r))
}

fn torque(_: &str, o: &mut Opts) -> Result<Output, String> {
    let f = o.num(0)?;
    let arm = o.num(1)?;
    let nm = f * arm;
    Ok(table(&[
        ("Torque", format!("{} N·m", fmt(nm, 4))),
        (
            "",
            format!(
                "{} lb·ft · {} lb·in · {} kgf·m",
                fmt(nm * 0.737562, 4),
                fmt(nm * 8.85075, 3),
                fmt(nm / 9.80665, 4)
            ),
        ),
    ]))
}

// 3D printing

const MATERIALS: super::Menu = &[
    ("1.24", "PLA"),
    ("1.27", "PETG"),
    ("1.04", "ABS"),
    ("1.07", "ASA"),
    ("1.21", "TPU"),
    ("1.14", "Nylon"),
    ("1.20", "PC"),
];

fn filament(_: &str, o: &mut Opts) -> Result<Output, String> {
    let density: f64 = o.mode.parse().unwrap_or(1.24);
    let d = o.positive(1)?;
    let area_cm2 = std::f64::consts::PI * (d / 20.0).powi(2);
    let v = o.positive(0)?;
    let (metres, grams) = if o.choice(1) == "weight" {
        (v / density / area_cm2 / 100.0, v)
    } else {
        (v, v * 100.0 * area_cm2 * density)
    };
    let mut r = vec![
        ("Length", format!("{} m", fmt(metres, 2))),
        ("Weight", format!("{} g", fmt(grams, 1))),
    ];
    if let Some(price) = o.opt_num(2)? {
        r.push(("Cost", fmt(grams / 1000.0 * price, 2)));
    }
    Ok(table(&r))
}

fn infill(_: &str, o: &mut Opts) -> Result<Output, String> {
    let volume = o.positive(0)?;
    let infill = o.num(1)? / 100.0;
    let shell = o.opt_num(2)?.unwrap_or(25.0) / 100.0;
    let density: f64 = o.mode.parse().unwrap_or(1.24);
    let solid = shell + (1.0 - shell) * infill;
    Ok(table(&[
        (
            "Estimated weight",
            format!("{} g", fmt(volume * solid * density, 1)),
        ),
        ("Solid share", format!("{}%", fmt(solid * 100.0, 1))),
    ]))
}

fn layer_height(_: &str, o: &mut Opts) -> Result<Output, String> {
    let nozzle = o.positive(0)?;
    let step = o.opt_num(1)?.unwrap_or(0.04);
    let snap = |h: f64| (h / step).round() * step;
    Ok(table(&[
        ("Finest", format!("{} mm", fmt(snap(nozzle * 0.25), 3))),
        ("Recommended", format!("{} mm", fmt(snap(nozzle * 0.5), 3))),
        ("Fastest", format!("{} mm", fmt(snap(nozzle * 0.75), 3))),
        (
            "Line width",
            format!("{} – {} mm", fmt(nozzle, 2), fmt(nozzle * 1.2, 2)),
        ),
        (
            "",
            format!(
                "Heights rounded to {} mm steps of the Z motor.",
                fmt(step, 3)
            ),
        ),
    ]))
}

fn model_scale(_: &str, o: &mut Opts) -> Result<Output, String> {
    let dims: Vec<f64> = (0..3).map(|i| o.positive(i)).collect::<Result<_, _>>()?;
    let target = o.positive(3)?;
    let axis = match o.mode {
        "y" => 1,
        "z" => 2,
        _ => 0,
    };
    let k = target / dims[axis];
    Ok(table(&[
        ("Scale", format!("{}%", fmt(k * 100.0, 3))),
        (
            "New size",
            format!(
                "{} × {} × {}",
                fmt(dims[0] * k, 2),
                fmt(dims[1] * k, 2),
                fmt(dims[2] * k, 2)
            ),
        ),
        ("Volume", format!("{}× the original", fmt(k.powi(3), 3))),
    ]))
}

fn flow_rate(_: &str, o: &mut Opts) -> Result<Output, String> {
    let h = o.positive(0)?;
    let w = o.positive(1)?;
    let speed = o.positive(2)?;
    let flow = h * w * speed;
    let limit = o.opt_num(3)?.unwrap_or(12.0);
    Ok(table(&[
        ("Volumetric flow", format!("{} mm³/s", fmt(flow, 2))),
        (
            "Speed at the hotend's limit",
            format!("{} mm/s", fmt(limit / (h * w), 0)),
        ),
        (
            "",
            if flow > limit {
                "⚠ Above the hotend's limit: expect under-extrusion."
            } else {
                "✓ Within the hotend's limit."
            }
            .into(),
        ),
    ]))
}

fn print_cost(_: &str, o: &mut Opts) -> Result<Output, String> {
    let grams = o.positive(0)?;
    let per_kg = o.num(1)?;
    let hours = o.num(2)?;
    let watts = o.opt_num(3)?.unwrap_or(120.0);
    let kwh = o.opt_num(4)?.unwrap_or(0.25);
    let wear = o.opt_num(5)?.unwrap_or(0.0);
    let material = grams / 1000.0 * per_kg;
    let power = watts * hours / 1000.0 * kwh;
    Ok(table(&[
        ("Material", fmt(material, 2)),
        ("Electricity", fmt(power, 2)),
        ("Machine wear", fmt(wear * hours, 2)),
        ("Total", fmt(material + power + wear * hours, 2)),
    ]))
}

fn resin(_: &str, o: &mut Opts) -> Result<Output, String> {
    let ml = o.positive(0)?;
    let price = o.num(1)?;
    let supports = o.opt_num(2)?.unwrap_or(15.0) / 100.0;
    let total = ml * (1.0 + supports);
    Ok(table(&[
        ("Resin", format!("{} ml", fmt(total, 1))),
        ("Cost", fmt(total / 1000.0 * price, 2)),
        ("Prints a litre", fmt((1000.0 / total).floor(), 0)),
    ]))
}

fn shrinkage(_: &str, o: &mut Opts) -> Result<Output, String> {
    let shrink: f64 = o.mode.parse().unwrap_or(0.5);
    let size = o.opt_num(0)?;
    let k = 1.0 / (1.0 - shrink / 100.0);
    let mut r = vec![("Scale the model to", format!("{}%", fmt(k * 100.0, 3)))];
    if let Some(s) = size {
        r.push(("Model size for that part", fmt(s * k, 3)));
    }
    Ok(table(&r))
}

fn temp_tower(_: &str, o: &mut Opts) -> Result<Output, String> {
    let start = o.num(0)?;
    let end = o.num(1)?;
    let step = o.positive(2)?.abs();
    let height = o.positive(3)?;
    let n = ((start - end).abs() / step).floor() as usize + 1;
    if n > 40 {
        return Err("That makes more than 40 blocks.".into());
    }
    let dir = if end < start { -1.0 } else { 1.0 };
    let mut out = Vec::new();
    for i in 0..n {
        let t = start + dir * step * i as f64;
        out.push(vec![
            (i + 1).to_string(),
            format!("{} mm", fmt(height * i as f64, 2)),
            format!("M104 S{}", fmt(t, 0)),
        ]);
    }
    Ok(grid(&["Block", "From Z", "G-code"], out))
}

fn bed_check(_: &str, o: &mut Opts) -> Result<Output, String> {
    let m: Vec<f64> = (0..3).map(|i| o.positive(i)).collect::<Result<_, _>>()?;
    let b: Vec<f64> = (3..6).map(|i| o.positive(i)).collect::<Result<_, _>>()?;
    let fits = m[0] <= b[0] && m[1] <= b[1] && m[2] <= b[2];
    let rotated = m[1] <= b[0] && m[0] <= b[1] && m[2] <= b[2];
    // Laid diagonally on the bed.
    let diagonal = (b[0] * b[0] + b[1] * b[1]).sqrt();
    let verdict = if fits {
        "✓ It fits as it is."
    } else if rotated {
        "✓ It fits turned 90°."
    } else if m[2] <= b[2]
        && m[0].max(m[1]) <= diagonal * 0.95
        && m[0].min(m[1]) < b[0].min(b[1]) * 0.5
    {
        "⚠ It may fit laid diagonally."
    } else {
        "✗ It does not fit: split or scale it."
    };
    let scale = (b[0] / m[0]).min(b[1] / m[1]).min(b[2] / m[2]).min(1.0);
    Ok(table(&[
        ("", verdict.into()),
        (
            "Largest scale that fits",
            format!("{}%", fmt(scale * 100.0, 1)),
        ),
    ]))
}

const RETRACTION: &str = "\
Setup                      Distance     Speed
Direct drive, PLA          0.5–1 mm     35–45 mm/s
Direct drive, PETG         1–2 mm       30–40 mm/s
Direct drive, TPU          0–1 mm       20–30 mm/s
Bowden, PLA                4–6 mm       40–50 mm/s
Bowden, PETG               5–7 mm       35–45 mm/s
Bowden, ABS                4–6 mm       40–50 mm/s
Tune with a retraction tower; raise distance for stringing, lower it for gaps.";

const TOLERANCES: &str = "\
Fit                         Gap per side (FDM)
Press fit                   0.0–0.1 mm
Snug, by hand               0.15 mm
Sliding fit                 0.2–0.3 mm
Loose, moves freely         0.4–0.5 mm
Print-in-place hinges       0.3–0.5 mm
Holes print small: add 0.1–0.2 mm to small hole diameters.";

const INSERTS: &str = "\
Thread   Insert hole (heat-set)   Clearance hole   Tap hole (printed)
M2       3.2 mm                   2.4 mm           1.6 mm
M2.5     3.6 mm                   2.9 mm           2.0 mm
M3       4.0–4.2 mm               3.4 mm           2.5 mm
M4       5.6 mm                   4.5 mm           3.3 mm
M5       6.4 mm                   5.5 mm           4.2 mm
M6       8.0 mm                   6.6 mm           5.0 mm
M8       9.7 mm                   9.0 mm           6.8 mm
Hole depth: the insert's length plus 1 mm.";

fn retraction(_: &str, o: &mut Opts) -> Result<Output, String> {
    reference(RETRACTION, o.field(0))
}

fn tolerances(_: &str, o: &mut Opts) -> Result<Output, String> {
    reference(TOLERANCES, o.field(0))
}

fn inserts(_: &str, o: &mut Opts) -> Result<Output, String> {
    reference(INSERTS, o.field(0))
}

// Photography

const SENSORS: super::Menu = &[
    ("1", "Full frame"),
    ("1.5", "APS-C (Nikon, Sony, Fuji)"),
    ("1.6", "APS-C (Canon)"),
    ("2", "Micro Four Thirds"),
    ("2.7", "1 inch"),
    ("5.6", "Phone (1/2.3\")"),
    ("0.79", "Medium format 44×33"),
];

fn dof(_: &str, o: &mut Opts) -> Result<Output, String> {
    let f = o.positive(0)?;
    let n = o.positive(1)?;
    let s = o.positive(2)? * 1000.0;
    let crop: f64 = o.mode.parse().unwrap_or(1.0);
    let coc = 0.03 / crop;
    let h = f * f / (n * coc) + f;
    let near = s * (h - f) / (h + s - 2.0 * f);
    let far = if s >= h {
        f64::INFINITY
    } else {
        s * (h - f) / (h - s)
    };
    let m = |v: f64| {
        if v.is_finite() {
            format!("{} m", fmt(v / 1000.0, 2))
        } else {
            "infinity".into()
        }
    };
    Ok(table(&[
        ("Near limit", m(near)),
        ("Far limit", m(far)),
        (
            "Depth of field",
            if far.is_finite() {
                m(far - near)
            } else {
                "infinite".into()
            },
        ),
        ("Hyperfocal distance", m(h)),
        ("Circle of confusion", format!("{} mm", fmt(coc, 4))),
    ]))
}

fn hyperfocal(_: &str, o: &mut Opts) -> Result<Output, String> {
    let f = o.positive(0)?;
    let n = o.positive(1)?;
    let crop: f64 = o.mode.parse().unwrap_or(1.0);
    let h = f * f / (n * 0.03 / crop) + f;
    Ok(table(&[
        ("Hyperfocal distance", format!("{} m", fmt(h / 1000.0, 2))),
        (
            "Sharp from",
            format!("{} m to infinity", fmt(h / 2000.0, 2)),
        ),
    ]))
}

fn shutter(s: &str) -> Option<f64> {
    let s = s.trim().trim_end_matches('s').trim_end_matches('"');
    match s.split_once('/') {
        Some((a, b)) => Some(a.trim().parse::<f64>().ok()? / b.trim().parse::<f64>().ok()?),
        None => s.parse().ok(),
    }
}

fn show_shutter(t: f64) -> String {
    if t >= 1.0 {
        format!("{}s", fmt(t, 1))
    } else {
        format!("1/{}", fmt((1.0 / t).round(), 0))
    }
}

fn exposure(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = o.positive(0)?;
    let t = shutter(o.field(1))
        .filter(|t| *t > 0.0)
        .ok_or("Write the shutter speed as 1/125 or 2.")?;
    let iso = o.positive(2)?;
    let ev = (n * n / t).log2() - (iso / 100.0).log2();
    let mut out = Vec::new();
    for stop in [1.4, 2.0, 2.8, 4.0, 5.6, 8.0, 11.0, 16.0, 22.0] {
        let t2 = t * (stop * stop) / (n * n);
        if (1.0 / 8000.0..=30.0).contains(&t2) {
            out.push(vec![format!("f/{stop}"), show_shutter(t2), fmt(iso, 0)]);
        }
    }
    Ok(table(&[
        ("Exposure value", fmt(ev, 2)),
        ("At ISO 100", fmt((n * n / t).log2(), 2)),
    ])
    .note("The same exposure:")
    .table(&["Aperture", "Shutter", "ISO"], out))
}

fn flash(_: &str, o: &mut Opts) -> Result<Output, String> {
    let gn = o.positive(0)?;
    let iso = o.positive(1)?;
    let n = o.positive(2)?;
    let effective = gn * (iso / 100.0).sqrt();
    Ok(table(&[
        ("Flash distance", format!("{} m", fmt(effective / n, 2))),
        ("Guide number at this ISO", fmt(effective, 1)),
    ]))
}

fn focal(_: &str, o: &mut Opts) -> Result<Output, String> {
    let f = o.positive(0)?;
    let crop: f64 = o.mode.parse().unwrap_or(1.5);
    let n = o.opt_num(1)?;
    let mut r = vec![("Full-frame equivalent", format!("{} mm", fmt(f * crop, 1)))];
    if let Some(n) = n {
        r.push((
            "Equivalent aperture (depth of field)",
            format!("f/{}", fmt(n * crop, 1)),
        ));
    }
    let diag = 43.27 / crop;
    r.push((
        "Diagonal field of view",
        format!("{}°", fmt(2.0 * (diag / (2.0 * f)).atan().to_degrees(), 1)),
    ));
    Ok(table(&r))
}

fn image_size(_: &str, o: &mut Opts) -> Result<Output, String> {
    let mp = o.positive(0)?;
    let bits = o.opt_num(1)?.unwrap_or(14.0);
    let raw = mp * 1e6 * bits / 8.0;
    Ok(table(&[
        ("RAW, uncompressed", format!("{} MB", fmt(raw / 1e6, 1))),
        (
            "RAW, lossless compressed",
            format!("about {} MB", fmt(raw * 0.6 / 1e6, 1)),
        ),
        (
            "JPEG, high quality",
            format!("about {} MB", fmt(mp * 0.4, 1)),
        ),
        ("TIFF, 16-bit", format!("{} MB", fmt(mp * 6.0, 1))),
        ("Shots on 64 GB (RAW)", fmt((64e9 / (raw * 0.6)).floor(), 0)),
    ]))
}

fn megapixels(_: &str, o: &mut Opts) -> Result<Output, String> {
    let w = o.positive(0)?;
    let h = o.positive(1)?;
    let g = gcd(w as u128, h as u128).max(1) as f64;
    Ok(table(&[
        ("Megapixels", fmt(w * h / 1e6, 2)),
        (
            "Aspect ratio",
            format!("{}:{}", fmt(w / g, 0), fmt(h / g, 0)),
        ),
        (
            "Print at 300 dpi",
            format!(
                "{} × {} cm",
                fmt(w / 300.0 * 2.54, 1),
                fmt(h / 300.0 * 2.54, 1)
            ),
        ),
        (
            "Print at 150 dpi",
            format!(
                "{} × {} cm",
                fmt(w / 150.0 * 2.54, 1),
                fmt(h / 150.0 * 2.54, 1)
            ),
        ),
    ]))
}

fn nd_filter(_: &str, o: &mut Opts) -> Result<Output, String> {
    let t = shutter(o.field(0))
        .filter(|t| *t > 0.0)
        .ok_or("Write the shutter speed as 1/125 or 2.")?;
    let stops: f64 = o.mode.parse().unwrap_or(10.0);
    let t2 = t * 2f64.powf(stops);
    Ok(table(&[
        (
            "New exposure",
            if t2 >= 60.0 {
                super::dates::duration_words(t2)
            } else {
                show_shutter(t2)
            },
        ),
        (
            "Filter",
            format!("ND{} ({} stops)", fmt(2f64.powf(stops), 0), stops),
        ),
    ]))
}

fn dpi(_: &str, o: &mut Opts) -> Result<Output, String> {
    let px = o.positive(0)?;
    let v = o.positive(1)?;
    Ok(if o.mode == "size" {
        table(&[("Resolution", format!("{} dpi", fmt(px / (v / 2.54), 0)))])
    } else {
        table(&[(
            "Print size",
            format!("{} cm ({} in)", fmt(px / v * 2.54, 2), fmt(px / v, 2)),
        )])
    })
}

fn star_trails(_: &str, o: &mut Opts) -> Result<Output, String> {
    let f = o.positive(0)?;
    let crop: f64 = o.mode.parse().unwrap_or(1.0);
    let n = o.opt_num(1)?.unwrap_or(2.8);
    let pitch = o.opt_num(2)?.unwrap_or(4.0);
    let rule500 = 500.0 / (f * crop);
    let npf = (35.0 * n + 30.0 * pitch) / f;
    Ok(table(&[
        ("500 rule", format!("{} s", fmt(rule500, 1))),
        ("NPF rule (sharper)", format!("{} s", fmt(npf, 1))),
        (
            "",
            "Longer exposures than these show stars as trails.".into(),
        ),
    ]))
}

fn timelapse(_: &str, o: &mut Opts) -> Result<Output, String> {
    let event = o.positive(0)? * 60.0;
    let interval = o.positive(1)?;
    let fps = o.positive(2)?;
    let mb = o.opt_num(3)?.unwrap_or(25.0);
    let frames = (event / interval).floor();
    Ok(table(&[
        ("Frames", fmt(frames, 0)),
        ("Clip length", format!("{} s", fmt(frames / fps, 1))),
        ("Storage", format!("{} GB", fmt(frames * mb / 1000.0, 2))),
        ("Speed-up", format!("{}×", fmt(interval * fps, 0))),
    ]))
}

fn aspect_crop(_: &str, o: &mut Opts) -> Result<Output, String> {
    let w = o.positive(0)?;
    let h = o.positive(1)?;
    let (a, b) = o
        .field(2)
        .split_once(':')
        .ok_or("Write the ratio as 16:9.")?;
    let r = a
        .trim()
        .parse::<f64>()
        .map_err(|_| "Write the ratio as 16:9.")?
        / b.trim()
            .parse::<f64>()
            .map_err(|_| "Write the ratio as 16:9.")?;
    let (cw, ch) = if w / h > r { (h * r, h) } else { (w, w / r) };
    Ok(table(&[
        (
            "Crop to",
            format!("{} × {}", fmt(cw.floor(), 0), fmt(ch.floor(), 0)),
        ),
        (
            "Offset",
            format!(
                "{} from the sides, {} from the top",
                fmt(((w - cw) / 2.0).floor(), 0),
                fmt(((h - ch) / 2.0).floor(), 0)
            ),
        ),
        ("Kept", format!("{}%", fmt(cw * ch / (w * h) * 100.0, 1))),
    ]))
}

// Music

const NOTES: [&str; 12] = [
    "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
];
const FLATS: [&str; 12] = [
    "C", "D♭", "D", "E♭", "E", "F", "G♭", "G", "A♭", "A", "B♭", "B",
];

fn note_index(s: &str) -> Option<(i32, Option<i32>)> {
    let s = s.trim().replace('♯', "#").replace('♭', "b");
    let mut c = s.chars();
    let base: i32 = match c.next()?.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest: String = c.collect();
    let (acc, octave) = match rest.chars().next() {
        Some('#') => (1, &rest[1..]),
        Some('b') => (-1, &rest[1..]),
        _ => (0, &rest[..]),
    };
    Some(((base + acc).rem_euclid(12), octave.parse().ok()))
}

fn note_frequency(_: &str, o: &mut Opts) -> Result<Output, String> {
    let a4 = o.opt_num(1)?.unwrap_or(440.0);
    let q = o.field(0);
    let midi = if let Some(hz) = super::parse_number(q) {
        69.0 + 12.0 * (hz / a4).log2()
    } else {
        let (n, oct) = note_index(q).ok_or("Write a note such as A4 or C#3, or a frequency.")?;
        ((oct.unwrap_or(4) + 1) * 12 + n) as f64
    };
    let nearest = midi.round();
    let cents = (midi - nearest) * 100.0;
    let name = format!(
        "{}{}",
        NOTES[(nearest as i64).rem_euclid(12) as usize],
        (nearest as i64).div_euclid(12) - 1
    );
    let hz = a4 * 2f64.powf((nearest - 69.0) / 12.0);
    Ok(table(&[
        (
            "Note",
            if cents.abs() > 0.5 {
                format!("{name} {:+.0} cents", cents)
            } else {
                name
            },
        ),
        (
            "Frequency",
            format!("{} Hz", fmt(a4 * 2f64.powf((midi - 69.0) / 12.0), 3)),
        ),
        ("The note exactly", format!("{} Hz", fmt(hz, 3))),
        ("MIDI number", fmt(nearest, 0)),
        ("Wavelength in air", format!("{} m", fmt(343.0 / hz, 3))),
    ]))
}

const SCALES: &[(&str, &str, &[i32])] = &[
    ("major", "Major", &[0, 2, 4, 5, 7, 9, 11]),
    ("minor", "Natural minor", &[0, 2, 3, 5, 7, 8, 10]),
    ("harmonic", "Harmonic minor", &[0, 2, 3, 5, 7, 8, 11]),
    ("melodic", "Melodic minor", &[0, 2, 3, 5, 7, 9, 11]),
    ("dorian", "Dorian", &[0, 2, 3, 5, 7, 9, 10]),
    ("phrygian", "Phrygian", &[0, 1, 3, 5, 7, 8, 10]),
    ("lydian", "Lydian", &[0, 2, 4, 6, 7, 9, 11]),
    ("mixolydian", "Mixolydian", &[0, 2, 4, 5, 7, 9, 10]),
    ("locrian", "Locrian", &[0, 1, 3, 5, 6, 8, 10]),
    ("penta-major", "Major pentatonic", &[0, 2, 4, 7, 9]),
    ("penta-minor", "Minor pentatonic", &[0, 3, 5, 7, 10]),
    ("blues", "Blues", &[0, 3, 5, 6, 7, 10]),
    (
        "chromatic",
        "Chromatic",
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
    ),
];

fn spell(n: i32, flats: bool) -> &'static str {
    let i = n.rem_euclid(12) as usize;
    if flats { FLATS[i] } else { NOTES[i] }
}

fn uses_flats(root: i32, minor: bool) -> bool {
    let major_root = if minor {
        (root + 3).rem_euclid(12)
    } else {
        root
    };
    matches!(major_root, 5 | 10 | 3 | 8 | 1 | 6)
}

fn scale(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (root, _) = note_index(o.field(0)).ok_or("Write a root note such as C or F#.")?;
    let (_, name, steps) = SCALES.iter().find(|s| s.0 == o.mode).unwrap_or(&SCALES[0]);
    let flats = o.field(0).contains(['b', '♭']) || uses_flats(root, steps.get(2) == Some(&3));
    let notes: Vec<&str> = steps.iter().map(|s| spell(root + s, flats)).collect();
    let title = format!("{} {name}", spell(root, flats));
    let mut out = vec![(title.as_str(), notes.join(" "))];
    if steps.len() == 7 {
        let quality = |i: usize| {
            let third = (steps[(i + 2) % 7] - steps[i]).rem_euclid(12);
            let fifth = (steps[(i + 4) % 7] - steps[i]).rem_euclid(12);
            match (third, fifth) {
                (4, 7) => "",
                (3, 7) => "m",
                (3, 6) => "dim",
                (4, 8) => "aug",
                _ => "?",
            }
        };
        let chords: Vec<String> = (0..7)
            .map(|i| format!("{}{}", notes[i], quality(i)))
            .collect();
        out.push(("Chords", chords.join("  ")));
    }
    Ok(table(&out))
}

fn progression(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (root, _) = note_index(o.field(0)).ok_or("Write a key such as G or Am.")?;
    let minor = o.field(0).trim_end().ends_with('m');
    let steps: &[i32] = if minor {
        &[0, 2, 3, 5, 7, 8, 10]
    } else {
        &[0, 2, 4, 5, 7, 9, 11]
    };
    let flats = o.field(0).contains(['b', '♭']) || uses_flats(root, minor);
    let romans = ["i", "ii", "iii", "iv", "v", "vi", "vii"];
    let mut out = Vec::new();
    for r in o
        .field(1)
        .split(['-', ' ', ',', '–'])
        .filter(|r| !r.is_empty())
    {
        let lower = r.to_lowercase();
        let base = lower
            .trim_end_matches(['7', '°', 'o'])
            .trim_end_matches("dim");
        let deg = romans
            .iter()
            .position(|x| *x == base)
            .ok_or_else(|| format!("“{r}” is not a Roman numeral from I to VII."))?;
        let upper = r.chars().next().is_some_and(char::is_uppercase);
        let dim = r.contains(['°', 'o']) || r.contains("dim");
        let seventh = r.ends_with('7');
        let suffix = if dim {
            "dim"
        } else if upper {
            ""
        } else {
            "m"
        };
        out.push(format!(
            "{}{suffix}{}",
            spell(root + steps[deg], flats),
            if seventh { "7" } else { "" }
        ));
    }
    Ok(table(&[("Chords", out.join(" – "))]))
}

fn transpose(text: &str, o: &mut Opts) -> Result<Output, String> {
    let k = o.num(0)? as i32;
    let flats = o.flag;
    let re = regex_lite::Regex::new(
        r"\b([A-G])([#b♯♭]?)((?:maj|min|m|dim|aug|sus|add)?\d*(?:/[A-G][#b♯♭]?)?)",
    )
    .unwrap();
    // Only lines that look like chord lines are changed.
    let is_chords = |l: &str| {
        let words: Vec<&str> = l.split_whitespace().collect();
        !words.is_empty()
            && words
                .iter()
                .all(|w| re.find(w).is_some_and(|m| m.as_str() == *w) || *w == "|" || *w == "-")
    };
    let shift = |root: &str| -> String {
        let (n, _) = note_index(root).unwrap();
        spell(n + k, flats).replace('♯', "#").replace('♭', "b")
    };
    Ok(text
        .lines()
        .map(|l| {
            if !is_chords(l) && !o.check(1) {
                return l.to_owned();
            }
            re.replace_all(l, |c: &regex_lite::Captures| {
                let tail = &c[3];
                let tail = match tail.split_once('/') {
                    Some((q, bass)) => format!("{q}/{}", shift(bass)),
                    None => tail.to_owned(),
                };
                format!("{}{tail}", shift(&format!("{}{}", &c[1], &c[2])))
            })
            .into_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
        .into())
}

fn bpm(_: &str, o: &mut Opts) -> Result<Output, String> {
    let b = o.positive(0)?;
    let beat = 60000.0 / b;
    let rows: Vec<(&str, String)> = [
        ("Whole note", 4.0),
        ("Half note", 2.0),
        ("Quarter note", 1.0),
        ("Eighth note", 0.5),
        ("Sixteenth note", 0.25),
        ("Thirty-second note", 0.125),
    ]
    .iter()
    .map(|(n, k)| {
        (
            *n,
            format!(
                "{} ms · dotted {} · triplet {}",
                fmt(beat * k, 2),
                fmt(beat * k * 1.5, 2),
                fmt(beat * k * 2.0 / 3.0, 2)
            ),
        )
    })
    .chain([
        ("Beat frequency", format!("{} Hz", fmt(b / 60.0, 4))),
        ("Bar of 4/4", format!("{} s", fmt(beat * 4.0 / 1000.0, 3))),
    ])
    .collect();
    Ok(table(&rows))
}

fn tap_bpm(text: &str, _: &mut Opts) -> Result<Output, String> {
    let times: Vec<f64> = super::calc::numbers(text)?;
    if times.len() < 2 {
        return Err("Write at least two beat times in seconds, or one interval per line.".into());
    }
    let increasing = times.windows(2).all(|w| w[1] > w[0]);
    let intervals: Vec<f64> = if increasing {
        times.windows(2).map(|w| w[1] - w[0]).collect()
    } else {
        times.clone()
    };
    let avg = intervals.iter().sum::<f64>() / intervals.len() as f64;
    Ok(table(&[("Tempo", format!("{} BPM", fmt(60.0 / avg, 1)))]))
}

// The menus

pub const SCIENCE: &[Tool] = &[
    tool(
        "kinematics",
        "Velocity and acceleration",
        "Final velocity and distance under constant acceleration.",
        kinematics,
    )
    .form()
    .fields(&[
        ("Initial velocity (m/s)", "0"),
        ("Acceleration (m/s²)", "9.81"),
        ("Time (s)", "3"),
    ]),
    tool(
        "projectile",
        "Projectile motion",
        "Range, height and flight time of a launched object.",
        projectile,
    )
    .form()
    .fields(&[
        ("Speed (m/s)", "20"),
        ("Angle (°)", "45"),
        ("Launch height (m)", "0"),
        ("Gravity (m/s²)", "9.80665"),
    ]),
    tool(
        "newton",
        "Force (F = ma)",
        "Force, mass or acceleration: leave the one to find empty.",
        newton,
    )
    .form()
    .fields(&[
        ("Mass (kg)", "10"),
        ("Acceleration (m/s²)", "2"),
        ("Force (N)", ""),
    ]),
    tool(
        "mechanical-energy",
        "Kinetic and potential energy",
        "The energies of a moving, raised mass.",
        energy,
    )
    .form()
    .fields(&[
        ("Mass (kg)", "2"),
        ("Speed (m/s)", "5"),
        ("Height (m)", "10"),
    ]),
    tool(
        "momentum",
        "Momentum",
        "Momentum, and the force to stop it.",
        momentum,
    )
    .form()
    .fields(&[
        ("Mass (kg)", "1200"),
        ("Velocity (m/s)", "15"),
        ("Stopping time (s)", "2"),
    ]),
    tool(
        "gravity",
        "Gravitational force",
        "Newton's law of gravitation between two masses.",
        gravity,
    )
    .form()
    .fields(&[
        ("Mass 1 (kg)", "5.972e24"),
        ("Mass 2 (kg)", "70"),
        ("Distance (m)", "6.371e6"),
    ]),
    tool(
        "electric-field",
        "Electric field",
        "Field and potential of a point charge.",
        electric_field,
    )
    .form()
    .fields(&[
        ("Charge (C)", "1e-6"),
        ("Distance (m)", "0.1"),
        ("Second charge (C, optional)", ""),
    ]),
    tool(
        "wave",
        "Wave properties",
        "Frequency, wavelength or speed: leave the one to find empty.",
        wave,
    )
    .form()
    .fields(&[
        ("Frequency (Hz)", "440"),
        ("Wavelength (m)", ""),
        ("Speed (m/s)", "343"),
    ]),
    tool(
        "work",
        "Work and power",
        "The work a force does, and the power.",
        work,
    )
    .form()
    .fields(&[
        ("Force (N)", "50"),
        ("Distance (m)", "10"),
        ("Angle (°)", "0"),
        ("Time (s, optional)", "5"),
    ]),
    tool(
        "molar-mass",
        "Molecular weight",
        "Molar mass and composition of a formula, such as Ca(OH)2 or CuSO4·5H2O.",
        molecular_weight,
    )
    .form()
    .fields(&[("Formula", "C6H12O6")]),
    tool(
        "balance",
        "Chemical equation balancer",
        "Balances a reaction such as Fe + O2 = Fe2O3.",
        balance,
    )
    .form()
    .fields(&[("Reaction", "C3H8 + O2 = CO2 + H2O")]),
    tool(
        "empirical",
        "Empirical formula",
        "The simplest formula from percent composition.",
        empirical,
    )
    .text("Elements and percentages, such as C 40, H 6.7, O 53.3"),
    tool(
        "molarity",
        "Molarity",
        "Concentration from mass, formula and volume.",
        molarity,
    )
    .form()
    .fields(&[
        ("Mass (g)", "5.85"),
        ("Molar mass, or formula", "NaCl"),
        ("Volume (L)", "0.5"),
    ]),
    tool(
        "dilution",
        "Dilution (C₁V₁ = C₂V₂)",
        "Leave the one to find empty.",
        dilution,
    )
    .form()
    .fields(&[("C₁", "2"), ("V₁", ""), ("C₂", "0.5"), ("V₂", "100")]),
    tool(
        "gas-law",
        "Ideal gas law",
        "PV = nRT: leave the one to find empty (atm, L, mol, K).",
        gas_law,
    )
    .form()
    .fields(&[
        ("Pressure (atm)", "1"),
        ("Volume (L)", ""),
        ("Amount (mol)", "1"),
        ("Temperature (K)", "273.15"),
    ]),
    tool("ph", "pH calculator", "pH, pOH and ion concentrations.", ph)
        .form()
        .fields(&[("Value", "1e-4")])
        .selects(&[(
            "The value is",
            &[
                ("h", "[H⁺] (mol/L)"),
                ("ph", "pH"),
                ("oh", "[OH⁻] (mol/L)"),
                ("poh", "pOH"),
            ],
        )]),
    tool(
        "buffer",
        "Buffer solution",
        "The pH of a buffer (Henderson–Hasselbalch).",
        buffer,
    )
    .form()
    .fields(&[
        ("pKa", "4.76"),
        ("Base concentration [A⁻]", "0.1"),
        ("Acid concentration [HA]", "0.1"),
    ]),
    tool(
        "periodic",
        "Periodic table",
        "An element's number, weight, place and electron configuration.",
        periodic,
    )
    .form()
    .fields(&[("Element (name, symbol or number)", "Fe")]),
    tool(
        "electron-config",
        "Electron configuration",
        "The electron configuration of an element.",
        electrons,
    )
    .form()
    .fields(&[("Element", "Cu")]),
];

pub const SPACE: &[Tool] = &[
    tool(
        "angular-size",
        "Angular size",
        "How big an object looks from a distance.",
        angular_size,
    )
    .form()
    .fields(&[("Size", "3474.8"), ("Distance (same unit)", "384400")]),
    tool(
        "magnitude",
        "Apparent magnitude",
        "How bright a star looks from its absolute magnitude and distance.",
        magnitude,
    )
    .form()
    .fields(&[("Absolute magnitude", "4.83"), ("Distance", "10")])
    .selects(&[("Distance in", &[("pc", "Parsecs"), ("ly", "Light years")])]),
    tool(
        "drake",
        "Drake equation",
        "An estimate of the civilizations in our galaxy.",
        drake,
    )
    .form()
    .fields(&[
        ("Stars formed a year", "1.5"),
        ("Share with planets", "1"),
        ("Habitable planets each", "0.4"),
        ("Share with life", "0.1"),
        ("Share with intelligence", "0.01"),
        ("Share that signal", "0.1"),
        ("Years they signal", "10000"),
    ]),
    tool(
        "escape-velocity",
        "Escape velocity",
        "Escape and orbital velocity of a planet or star.",
        escape,
    )
    .form()
    .fields(&[("Mass (kg, your body)", ""), ("Radius (m, your body)", "")])
    .selects(&[(
        "Body",
        &[
            ("earth", "Earth"),
            ("moon", "Moon"),
            ("mars", "Mars"),
            ("venus", "Venus"),
            ("mercury", "Mercury"),
            ("jupiter", "Jupiter"),
            ("saturn", "Saturn"),
            ("uranus", "Uranus"),
            ("neptune", "Neptune"),
            ("sun", "Sun"),
            ("custom", "Your own"),
        ],
    )]),
    tool(
        "orbit",
        "Orbital period",
        "The period of an orbit (Kepler's third law).",
        orbit,
    )
    .form()
    .fields(&[
        ("Orbit radius", "42164"),
        ("Central mass (kg, your body)", ""),
    ])
    .selects(&[
        (
            "Around",
            &[
                ("earth", "Earth"),
                ("sun", "Sun"),
                ("moon", "Moon"),
                ("mars", "Mars"),
                ("jupiter", "Jupiter"),
                ("custom", "Your own"),
            ],
        ),
        (
            "Radius in",
            &[("km", "Kilometres"), ("au", "Astronomical units")],
        ),
    ]),
    tool(
        "planet-age",
        "Age on other planets",
        "Your age in the years of each planet.",
        planet_age,
    )
    .form()
    .fields(&[("Birth date", "1990-05-15"), ("On (empty: today)", "")]),
    tool(
        "redshift",
        "Redshift",
        "Recession velocity and distance from redshift.",
        redshift,
    )
    .form()
    .fields(&[
        ("Redshift z, or emitted wavelength", "0.1"),
        ("Observed wavelength", ""),
    ])
    .selects(&[("From", &[("z", "z"), ("wavelengths", "Wavelengths")])]),
    tool(
        "space-travel",
        "Space travel time",
        "Travel time at a speed, on Earth and aboard.",
        space_travel,
    )
    .form()
    .fields(&[("Distance", "4.24"), ("Speed", "0.1")])
    .selects(&[
        (
            "Speed in",
            &[("c", "Fraction of light speed"), ("kms", "km/s")],
        ),
        (
            "Distance in",
            &[("ly", "Light years"), ("au", "Astronomical units")],
        ),
    ]),
    tool(
        "telescope",
        "Telescope calculator",
        "Magnification, field of view and limits of a telescope.",
        telescope,
    )
    .form()
    .fields(&[
        ("Telescope focal length (mm)", "1200"),
        ("Eyepiece focal length (mm)", "25"),
        ("Eyepiece apparent field (°)", "52"),
        ("Aperture (mm)", "200"),
    ]),
    tool(
        "meteor-showers",
        "Meteor showers",
        "The year's main meteor showers.",
        showers,
    )
    .form()
    .fields(&[("Search", "")]),
];

pub const ELECTRONICS: &[Tool] = &[
    tool(
        "ohm",
        "Ohm's law",
        "Voltage, current, resistance and power from any two.",
        ohm,
    )
    .form()
    .fields(&[
        ("Voltage (V)", "12"),
        ("Current (A)", ""),
        ("Resistance (Ω)", "220"),
        ("Power (W)", ""),
    ]),
    tool(
        "resistor",
        "Resistor color code",
        "A resistor's value from its bands, or the bands of a value.",
        resistor,
    )
    .form()
    .fields(&[("Bands, or a value", "yellow violet red gold")])
    .selects(&[("From", &[("bands", "Band colors"), ("value", "A value")])])
    .checks(&["Five bands"]),
    tool(
        "smd",
        "SMD resistor code",
        "The value of 3-digit, 4-digit and EIA-96 codes.",
        smd,
    )
    .form()
    .fields(&[("Code", "472")]),
    tool(
        "capacitor",
        "Capacitor code",
        "The value of a capacitor code such as 104.",
        capacitor,
    )
    .form()
    .fields(&[("Code or value", "104")]),
    tool("led", "LED resistor", "The series resistor for LEDs.", led)
        .form()
        .fields(&[
            ("Supply (V)", "5"),
            ("LED forward voltage (V)", "2"),
            ("Current (mA)", "20"),
            ("LEDs in series", "1"),
        ]),
    tool(
        "divider",
        "Voltage divider",
        "The output of a divider, or the R2 for an output.",
        divider,
    )
    .form()
    .fields(&[
        ("Input (V)", "12"),
        ("R1", "10k"),
        ("R2 (empty: find it)", "4.7k"),
        ("Wanted output (V)", ""),
    ]),
    tool(
        "555",
        "555 timer",
        "Frequency and duty cycle of a 555 astable, or a monostable's pulse.",
        timer555,
    )
    .form()
    .fields(&[("R1", "1k"), ("R2", "10k"), ("C", "10u")])
    .selects(&[(
        "Mode",
        &[("astable", "Astable"), ("mono", "Monostable (R1, C)")],
    )]),
    tool(
        "awg",
        "Wire gauge",
        "AWG to diameter, area and resistance, and back.",
        awg,
    )
    .form()
    .fields(&[("Value", "12")])
    .selects(&[("From", &[("awg", "AWG"), ("mm2", "Area (mm²)")])]),
    tool(
        "gear",
        "Gear ratio",
        "Ratio, output speed and torque of a gear pair.",
        gear,
    )
    .form()
    .fields(&[
        ("Driving gear teeth", "12"),
        ("Driven gear teeth", "36"),
        ("Input speed (rpm)", "1500"),
        ("Input torque", "10"),
    ]),
    tool(
        "belt",
        "Belt length",
        "The length of a belt over two pulleys.",
        belt,
    )
    .form()
    .fields(&[
        ("Pulley 1 diameter", "100"),
        ("Pulley 2 diameter", "50"),
        ("Centre distance", "300"),
    ]),
    tool(
        "rotation",
        "Rotation speed",
        "Surface speed from rpm, or rpm for a cutting speed.",
        rotation,
    )
    .form()
    .fields(&[("Diameter (mm)", "50"), ("Value", "1000")])
    .selects(&[(
        "The value is",
        &[("rpm", "rpm"), ("speed", "Cutting speed (m/min)")],
    )]),
    tool(
        "thread",
        "Thread pitch",
        "Threads per inch and millimetre pitch, and the tap drill.",
        thread,
    )
    .form()
    .fields(&[("Value", "1.25"), ("Major diameter (mm, optional)", "8")])
    .selects(&[(
        "The value is",
        &[("mm", "Pitch (mm)"), ("tpi", "Threads per inch")],
    )]),
    tool(
        "torque",
        "Torque",
        "Torque from force and lever arm, in several units.",
        torque,
    )
    .form()
    .fields(&[("Force (N)", "100"), ("Lever arm (m)", "0.3")]),
];

pub const PRINTING: &[Tool] = &[
    tool(
        "bed-check",
        "Bed size checker",
        "Whether a model fits the print bed.",
        bed_check,
    )
    .form()
    .fields(&[
        ("Model X", "180"),
        ("Model Y", "250"),
        ("Model Z", "100"),
        ("Bed X", "220"),
        ("Bed Y", "220"),
        ("Bed Z", "250"),
    ]),
    tool(
        "filament",
        "Filament calculator",
        "Filament length and weight, and cost.",
        filament,
    )
    .form()
    .fields(&[
        ("Amount", "50"),
        ("Diameter (mm)", "1.75"),
        ("Price per kg", "20"),
    ])
    .selects(&[
        ("Material", MATERIALS),
        (
            "The amount is",
            &[("length", "Metres"), ("weight", "Grams")],
        ),
    ]),
    tool(
        "infill",
        "Infill weight",
        "A part's weight from its volume and infill.",
        infill,
    )
    .form()
    .fields(&[
        ("Model volume (cm³)", "100"),
        ("Infill (%)", "20"),
        ("Walls and skins (%)", "25"),
    ])
    .selects(&[("Material", MATERIALS)]),
    tool(
        "layer-height",
        "Layer height guide",
        "Layer heights for a nozzle, snapped to motor steps.",
        layer_height,
    )
    .form()
    .fields(&[("Nozzle (mm)", "0.4"), ("Z step (mm)", "0.04")]),
    tool(
        "model-scale",
        "Model scaler",
        "The scale that gives a part a wanted size.",
        model_scale,
    )
    .form()
    .fields(&[
        ("X", "120"),
        ("Y", "80"),
        ("Z", "40"),
        ("Wanted size", "150"),
    ])
    .selects(&[("Along", &[("x", "X"), ("y", "Y"), ("z", "Z")])]),
    tool(
        "flow-rate",
        "Nozzle flow rate",
        "The volumetric flow a print needs.",
        flow_rate,
    )
    .form()
    .fields(&[
        ("Layer height (mm)", "0.2"),
        ("Line width (mm)", "0.45"),
        ("Speed (mm/s)", "150"),
        ("Hotend limit (mm³/s)", "15"),
    ]),
    tool(
        "print-cost",
        "Print cost",
        "Material, power and wear for a print.",
        print_cost,
    )
    .form()
    .fields(&[
        ("Filament (g)", "120"),
        ("Price per kg", "22"),
        ("Print time (h)", "6"),
        ("Power (W)", "120"),
        ("Price per kWh", "0.25"),
        ("Wear per hour", "0.1"),
    ]),
    tool(
        "resin",
        "Resin calculator",
        "Resin and cost of a resin print.",
        resin,
    )
    .form()
    .fields(&[
        ("Model volume (ml)", "45"),
        ("Price per litre", "35"),
        ("Supports and waste (%)", "15"),
    ]),
    tool(
        "shrinkage",
        "Shrinkage compensator",
        "The scale that makes up for a material's shrinkage.",
        shrinkage,
    )
    .form()
    .fields(&[("Part size (optional)", "100")])
    .selects(&[(
        "Material",
        &[
            ("0.3", "PLA (0.3%)"),
            ("0.5", "PETG (0.5%)"),
            ("0.8", "ABS (0.8%)"),
            ("0.6", "ASA (0.6%)"),
            ("1.5", "Nylon (1.5%)"),
            ("0.7", "PC (0.7%)"),
        ],
    )]),
    tool(
        "temp-tower",
        "Temperature tower",
        "Heights and temperatures of a temperature tower.",
        temp_tower,
    )
    .form()
    .fields(&[
        ("Start temperature", "230"),
        ("End temperature", "190"),
        ("Step", "5"),
        ("Block height (mm)", "10"),
    ]),
    tool(
        "retraction",
        "Retraction guide",
        "Starting retraction settings.",
        retraction,
    )
    .form()
    .fields(&[("Search", "")]),
    tool(
        "tolerances",
        "Tolerance and fit guide",
        "Gaps for press, sliding and loose fits.",
        tolerances,
    )
    .form()
    .fields(&[("Search", "")]),
    tool(
        "inserts",
        "Threads and inserts",
        "Holes for heat-set inserts, screws and taps.",
        inserts,
    )
    .form()
    .fields(&[("Search", "")]),
];

pub const PHOTO: &[Tool] = &[
    tool(
        "dof",
        "Depth of field",
        "Near and far limits of sharpness.",
        dof,
    )
    .form()
    .fields(&[
        ("Focal length (mm)", "50"),
        ("Aperture (f/)", "2.8"),
        ("Distance (m)", "3"),
    ])
    .selects(&[("Sensor", SENSORS)]),
    tool(
        "hyperfocal",
        "Hyperfocal distance",
        "Where to focus for the most depth of field.",
        hyperfocal,
    )
    .form()
    .fields(&[("Focal length (mm)", "24"), ("Aperture (f/)", "8")])
    .selects(&[("Sensor", SENSORS)]),
    tool(
        "exposure",
        "Exposure triangle",
        "EV of a setting, and the equivalent exposures.",
        exposure,
    )
    .form()
    .fields(&[
        ("Aperture (f/)", "8"),
        ("Shutter (s)", "1/125"),
        ("ISO", "100"),
    ]),
    tool(
        "flash",
        "Flash guide number",
        "How far a flash reaches.",
        flash,
    )
    .form()
    .fields(&[
        ("Guide number (m, ISO 100)", "40"),
        ("ISO", "200"),
        ("Aperture (f/)", "5.6"),
    ]),
    tool(
        "focal-length",
        "Focal length equivalent",
        "The full-frame equivalent of a lens.",
        focal,
    )
    .form()
    .fields(&[
        ("Focal length (mm)", "35"),
        ("Aperture (f/, optional)", "1.8"),
    ])
    .selects(&[(
        "Sensor",
        &[
            ("1.5", "APS-C (1.5×)"),
            ("1.6", "APS-C (1.6×)"),
            ("2", "Micro Four Thirds"),
            ("2.7", "1 inch"),
            ("1", "Full frame"),
            ("5.6", "Phone"),
        ],
    )]),
    tool(
        "image-size",
        "Image size estimator",
        "File sizes of photos.",
        image_size,
    )
    .form()
    .fields(&[("Megapixels", "24"), ("Bit depth", "14")]),
    tool(
        "megapixels",
        "Megapixel calculator",
        "Megapixels, aspect ratio and print sizes.",
        megapixels,
    )
    .form()
    .fields(&[("Width (px)", "6000"), ("Height (px)", "4000")]),
    tool(
        "nd-filter",
        "ND filter",
        "The exposure with an ND filter.",
        nd_filter,
    )
    .form()
    .fields(&[("Shutter without the filter", "1/60")])
    .selects(&[(
        "Filter",
        &[
            ("10", "ND1000 (10 stops)"),
            ("6", "ND64 (6 stops)"),
            ("3", "ND8 (3 stops)"),
            ("2", "ND4 (2 stops)"),
            ("1", "ND2 (1 stop)"),
            ("15", "ND32000 (15 stops)"),
        ],
    )]),
    tool(
        "dpi",
        "DPI calculator",
        "Print size from pixels, or resolution from a size.",
        dpi,
    )
    .form()
    .fields(&[("Pixels", "3000"), ("DPI, or size (cm)", "300")])
    .selects(&[(
        "Second value",
        &[("dpi", "DPI"), ("size", "Print size (cm)")],
    )]),
    tool(
        "star-trails",
        "Astrophotography exposure",
        "The longest exposure before stars trail (500 and NPF rules).",
        star_trails,
    )
    .form()
    .fields(&[
        ("Focal length (mm)", "14"),
        ("Aperture (f/)", "2.8"),
        ("Pixel pitch (µm)", "4.3"),
    ])
    .selects(&[("Sensor", SENSORS)]),
    tool(
        "timelapse",
        "Time-lapse calculator",
        "Frames, clip length and storage of a time-lapse.",
        timelapse,
    )
    .form()
    .fields(&[
        ("Event (minutes)", "120"),
        ("Interval (s)", "5"),
        ("Playback (fps)", "30"),
        ("Per frame (MB)", "25"),
    ]),
    tool(
        "aspect-crop",
        "Aspect ratio cropper",
        "The largest crop of an image to a ratio.",
        aspect_crop,
    )
    .form()
    .fields(&[
        ("Width (px)", "6000"),
        ("Height (px)", "4000"),
        ("Ratio", "16:9"),
    ]),
];

pub const MUSIC: &[Tool] = &[
    tool(
        "bpm",
        "BPM calculator",
        "Note lengths and delay times at a tempo.",
        bpm,
    )
    .form()
    .fields(&[("Tempo (BPM)", "120")]),
    tool(
        "tap-bpm",
        "Tempo from beats",
        "The tempo of beat times or intervals.",
        tap_bpm,
    )
    .text("Beat times in seconds (or intervals)"),
    tool(
        "note-frequency",
        "Note frequency",
        "A note's frequency, or the note of a frequency.",
        note_frequency,
    )
    .form()
    .fields(&[("Note or frequency", "A4"), ("A4 tuning (Hz)", "440")]),
    tool(
        "scale",
        "Scale explorer",
        "The notes and chords of a scale.",
        scale,
    )
    .form()
    .fields(&[("Root", "D")])
    .selects(&[(
        "Scale",
        &[
            ("major", "Major"),
            ("minor", "Natural minor"),
            ("harmonic", "Harmonic minor"),
            ("melodic", "Melodic minor"),
            ("dorian", "Dorian"),
            ("phrygian", "Phrygian"),
            ("lydian", "Lydian"),
            ("mixolydian", "Mixolydian"),
            ("locrian", "Locrian"),
            ("penta-major", "Major pentatonic"),
            ("penta-minor", "Minor pentatonic"),
            ("blues", "Blues"),
            ("chromatic", "Chromatic"),
        ],
    )]),
    tool(
        "progression",
        "Chord progression",
        "The chords of a Roman numeral progression in a key.",
        progression,
    )
    .form()
    .fields(&[
        ("Key (Am for minor)", "G"),
        ("Progression", "I - V - vi - IV"),
    ]),
    tool(
        "transpose",
        "Key transposer",
        "Transposes the chords of a song.",
        transpose,
    )
    .text("Song with chords")
    .fields(&[("Semitones", "2")])
    .checks(&["Use flats", "Change chords in every line"]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chemistry() {
        assert!((molar_mass("H2O").unwrap() - 18.015).abs() < 0.01);
        assert!((molar_mass("CuSO4·5H2O").unwrap() - 249.68).abs() < 0.05);
        assert!((molar_mass("Ca(OH)2").unwrap() - 74.09).abs() < 0.01);
        let f = |s: &str| parse_formula(s).unwrap();
        let k =
            balance_coefficients(&[vec![f("C3H8"), f("O2")], vec![f("CO2"), f("H2O")]]).unwrap();
        assert_eq!(k, vec![1, 5, 3, 4]);
        let k = balance_coefficients(&[vec![f("Fe"), f("O2")], vec![f("Fe2O3")]]).unwrap();
        assert_eq!(k, vec![4, 3, 2]);
        assert_eq!(electron_config(29), "1s² 2s² 2p⁶ 3s² 3p⁶ 3d¹⁰ 4s¹");
        assert_eq!(place(26), (4, Some(8), "transition metal"));
        assert_eq!(place(79).1, Some(11));
    }

    #[test]
    fn electronics() {
        assert_eq!(parse_si("4k7"), Some(4700.0));
        assert_eq!(parse_si("100n"), Some(1e-7));
        assert_eq!(si(4700.0, "Ω"), "4.7 kΩ");
        assert_eq!(nearest_e12(150.0, true), 150.0);
        assert_eq!(nearest_e12(160.0, true), 180.0);
    }

    #[test]
    fn music() {
        assert_eq!(note_index("C#4"), Some((1, Some(4))));
        assert_eq!(spell(10, true), "B♭");
    }
}
