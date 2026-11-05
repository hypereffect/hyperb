//! Colors, CSS generators, codes, fractals and placeholder images: the
//! tools that answer with a picture.

use super::Output;
use super::{Opts, Tool, fmt, table, tool};
use crate::img::{self, Canvas, Rgb};

// Colors

/// The CSS named colors.
const NAMED: &[(&str, u32)] = &[
    ("aliceblue", 0xf0f8ff),
    ("antiquewhite", 0xfaebd7),
    ("aqua", 0x00ffff),
    ("aquamarine", 0x7fffd4),
    ("azure", 0xf0ffff),
    ("beige", 0xf5f5dc),
    ("bisque", 0xffe4c4),
    ("black", 0x000000),
    ("blanchedalmond", 0xffebcd),
    ("blue", 0x0000ff),
    ("blueviolet", 0x8a2be2),
    ("brown", 0xa52a2a),
    ("burlywood", 0xdeb887),
    ("cadetblue", 0x5f9ea0),
    ("chartreuse", 0x7fff00),
    ("chocolate", 0xd2691e),
    ("coral", 0xff7f50),
    ("cornflowerblue", 0x6495ed),
    ("cornsilk", 0xfff8dc),
    ("crimson", 0xdc143c),
    ("cyan", 0x00ffff),
    ("darkblue", 0x00008b),
    ("darkcyan", 0x008b8b),
    ("darkgoldenrod", 0xb8860b),
    ("darkgray", 0xa9a9a9),
    ("darkgreen", 0x006400),
    ("darkkhaki", 0xbdb76b),
    ("darkmagenta", 0x8b008b),
    ("darkolivegreen", 0x556b2f),
    ("darkorange", 0xff8c00),
    ("darkorchid", 0x9932cc),
    ("darkred", 0x8b0000),
    ("darksalmon", 0xe9967a),
    ("darkseagreen", 0x8fbc8f),
    ("darkslateblue", 0x483d8b),
    ("darkslategray", 0x2f4f4f),
    ("darkturquoise", 0x00ced1),
    ("darkviolet", 0x9400d3),
    ("deeppink", 0xff1493),
    ("deepskyblue", 0x00bfff),
    ("dimgray", 0x696969),
    ("dodgerblue", 0x1e90ff),
    ("firebrick", 0xb22222),
    ("floralwhite", 0xfffaf0),
    ("forestgreen", 0x228b22),
    ("fuchsia", 0xff00ff),
    ("gainsboro", 0xdcdcdc),
    ("ghostwhite", 0xf8f8ff),
    ("gold", 0xffd700),
    ("goldenrod", 0xdaa520),
    ("gray", 0x808080),
    ("green", 0x008000),
    ("greenyellow", 0xadff2f),
    ("honeydew", 0xf0fff0),
    ("hotpink", 0xff69b4),
    ("indianred", 0xcd5c5c),
    ("indigo", 0x4b0082),
    ("ivory", 0xfffff0),
    ("khaki", 0xf0e68c),
    ("lavender", 0xe6e6fa),
    ("lavenderblush", 0xfff0f5),
    ("lawngreen", 0x7cfc00),
    ("lemonchiffon", 0xfffacd),
    ("lightblue", 0xadd8e6),
    ("lightcoral", 0xf08080),
    ("lightcyan", 0xe0ffff),
    ("lightgoldenrodyellow", 0xfafad2),
    ("lightgray", 0xd3d3d3),
    ("lightgreen", 0x90ee90),
    ("lightpink", 0xffb6c1),
    ("lightsalmon", 0xffa07a),
    ("lightseagreen", 0x20b2aa),
    ("lightskyblue", 0x87cefa),
    ("lightslategray", 0x778899),
    ("lightsteelblue", 0xb0c4de),
    ("lightyellow", 0xffffe0),
    ("lime", 0x00ff00),
    ("limegreen", 0x32cd32),
    ("linen", 0xfaf0e6),
    ("magenta", 0xff00ff),
    ("maroon", 0x800000),
    ("mediumaquamarine", 0x66cdaa),
    ("mediumblue", 0x0000cd),
    ("mediumorchid", 0xba55d3),
    ("mediumpurple", 0x9370db),
    ("mediumseagreen", 0x3cb371),
    ("mediumslateblue", 0x7b68ee),
    ("mediumspringgreen", 0x00fa9a),
    ("mediumturquoise", 0x48d1cc),
    ("mediumvioletred", 0xc71585),
    ("midnightblue", 0x191970),
    ("mintcream", 0xf5fffa),
    ("mistyrose", 0xffe4e1),
    ("moccasin", 0xffe4b5),
    ("navajowhite", 0xffdead),
    ("navy", 0x000080),
    ("oldlace", 0xfdf5e6),
    ("olive", 0x808000),
    ("olivedrab", 0x6b8e23),
    ("orange", 0xffa500),
    ("orangered", 0xff4500),
    ("orchid", 0xda70d6),
    ("palegoldenrod", 0xeee8aa),
    ("palegreen", 0x98fb98),
    ("paleturquoise", 0xafeeee),
    ("palevioletred", 0xdb7093),
    ("papayawhip", 0xffefd5),
    ("peachpuff", 0xffdab9),
    ("peru", 0xcd853f),
    ("pink", 0xffc0cb),
    ("plum", 0xdda0dd),
    ("powderblue", 0xb0e0e6),
    ("purple", 0x800080),
    ("rebeccapurple", 0x663399),
    ("red", 0xff0000),
    ("rosybrown", 0xbc8f8f),
    ("royalblue", 0x4169e1),
    ("saddlebrown", 0x8b4513),
    ("salmon", 0xfa8072),
    ("sandybrown", 0xf4a460),
    ("seagreen", 0x2e8b57),
    ("seashell", 0xfff5ee),
    ("sienna", 0xa0522d),
    ("silver", 0xc0c0c0),
    ("skyblue", 0x87ceeb),
    ("slateblue", 0x6a5acd),
    ("slategray", 0x708090),
    ("snow", 0xfffafa),
    ("springgreen", 0x00ff7f),
    ("steelblue", 0x4682b4),
    ("tan", 0xd2b48c),
    ("teal", 0x008080),
    ("thistle", 0xd8bfd8),
    ("tomato", 0xff6347),
    ("turquoise", 0x40e0d0),
    ("violet", 0xee82ee),
    ("wheat", 0xf5deb3),
    ("white", 0xffffff),
    ("whitesmoke", 0xf5f5f5),
    ("yellow", 0xffff00),
    ("yellowgreen", 0x9acd32),
];

fn rgb_of(n: u32) -> Rgb {
    [(n >> 16) as u8, (n >> 8) as u8, n as u8]
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [
        ((r + m) * 255.0).round() as u8,
        ((g + m) * 255.0).round() as u8,
        ((b + m) * 255.0).round() as u8,
    ]
}

fn rgb_to_hsl(c: Rgb) -> (f64, f64, f64) {
    let [r, g, b] = c.map(|v| v as f64 / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    if max == min {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, s, l)
}

/// A color as hex (`#abc`, `#aabbcc`), `rgb(…)`, `hsl(…)`, `cmyk(…)` or a
/// CSS name.
pub fn parse_color(s: &str) -> Result<Rgb, String> {
    let t = s.trim().to_lowercase();
    let nums = |inner: &str| -> Vec<f64> {
        inner
            .split([',', ' ', '/'])
            .filter(|p| !p.is_empty())
            .filter_map(|p| {
                p.trim_end_matches(['%', '°'])
                    .trim_end_matches("deg")
                    .parse()
                    .ok()
            })
            .collect()
    };
    if let Some((_, n)) = NAMED
        .iter()
        .find(|(name, _)| *name == t.replace(['-', ' '], "").replace("grey", "gray"))
    {
        return Ok(rgb_of(*n));
    }
    if let Some(inner) = t
        .strip_prefix("rgb(")
        .or_else(|| t.strip_prefix("rgba("))
        .and_then(|r| r.strip_suffix(')'))
    {
        let v = nums(inner);
        if v.len() >= 3 {
            return Ok([v[0], v[1], v[2]].map(|x| x.clamp(0.0, 255.0).round() as u8));
        }
    }
    if let Some(inner) = t
        .strip_prefix("hsl(")
        .or_else(|| t.strip_prefix("hsla("))
        .and_then(|r| r.strip_suffix(')'))
    {
        let v = nums(inner);
        if v.len() >= 3 {
            return Ok(hsl_to_rgb(v[0], v[1] / 100.0, v[2] / 100.0));
        }
    }
    if let Some(inner) = t.strip_prefix("cmyk(").and_then(|r| r.strip_suffix(')')) {
        let v = nums(inner);
        if v.len() == 4 {
            let k = 1.0 - v[3] / 100.0;
            return Ok([v[0], v[1], v[2]].map(|c| (255.0 * (1.0 - c / 100.0) * k).round() as u8));
        }
    }
    let hex = t.trim_start_matches('#');
    let full: String = match hex.len() {
        3 | 4 => hex.chars().take(3).flat_map(|c| [c, c]).collect(),
        6 | 8 => hex[..6].to_owned(),
        _ => String::new(),
    };
    if let Ok(n) = u32::from_str_radix(&full, 16) {
        return Ok(rgb_of(n));
    }
    // Bare numbers: "255, 0, 128".
    let v = nums(&t);
    if v.len() == 3 && v.iter().all(|x| (0.0..=255.0).contains(x)) {
        return Ok([v[0] as u8, v[1] as u8, v[2] as u8]);
    }
    Err(format!(
        "“{s}” is not a color: use #ff8800, rgb(255, 136, 0), hsl(32, 100%, 50%) or a name."
    ))
}

pub fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn cmyk(c: Rgb) -> (f64, f64, f64, f64) {
    let [r, g, b] = c.map(|v| v as f64 / 255.0);
    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 {
        return (0.0, 0.0, 0.0, 100.0);
    }
    (
        (1.0 - r - k) / (1.0 - k) * 100.0,
        (1.0 - g - k) / (1.0 - k) * 100.0,
        (1.0 - b - k) / (1.0 - k) * 100.0,
        k * 100.0,
    )
}

fn linear(v: u8) -> f64 {
    let c = v as f64 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn gamma(v: f64) -> u8 {
    let c = if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (c.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn luminance(c: Rgb) -> f64 {
    0.2126 * linear(c[0]) + 0.7152 * linear(c[1]) + 0.0722 * linear(c[2])
}

fn lab(c: Rgb) -> (f64, f64, f64) {
    let [r, g, b] = c.map(linear);
    let (x, y, z) = (
        (0.4124 * r + 0.3576 * g + 0.1805 * b) / 0.95047,
        0.2126 * r + 0.7152 * g + 0.0722 * b,
        (0.0193 * r + 0.1192 * g + 0.9505 * b) / 1.08883,
    );
    let f = |t: f64| {
        if t > 0.008856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };
    (
        116.0 * f(y) - 16.0,
        500.0 * (f(x) - f(y)),
        200.0 * (f(y) - f(z)),
    )
}

fn nearest_name(c: Rgb) -> (&'static str, f64) {
    let l = lab(c);
    NAMED
        .iter()
        .map(|(n, v)| {
            let m = lab(rgb_of(*v));
            (
                *n,
                ((l.0 - m.0).powi(2) + (l.1 - m.1).powi(2) + (l.2 - m.2).powi(2)).sqrt(),
            )
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap()
}

/// A strip of swatches.
fn swatches(colors: &[Rgb]) -> Vec<u8> {
    let w = 80;
    let mut c = Canvas::new(w * colors.len() as i32, 80, [255, 255, 255]);
    for (i, col) in colors.iter().enumerate() {
        c.rect(i as i32 * w, 0, w, 80, *col);
    }
    c.png()
}

fn describe(c: Rgb) -> Vec<(&'static str, String)> {
    let (h, s, l) = rgb_to_hsl(c);
    let (cc, m, y, k) = cmyk(c);
    let [r, g, b] = c.map(|v| v as f64 / 255.0);
    let max = r.max(g).max(b);
    let v_s = if max == 0.0 {
        0.0
    } else {
        (max - r.min(g).min(b)) / max
    };
    let (ll, aa, bb) = lab(c);
    let (name, d) = nearest_name(c);
    vec![
        ("HEX", hex(c)),
        ("RGB", format!("rgb({}, {}, {})", c[0], c[1], c[2])),
        (
            "HSL",
            format!("hsl({:.0}, {:.0}%, {:.0}%)", h, s * 100.0, l * 100.0),
        ),
        (
            "HSV",
            format!("hsv({:.0}, {:.0}%, {:.0}%)", h, v_s * 100.0, max * 100.0),
        ),
        (
            "CMYK",
            format!("cmyk({:.0}%, {:.0}%, {:.0}%, {:.0}%)", cc, m, y, k),
        ),
        ("CIELAB", format!("lab({ll:.1}, {aa:.1}, {bb:.1})")),
        (
            "Nearest name",
            if d < 0.5 {
                name.to_owned()
            } else {
                format!("{name} (ΔE {d:.1})")
            },
        ),
    ]
}

fn color_convert(_: &str, o: &mut Opts) -> Result<Output, String> {
    let c = parse_color(o.field(0))?;
    Ok(Output::default()
        .image(swatches(&[c]), hex(c))
        .then(table(&describe(c))))
}

fn cmyk_rgb(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v: Vec<f64> = (0..4).map(|i| o.num(i)).collect::<Result<_, _>>()?;
    let c = parse_color(&format!("cmyk({}, {}, {}, {})", v[0], v[1], v[2], v[3]))?;
    Ok(Output::default()
        .image(swatches(&[c]), hex(c))
        .then(table(&describe(c))))
}

fn harmonies(_: &str, o: &mut Opts) -> Result<Output, String> {
    let base = parse_color(o.field(0))?;
    let (h, s, l) = rgb_to_hsl(base);
    let colors: Vec<Rgb> = match o.mode {
        "analogous" => [-30.0, 0.0, 30.0]
            .iter()
            .map(|d| hsl_to_rgb(h + d, s, l))
            .collect(),
        "triadic" => [0.0, 120.0, 240.0]
            .iter()
            .map(|d| hsl_to_rgb(h + d, s, l))
            .collect(),
        "split" => [0.0, 150.0, 210.0]
            .iter()
            .map(|d| hsl_to_rgb(h + d, s, l))
            .collect(),
        "tetradic" => [0.0, 90.0, 180.0, 270.0]
            .iter()
            .map(|d| hsl_to_rgb(h + d, s, l))
            .collect(),
        "monochromatic" => [0.2, 0.35, 0.5, 0.65, 0.8]
            .iter()
            .map(|ll| hsl_to_rgb(h, s, *ll))
            .collect(),
        "shades" => (0..6)
            .map(|i| hsl_to_rgb(h, s, l * (1.0 - i as f64 / 6.0)))
            .collect(),
        "tints" => (0..6)
            .map(|i| hsl_to_rgb(h, s, l + (1.0 - l) * i as f64 / 6.0))
            .collect(),
        _ => [0.0, 180.0]
            .iter()
            .map(|d| hsl_to_rgb(h + d, s, l))
            .collect(),
    };
    let names: Vec<String> = colors.iter().map(|c| hex(*c)).collect();
    Ok(Output::default()
        .image(swatches(&colors), names.join(", "))
        .table(
            &["Color", "RGB"],
            colors
                .iter()
                .map(|c| vec![hex(*c), format!("rgb({}, {}, {})", c[0], c[1], c[2])]),
        ))
}

fn mix(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (a, b) = (parse_color(o.field(0))?, parse_color(o.field(1))?);
    let t = o.number as f64 / 100.0;
    let screen = [0, 1, 2].map(|i| (a[i] as f64 * (1.0 - t) + b[i] as f64 * t).round() as u8);
    // Paint mixes subtract light: mix the absorbances.
    let paint = [0, 1, 2].map(|i| {
        gamma((linear(a[i]).max(1e-4).ln() * (1.0 - t) + linear(b[i]).max(1e-4).ln() * t).exp())
    });
    let lin = [0, 1, 2].map(|i| gamma(linear(a[i]) * (1.0 - t) + linear(b[i]) * t));
    let shown = [a, paint, lin, screen, b];
    Ok(Output::default()
        .image(swatches(&shown), shown.map(hex).join(", "))
        .then(table(&[
            ("Paint (subtractive)", hex(paint)),
            ("Light (linear)", hex(lin)),
            ("Average of the values", hex(screen)),
            (
                "",
                format!(
                    "{}% of the second color; the swatches show A, the three mixes, and B.",
                    o.number
                ),
            ),
        ])))
}

fn name_lookup(_: &str, o: &mut Opts) -> Result<Output, String> {
    let c = parse_color(o.field(0))?;
    let l = lab(c);
    let mut ranked: Vec<(&str, u32, f64)> = NAMED
        .iter()
        .map(|(n, v)| {
            let m = lab(rgb_of(*v));
            (
                *n,
                *v,
                ((l.0 - m.0).powi(2) + (l.1 - m.1).powi(2) + (l.2 - m.2).powi(2)).sqrt(),
            )
        })
        .collect();
    ranked.sort_by(|a, b| a.2.total_cmp(&b.2));
    let shown: Vec<Rgb> = std::iter::once(c)
        .chain(ranked.iter().take(4).map(|r| rgb_of(r.1)))
        .collect();
    Ok(Output::default()
        .image(
            swatches(&shown),
            format!("{} and its nearest named colors", hex(c)),
        )
        .table(
            &["Name", "Color", "Distance"],
            ranked
                .iter()
                .take(5)
                .map(|(n, v, d)| vec![(*n).to_owned(), hex(rgb_of(*v)), format!("ΔE {d:.1}")]),
        ))
}

fn contrast(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (fg, bg) = (parse_color(o.field(0))?, parse_color(o.field(1))?);
    let (l1, l2) = (luminance(fg), luminance(bg));
    let ratio = (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05);
    let pass = |need: f64| {
        if ratio >= need {
            "✓ pass"
        } else {
            "✗ fail"
        }
    };
    let mut c = Canvas::new(360, 120, bg);
    c.text(20, 22, 6, "Aa 123", fg);
    c.text(20, 86, 2, "THE QUICK BROWN FOX", fg);
    Ok(Output::default()
        .image(c.png(), format!("{} text on {}", hex(fg), hex(bg)))
        .then(table(&[
            ("Contrast ratio", format!("{}:1", fmt(ratio, 2))),
            ("AA, normal text (4.5)", pass(4.5).into()),
            ("AA, large text (3)", pass(3.0).into()),
            ("AAA, normal text (7)", pass(7.0).into()),
            ("AAA, large text (4.5)", pass(4.5).into()),
            ("Interface parts (3)", pass(3.0).into()),
        ])))
}

fn blindness(_: &str, o: &mut Opts) -> Result<Output, String> {
    let c = parse_color(o.field(0))?;
    let lin = c.map(linear);
    let apply = |m: [[f64; 3]; 3]| -> Rgb {
        [0, 1, 2].map(|i| gamma(m[i][0] * lin[0] + m[i][1] * lin[1] + m[i][2] * lin[2]))
    };
    // Machado, Oliveira and Fernandes (2009), full severity.
    let kinds = [
        (
            "Protanopia (no red)",
            [
                [0.152286, 1.052583, -0.204868],
                [0.114503, 0.786281, 0.099216],
                [-0.003882, -0.048116, 1.051998],
            ],
        ),
        (
            "Deuteranopia (no green)",
            [
                [0.367322, 0.860646, -0.227968],
                [0.280085, 0.672501, 0.047413],
                [-0.011820, 0.042940, 0.968881],
            ],
        ),
        (
            "Tritanopia (no blue)",
            [
                [1.255528, -0.076749, -0.178779],
                [-0.078411, 0.930809, 0.147602],
                [0.004733, 0.691367, 0.303900],
            ],
        ),
    ];
    let mut colors = vec![c];
    let mut rows = vec![("Normal vision", hex(c))];
    for (name, m) in kinds {
        let s = apply(m);
        colors.push(s);
        rows.push((name, hex(s)));
    }
    let grey = gamma(luminance(c));
    colors.push([grey; 3]);
    rows.push(("Achromatopsia (no color)", hex([grey; 3])));
    Ok(Output::default()
        .image(
            swatches(&colors),
            format!("{} as seen with each kind of color blindness", hex(c)),
        )
        .then(table(&rows)))
}

fn gradient(_: &str, o: &mut Opts) -> Result<Output, String> {
    let stops: Vec<Rgb> = o
        .field(0)
        .split([',', ' '])
        .filter(|s| !s.is_empty())
        .map(parse_color)
        .collect::<Result<_, _>>()?;
    if stops.len() < 2 {
        return Err("Write two or more colors, separated by commas.".into());
    }
    let angle = o.opt_num(1)?.unwrap_or(90.0);
    let names: Vec<String> = stops.iter().map(|c| hex(*c)).collect();
    let css = match o.mode {
        "radial" => format!("background: radial-gradient(circle, {});", names.join(", ")),
        "conic" => format!(
            "background: conic-gradient(from {}deg, {});",
            fmt(angle, 0),
            names.join(", ")
        ),
        _ => format!(
            "background: linear-gradient({}deg, {});",
            fmt(angle, 0),
            names.join(", ")
        ),
    };
    let (w, h) = (400, 160);
    let mut c = Canvas::new(w, h, [255, 255, 255]);
    let a = (angle - 90.0).to_radians();
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = (x as f64 - w as f64 / 2.0, y as f64 - h as f64 / 2.0);
            let t = match o.mode {
                "radial" => (dx.hypot(dy) / (w as f64 / 2.0).hypot(h as f64 / 2.0)).min(1.0),
                "conic" => {
                    ((dx.atan2(-dy) - (angle).to_radians()).rem_euclid(std::f64::consts::TAU))
                        / std::f64::consts::TAU
                }
                _ => {
                    let len = (w as f64 * a.cos()).abs() + (h as f64 * a.sin()).abs();
                    ((dx * a.cos() + dy * a.sin()) / len + 0.5).clamp(0.0, 1.0)
                }
            };
            let seg = t * (stops.len() - 1) as f64;
            let i = (seg.floor() as usize).min(stops.len() - 2);
            let f = seg - i as f64;
            let col = [0, 1, 2].map(|k| {
                (stops[i][k] as f64 * (1.0 - f) + stops[i + 1][k] as f64 * f).round() as u8
            });
            c.put(x, y, col);
        }
    }
    Ok(Output::default()
        .image(c.png(), names.join(" to "))
        .code(css))
}

fn box_shadow(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (x, y, blur, spread) = (
        o.num(0)?,
        o.num(1)?,
        o.num(2)?,
        o.opt_num(3)?.unwrap_or(0.0),
    );
    let color = parse_color(o.field(4))?;
    let alpha = o.number as f64 / 100.0;
    let inset = if o.flag { "inset " } else { "" };
    Ok(format!(
        "box-shadow: {inset}{}px {}px {}px {}px rgba({}, {}, {}, {});",
        fmt(x, 1),
        fmt(y, 1),
        fmt(blur, 1),
        fmt(spread, 1),
        color[0],
        color[1],
        color[2],
        fmt(alpha, 2)
    )
    .into())
}

fn border_radius(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v: Vec<f64> = (0..4)
        .map(|i| o.opt_num(i).map(|v| v.unwrap_or(0.0)))
        .collect::<Result<_, _>>()?;
    let unit = o.mode;
    let all_same = v.iter().all(|x| *x == v[0]);
    let s = |x: f64| format!("{}{unit}", fmt(x, 2));
    Ok(if all_same {
        format!("border-radius: {};", s(v[0]))
    } else {
        format!(
            "border-radius: {} {} {} {};",
            s(v[0]),
            s(v[1]),
            s(v[2]),
            s(v[3])
        )
    }
    .into())
}

fn glass(_: &str, o: &mut Opts) -> Result<Output, String> {
    let c = parse_color(o.field(0))?;
    let blur = o.opt_num(1)?.unwrap_or(10.0);
    let alpha = o.number as f64 / 100.0;
    Ok(format!(
        "background: rgba({}, {}, {}, {});\nbackdrop-filter: blur({}px) saturate(180%);\n-webkit-backdrop-filter: blur({}px) saturate(180%);\nborder: 1px solid rgba(255, 255, 255, 0.25);\nborder-radius: 16px;\nbox-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);",
        c[0], c[1], c[2], fmt(alpha, 2), fmt(blur, 0), fmt(blur, 0)
    ).into())
}

fn transform(_: &str, o: &mut Opts) -> Result<Output, String> {
    let parts = [
        (o.opt_num(0)?, "rotate", "deg", 0.0),
        (o.opt_num(1)?, "scale", "", 1.0),
        (o.opt_num(2)?, "translateX", "px", 0.0),
        (o.opt_num(3)?, "translateY", "px", 0.0),
        (o.opt_num(4)?, "skewX", "deg", 0.0),
        (o.opt_num(5)?, "skewY", "deg", 0.0),
    ];
    let t: Vec<String> = parts
        .iter()
        .filter_map(|(v, f, u, idle)| {
            v.filter(|x| x != idle)
                .map(|x| format!("{f}({}{u})", fmt(x, 3)))
        })
        .collect();
    Ok(if t.is_empty() {
        "transform: none;".into()
    } else {
        format!("transform: {};", t.join(" "))
    }
    .into())
}

fn aspect_ratio(_: &str, o: &mut Opts) -> Result<Output, String> {
    let w = o.positive(0)?;
    let h = o.positive(1)?;
    let g = super::math::gcd(w.round() as u128, h.round() as u128).max(1) as f64;
    let mut rows = vec![
        (
            "Ratio",
            format!("{}:{}", fmt(w.round() / g, 0), fmt(h.round() / g, 0)),
        ),
        ("Decimal", fmt(w / h, 4)),
    ];
    if let Some(nw) = o.opt_num(2)? {
        rows.push(("Height for the new width", fmt(nw * h / w, 2)));
    }
    if let Some(nh) = o.opt_num(3)? {
        rows.push(("Width for the new height", fmt(nh * w / h, 2)));
    }
    Ok(table(&rows))
}

fn font_size(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.num(0)?;
    let base = o.opt_num(1)?.unwrap_or(16.0);
    let px = match o.mode {
        "rem" | "em" => v * base,
        "pt" => v * 4.0 / 3.0,
        "percent" => v / 100.0 * base,
        "vw" => v / 100.0 * o.opt_num(2)?.unwrap_or(1440.0),
        _ => v,
    };
    Ok(table(&[
        ("Pixels", format!("{}px", fmt(px, 3))),
        ("rem / em", format!("{}rem", fmt(px / base, 4))),
        ("Points", format!("{}pt", fmt(px * 0.75, 3))),
        ("Percent", format!("{}%", fmt(px / base * 100.0, 2))),
        (
            "vw (at the viewport width)",
            format!("{}vw", fmt(px / o.opt_num(2)?.unwrap_or(1440.0) * 100.0, 4)),
        ),
    ]))
}

// Codes

fn qr_png(data: &str, scale: usize) -> Result<(Vec<u8>, usize), String> {
    let code =
        qrcode::QrCode::new(data.as_bytes()).map_err(|e| format!("Cannot encode that: {e}."))?;
    let modules = code.width();
    let quiet = 4;
    let side = (modules + 2 * quiet) * scale;
    let colors = code.to_colors();
    let mut white = vec![true; side * side];
    for (i, c) in colors.iter().enumerate() {
        if *c == qrcode::Color::Dark {
            let (mx, my) = (i % modules + quiet, i / modules + quiet);
            for y in my * scale..(my + 1) * scale {
                for x in mx * scale..(mx + 1) * scale {
                    white[y * side + x] = false;
                }
            }
        }
    }
    Ok((img::png_bilevel(side as u32, side as u32, &white), modules))
}

fn qr(text: &str, o: &mut Opts) -> Result<Output, String> {
    let (png, modules) = qr_png(text, o.number.max(2))?;
    Ok(Output::default()
        .image(png, format!("A QR code for: {text}"))
        .note(format!(
            "{modules}×{modules} modules for {} bytes",
            text.len()
        )))
}

fn wifi_qr(_: &str, o: &mut Opts) -> Result<Output, String> {
    let esc = |s: &str| {
        s.replace('\\', "\\\\")
            .replace(';', "\\;")
            .replace(',', "\\,")
            .replace(':', "\\:")
            .replace('"', "\\\"")
    };
    let ssid = o.field(0);
    if ssid.is_empty() {
        return Err("Fill in the network name.".into());
    }
    let data = format!(
        "WIFI:T:{};S:{};P:{};H:{};;",
        o.mode,
        esc(ssid),
        esc(o.field(1)),
        o.flag
    );
    let (png, _) = qr_png(&data, 8)?;
    Ok(Output::default()
        .image(png, format!("A QR code to join the network “{ssid}”"))
        .note(format!("Scan with a phone camera to join “{ssid}”."))
        .code(data))
}

const CODE128: [&str; 107] = [
    "212222", "222122", "222221", "121223", "121322", "131222", "122213", "122312", "132212",
    "221213", "221312", "231212", "112232", "122132", "122231", "113222", "123122", "123221",
    "223211", "221132", "221231", "213212", "223112", "312131", "311222", "321122", "321221",
    "312212", "322112", "322211", "212123", "212321", "232121", "111323", "131123", "131321",
    "112313", "132113", "132311", "211313", "231113", "231311", "112133", "112331", "132131",
    "113123", "113321", "133121", "313121", "211331", "231131", "213113", "213311", "213131",
    "311123", "311321", "331121", "312113", "312311", "332111", "314111", "221411", "431111",
    "111224", "111422", "121124", "121421", "141122", "141221", "112214", "112412", "122114",
    "122411", "142112", "142211", "241211", "221114", "413111", "241112", "134111", "111242",
    "121142", "121241", "114212", "124112", "124211", "411212", "421112", "421211", "212141",
    "214121", "412121", "111143", "111341", "131141", "114113", "114311", "411113", "411311",
    "113141", "114131", "311141", "411131", "211412", "211214", "211232", "2331112",
];

/// Bar widths of a Code 128 B symbol, bars and spaces alternating.
fn code128(s: &str) -> Result<Vec<u8>, String> {
    let mut codes = vec![104usize];
    for c in s.chars() {
        let v = c as usize;
        if !(32..=127).contains(&v) {
            return Err(format!("Code 128 B holds ASCII only, not “{c}”."));
        }
        codes.push(v - 32);
    }
    let check = codes
        .iter()
        .enumerate()
        .map(|(i, c)| c * i.max(1))
        .sum::<usize>()
        % 103;
    codes.push(check);
    codes.push(106);
    Ok(codes
        .iter()
        .flat_map(|c| CODE128[*c].bytes().map(|b| b - b'0'))
        .collect())
}

const EAN_L: [&str; 10] = [
    "0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011",
    "0110111", "0001011",
];
const EAN_FIRST: [&str; 10] = [
    "LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG", "LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL",
    "LGGLGL",
];

/// Modules of an EAN-13 symbol, `true` for black.
fn ean13(s: &str) -> Result<(Vec<bool>, String), String> {
    let digits: Vec<u32> = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| {
            c.to_digit(10)
                .ok_or("EAN-13 holds digits only.".to_string())
        })
        .collect::<Result<_, _>>()?;
    let mut d = match digits.len() {
        12 => digits,
        13 => digits[..12].to_vec(),
        _ => return Err("An EAN-13 has 12 digits, or 13 with the check digit.".into()),
    };
    let check = (10
        - d.iter()
            .enumerate()
            .map(|(i, x)| x * if i % 2 == 0 { 1 } else { 3 })
            .sum::<u32>()
            % 10)
        % 10;
    if s.chars().filter(char::is_ascii_digit).count() == 13
        && s.chars()
            .filter(char::is_ascii_digit)
            .last()
            .and_then(|c| c.to_digit(10))
            != Some(check)
    {
        return Err(format!("The check digit should be {check}."));
    }
    d.push(check);
    let mut bits = String::from("101");
    let pattern = EAN_FIRST[d[0] as usize];
    for (i, x) in d[1..7].iter().enumerate() {
        let l = EAN_L[*x as usize];
        bits.push_str(&if pattern.as_bytes()[i] == b'L' {
            l.to_owned()
        } else {
            l.chars()
                .rev()
                .map(|c| if c == '0' { '1' } else { '0' })
                .collect()
        });
    }
    bits.push_str("01010");
    for x in &d[7..] {
        bits.extend(
            EAN_L[*x as usize]
                .chars()
                .map(|c| if c == '0' { '1' } else { '0' }),
        );
    }
    bits.push_str("101");
    Ok((
        bits.chars().map(|c| c == '1').collect(),
        d.iter().map(|x| x.to_string()).collect(),
    ))
}

fn barcode(_: &str, o: &mut Opts) -> Result<Output, String> {
    let data = o.field(0);
    let (modules, label): (Vec<bool>, String) = if o.mode == "ean13" {
        ean13(data)?
    } else {
        if data.is_empty() {
            return Err("Fill in the text to encode.".into());
        }
        let widths = code128(data)?;
        let mut m = Vec::new();
        for (i, w) in widths.iter().enumerate() {
            m.extend(std::iter::repeat_n(i % 2 == 0, *w as usize));
        }
        (m, data.to_owned())
    };
    let scale = 3;
    let quiet = 10 * scale;
    let width = modules.len() * scale + 2 * quiet;
    let mut c = Canvas::new(width as i32, 130, [255, 255, 255]);
    for (i, black) in modules.iter().enumerate() {
        if *black {
            c.rect((quiet + i * scale) as i32, 10, scale as i32, 90, [0, 0, 0]);
        }
    }
    let tw = img::text_width(&label, 2);
    if tw < width as i32 {
        c.text((width as i32 - tw) / 2, 108, 2, &label, [0, 0, 0]);
    }
    Ok(Output::default()
        .image(c.png(), format!("A barcode for {label}"))
        .note(format!("{} modules", modules.len())))
}

// Fractals

/// Turtle points of an L-system after `n` rewrites.
fn lsystem(
    axiom: &str,
    rules: &[(char, &str)],
    angle: f64,
    n: usize,
    start: f64,
) -> Vec<Vec<(f64, f64)>> {
    let mut s = axiom.to_owned();
    for _ in 0..n {
        s = s
            .chars()
            .map(|c| {
                rules
                    .iter()
                    .find(|r| r.0 == c)
                    .map_or(c.to_string(), |r| r.1.to_owned())
            })
            .collect();
        if s.len() > 2_000_000 {
            break;
        }
    }
    let (mut x, mut y, mut a) = (0.0, 0.0, start.to_radians());
    let mut paths = vec![vec![(x, y)]];
    let mut stack = Vec::new();
    for c in s.chars() {
        match c {
            'F' | 'G' | 'A' | 'B' => {
                x += a.cos();
                y += a.sin();
                paths.last_mut().unwrap().push((x, y));
            }
            '+' => a += angle.to_radians(),
            '-' => a -= angle.to_radians(),
            '[' => stack.push((x, y, a)),
            ']' => {
                if let Some(p) = stack.pop() {
                    (x, y, a) = p;
                    paths.push(vec![(x, y)]);
                }
            }
            _ => {}
        }
    }
    paths
}

fn tree(
    x: f64,
    y: f64,
    a: f64,
    len: f64,
    depth: usize,
    spread: f64,
    ratio: f64,
    out: &mut Vec<Vec<(f64, f64)>>,
) {
    if depth == 0 {
        return;
    }
    let (nx, ny) = (x + len * a.cos(), y + len * a.sin());
    out.push(vec![(x, y), (nx, ny)]);
    tree(
        nx,
        ny,
        a - spread,
        len * ratio,
        depth - 1,
        spread,
        ratio,
        out,
    );
    tree(
        nx,
        ny,
        a + spread,
        len * ratio,
        depth - 1,
        spread,
        ratio,
        out,
    );
}

fn pythagoras(p: (f64, f64), q: (f64, f64), depth: usize, out: &mut Vec<Vec<(f64, f64)>>) {
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let (r, s) = ((q.0 - dy, q.1 + dx), (p.0 - dy, p.1 + dx));
    out.push(vec![p, q, r, s, p]);
    if depth == 0 {
        return;
    }
    let apex = (s.0 + (dx - dy) / 2.0, s.1 + (dy + dx) / 2.0);
    pythagoras(s, apex, depth - 1, out);
    pythagoras(apex, r, depth - 1, out);
}

fn draw(paths: &[Vec<(f64, f64)>], color: Rgb) -> Vec<u8> {
    let pts = paths.iter().flatten();
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for (x, y) in pts {
        x0 = x0.min(*x);
        y0 = y0.min(*y);
        x1 = x1.max(*x);
        y1 = y1.max(*y);
    }
    let size = 560.0;
    let k = (size - 40.0) / (x1 - x0).max(y1 - y0).max(1e-9);
    let (ox, oy) = ((size - (x1 - x0) * k) / 2.0, (size - (y1 - y0) * k) / 2.0);
    let mut c = Canvas::new(size as i32, size as i32, [255, 255, 255]);
    for path in paths {
        for w in path.windows(2) {
            let p = |(x, y): (f64, f64)| {
                (
                    (ox + (x - x0) * k) as i32,
                    (size - oy - (y - y0) * k) as i32,
                )
            };
            c.line(p(w[0]), p(w[1]), 1, color);
        }
    }
    c.png()
}

fn fractal(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = o.number.max(1);
    let color = parse_color(o.field(0)).unwrap_or([30, 60, 140]);
    let paths = match o.mode {
        "koch" => lsystem("F--F--F", &[('F', "F+F--F+F")], 60.0, n.min(6), 0.0),
        "dragon" => lsystem(
            "FX",
            &[('X', "X+YF+"), ('Y', "-FX-Y")],
            90.0,
            n.min(16),
            0.0,
        ),
        "hilbert" => lsystem(
            "A",
            &[('A', "+BF-AFA-FB+"), ('B', "-AF+BFB+FA-")],
            90.0,
            n.min(8),
            0.0,
        ),
        "peano" => lsystem(
            "X",
            &[
                ('X', "XFYFX+F+YFXFY-F-XFYFX"),
                ('Y', "YFXFY-F-XFYFX+F+YFXFY"),
            ],
            90.0,
            n.min(5),
            0.0,
        ),
        "gosper" => lsystem(
            "A",
            &[('A', "A-B--B+A++AA+B-"), ('B', "+A-BB--B-A++A+B")],
            60.0,
            n.min(5),
            0.0,
        ),
        "levy" => lsystem("F", &[('F', "+F--F+")], 45.0, n.min(16), 0.0),
        "plant" => lsystem(
            "X",
            &[('X', "F+[[X]-X]-F[-FX]+X"), ('F', "FF")],
            25.0,
            n.min(7),
            65.0,
        ),
        "canopy" => {
            let mut out = Vec::new();
            tree(
                0.0,
                0.0,
                std::f64::consts::FRAC_PI_2,
                1.0,
                n.min(14),
                0.45,
                0.7,
                &mut out,
            );
            out
        }
        "pythagoras" => {
            let mut out = Vec::new();
            pythagoras((0.0, 0.0), (1.0, 0.0), n.min(12), &mut out);
            out
        }
        _ => lsystem("A", &[('A', "B-A-B"), ('B', "A+B+A")], 60.0, n.min(9), 0.0),
    };
    let segments = paths
        .iter()
        .map(|p| p.len().saturating_sub(1))
        .sum::<usize>();
    Ok(Output::default()
        .image(draw(&paths, color), format!("A {} fractal", o.mode))
        .note(format!("{segments} segments")))
}

fn placeholder(_: &str, o: &mut Opts) -> Result<Output, String> {
    let w = o.positive(0)?.round() as i32;
    let h = o.positive(1)?.round() as i32;
    if w > 1600 || h > 1600 {
        return Err("At most 1600 pixels a side.".into());
    }
    let bg = parse_color(if o.field(2).is_empty() {
        "#cccccc"
    } else {
        o.field(2)
    })?;
    let fg = parse_color(if o.field(3).is_empty() {
        "#555555"
    } else {
        o.field(3)
    })?;
    let label = if o.field(4).is_empty() {
        format!("{w} X {h}")
    } else {
        o.field(4).to_owned()
    };
    let mut c = Canvas::new(w, h, bg);
    let mut scale = 8;
    while scale > 1 && (img::text_width(&label, scale) > w * 9 / 10 || 7 * scale > h / 2) {
        scale -= 1;
    }
    c.text(
        (w - img::text_width(&label, scale)) / 2,
        (h - 7 * scale) / 2,
        scale,
        &label,
        fg,
    );
    Ok(Output::default()
        .image(c.png(), format!("A {w}×{h} placeholder reading “{label}”"))
        .note(format!("{w}×{h} PNG")))
}

pub const COLOR: &[Tool] = &[
    tool(
        "color",
        "Color converter",
        "A color in HEX, RGB, HSL, HSV, CMYK and Lab, with its nearest name.",
        color_convert,
    )
    .form()
    .fields(&[("Color", "#3d8bfd")]),
    tool(
        "cmyk-rgb",
        "CMYK to RGB",
        "A CMYK color as RGB and HEX.",
        cmyk_rgb,
    )
    .form()
    .fields(&[
        ("Cyan (%)", "76"),
        ("Magenta (%)", "45"),
        ("Yellow (%)", "0"),
        ("Black (%)", "1"),
    ]),
    tool(
        "harmonies",
        "Color harmonies",
        "Complementary, triadic, analogous and other color schemes.",
        harmonies,
    )
    .form()
    .fields(&[("Base color", "#e4572e")])
    .selects(&[(
        "Scheme",
        &[
            ("complementary", "Complementary"),
            ("analogous", "Analogous"),
            ("triadic", "Triadic"),
            ("split", "Split complementary"),
            ("tetradic", "Tetradic"),
            ("monochromatic", "Monochromatic"),
            ("shades", "Shades"),
            ("tints", "Tints"),
        ],
    )]),
    tool(
        "mix",
        "Color mixer",
        "Mixes two colors like paint and like light.",
        mix,
    )
    .form()
    .fields(&[("Color A", "yellow"), ("Color B", "blue")])
    .slider("Share of B (%)", 0.0, 100.0, 50.0),
    tool(
        "color-name",
        "Color name lookup",
        "The CSS color names closest to a color.",
        name_lookup,
    )
    .form()
    .fields(&[("Color", "#4b6f8a")]),
    tool(
        "contrast",
        "WCAG contrast checker",
        "The contrast ratio of text on a background, against WCAG levels.",
        contrast,
    )
    .form()
    .fields(&[("Text color", "#767676"), ("Background", "#ffffff")]),
    tool(
        "color-blindness",
        "Color blindness simulator",
        "How a color looks with each kind of color blindness.",
        blindness,
    )
    .form()
    .fields(&[("Color", "#e4572e")]),
    tool(
        "gradient",
        "CSS gradient",
        "A CSS gradient and its preview.",
        gradient,
    )
    .form()
    .fields(&[("Colors", "#ff7e5f, #feb47b, #86a8e7"), ("Angle (°)", "90")])
    .selects(&[(
        "Type",
        &[
            ("linear", "Linear"),
            ("radial", "Radial"),
            ("conic", "Conic"),
        ],
    )]),
    tool(
        "box-shadow",
        "CSS box shadow",
        "A CSS box-shadow rule.",
        box_shadow,
    )
    .form()
    .fields(&[
        ("Offset X (px)", "0"),
        ("Offset Y (px)", "8"),
        ("Blur (px)", "24"),
        ("Spread (px)", "0"),
        ("Color", "black"),
    ])
    .checks(&["Inset"])
    .slider("Opacity (%)", 0.0, 100.0, 20.0),
    tool(
        "border-radius",
        "CSS border radius",
        "A CSS border-radius rule.",
        border_radius,
    )
    .form()
    .fields(&[
        ("Top left", "12"),
        ("Top right", "12"),
        ("Bottom right", "12"),
        ("Bottom left", "12"),
    ])
    .selects(&[("Unit", &[("px", "px"), ("%", "%"), ("rem", "rem")])]),
    tool(
        "glass",
        "CSS glassmorphism",
        "CSS for a frosted glass panel.",
        glass,
    )
    .form()
    .fields(&[("Tint", "white"), ("Blur (px)", "12")])
    .slider("Opacity (%)", 0.0, 100.0, 20.0),
    tool(
        "css-transform",
        "CSS transform",
        "A CSS transform rule.",
        transform,
    )
    .form()
    .fields(&[
        ("Rotate (°)", "15"),
        ("Scale", "1.1"),
        ("Move X (px)", ""),
        ("Move Y (px)", ""),
        ("Skew X (°)", ""),
        ("Skew Y (°)", ""),
    ]),
    tool(
        "aspect-ratio",
        "Aspect ratio",
        "The ratio of a size, and sizes with the same ratio.",
        aspect_ratio,
    )
    .form()
    .fields(&[
        ("Width", "1920"),
        ("Height", "1080"),
        ("New width", "1280"),
        ("New height", ""),
    ]),
    tool(
        "font-size",
        "Font size converter",
        "px, rem, em, pt, % and vw.",
        font_size,
    )
    .form()
    .fields(&[
        ("Size", "18"),
        ("Base font size (px)", "16"),
        ("Viewport width (px)", "1440"),
    ])
    .selects(&[(
        "In",
        &[
            ("px", "px"),
            ("rem", "rem"),
            ("em", "em"),
            ("pt", "pt"),
            ("percent", "%"),
            ("vw", "vw"),
        ],
    )]),
];

pub const IMAGES: &[Tool] = &[
    tool(
        "qr",
        "QR code generator",
        "A QR code of any text or link.",
        qr,
    )
    .text("Text or link")
    .slider("Pixels per module", 2.0, 12.0, 6.0),
    tool(
        "wifi-qr",
        "Wi-Fi QR code",
        "A QR code that joins a Wi-Fi network.",
        wifi_qr,
    )
    .form()
    .fields(&[("Network name (SSID)", "MyNetwork"), ("Password", "")])
    .selects(&[(
        "Security",
        &[("WPA", "WPA/WPA2/WPA3"), ("WEP", "WEP"), ("nopass", "None")],
    )])
    .checks(&["Hidden network"]),
    tool(
        "barcode",
        "Barcode generator",
        "A Code 128 or EAN-13 barcode.",
        barcode,
    )
    .form()
    .fields(&[("Data", "HELLO-123")])
    .selects(&[("Symbology", &[("code128", "Code 128"), ("ean13", "EAN-13")])]),
    tool(
        "fractal",
        "Fractals",
        "Sierpiński, Koch, dragon, Hilbert, Peano, Gosper, Lévy, trees and Pythagoras.",
        fractal,
    )
    .form()
    .fields(&[("Color", "#1e3c8c")])
    .selects(&[(
        "Fractal",
        &[
            ("sierpinski", "Sierpiński triangle"),
            ("koch", "Koch snowflake"),
            ("dragon", "Dragon curve"),
            ("hilbert", "Hilbert curve"),
            ("peano", "Peano curve"),
            ("gosper", "Gosper curve"),
            ("levy", "Lévy C curve"),
            ("canopy", "Fractal canopy"),
            ("pythagoras", "Pythagoras tree"),
            ("plant", "Fractal plant"),
        ],
    )])
    .slider("Depth", 1.0, 12.0, 5.0),
    tool(
        "placeholder",
        "Placeholder image",
        "A placeholder image of any size.",
        placeholder,
    )
    .form()
    .fields(&[
        ("Width", "640"),
        ("Height", "360"),
        ("Background", "#cccccc"),
        ("Text color", "#555555"),
        ("Text (empty: the size)", ""),
    ]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors() {
        assert_eq!(parse_color("#f80").unwrap(), [255, 136, 0]);
        assert_eq!(parse_color("rgb(1, 2, 3)").unwrap(), [1, 2, 3]);
        assert_eq!(parse_color("hsl(0, 100%, 50%)").unwrap(), [255, 0, 0]);
        assert_eq!(parse_color("Rebecca Purple").unwrap(), [0x66, 0x33, 0x99]);
        assert_eq!(nearest_name([250, 128, 114]).0, "salmon");
        let ratio = (luminance([255, 255, 255]) + 0.05) / (luminance([0, 0, 0]) + 0.05);
        assert!((ratio - 21.0).abs() < 1e-9);
    }

    #[test]
    fn codes() {
        // The check digit of 400638133393 is 1.
        assert_eq!(ean13("400638133393").unwrap().1, "4006381333931");
        assert_eq!(ean13("4006381333931").unwrap().0.len(), 95);
        assert!(ean13("4006381333932").is_err());
        let w = code128("A").unwrap();
        assert_eq!(w.iter().map(|x| *x as usize).sum::<usize>(), 11 * 3 + 13);
    }
}
