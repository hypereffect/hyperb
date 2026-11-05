//! Unit converters. A unit's menu value is its size in the quantity's base
//! unit, so the menus are the conversion tables.

use super::Output;
use super::{Opts, Tool, fmt, table, tool};

/// Converts with a table whose values are factors to a base unit.
fn convert(units: super::Menu, o: &mut Opts) -> Result<Output, String> {
    let v = o.num(0)?;
    let factor = |id: &str| id.parse::<f64>().unwrap_or(1.0);
    let base = v * factor(o.mode);
    let label = |id: &str| units.iter().find(|u| u.0 == id).map_or("", |u| u.1);
    let digits = o.number.max(1);
    let rows: Vec<(&str, String)> = units
        .iter()
        .map(|(id, l)| (*l, fmt(base / factor(id), digits)))
        .collect();
    let from = format!("{} {}", fmt(v, 12), label(o.mode));
    Ok(table(&[(
        &from,
        format!(
            "{} {}",
            fmt(base / factor(o.choice(1)), digits),
            label(o.choice(1))
        ),
    )])
    .note("In every unit")
    .facts(&rows))
}

macro_rules! units {
    ($($fn:ident, $table:ident, $id:literal, $name:literal, $about:literal, $value:literal;)*) => {
        $(
            fn $fn(_: &str, o: &mut Opts) -> Result<Output, String> {
                convert($table, o)
            }
        )*
        pub const UNITS: &[Tool] = &[
            TEMPERATURE,
            FUEL_ECONOMY,
            SHOE_SIZE,
            RING_SIZE,
            $(
                tool($id, $name, $about, $fn)
                    .form()
                    .fields(&[("Value", $value)])
                    .selects(&[("From", $table), ("To", $table)])
                    .slider("Decimals", 1.0, 12.0, 6.0),
            )*
        ];
    };
}

const LENGTH: super::Menu = &[
    ("1", "Metres"),
    ("1000", "Kilometres"),
    ("0.01", "Centimetres"),
    ("0.001", "Millimetres"),
    ("0.000001", "Micrometres"),
    ("1e-9", "Nanometres"),
    ("0.0254", "Inches"),
    ("0.3048", "Feet"),
    ("0.9144", "Yards"),
    ("1609.344", "Miles"),
    ("1852", "Nautical miles"),
    ("20.1168", "Chains"),
    ("201.168", "Furlongs"),
    ("1e-10", "Ångströms"),
    ("0.000352778", "Typographic points"),
];
const WEIGHT: super::Menu = &[
    ("1", "Kilograms"),
    ("0.001", "Grams"),
    ("0.000001", "Milligrams"),
    ("1000", "Tonnes"),
    ("0.45359237", "Pounds"),
    ("0.028349523125", "Ounces"),
    ("6.35029318", "Stone"),
    ("907.18474", "Short tons (US)"),
    ("1016.0469088", "Long tons (UK)"),
    ("0.0002", "Carats"),
    ("0.0311034768", "Troy ounces"),
    ("0.00006479891", "Grains"),
];
const VOLUME: super::Menu = &[
    ("1", "Litres"),
    ("0.001", "Millilitres"),
    ("1000", "Cubic metres"),
    ("0.001000001", "Cubic centimetres"),
    ("3.785411784", "US gallons"),
    ("4.54609", "Imperial gallons"),
    ("0.946352946", "US quarts"),
    ("0.473176473", "US pints"),
    ("0.56826125", "Imperial pints"),
    ("0.2365882365", "US cups"),
    ("0.0295735295625", "US fluid ounces"),
    ("0.0284130625", "Imperial fluid ounces"),
    ("0.01478676478125", "Tablespoons"),
    ("0.00492892159375", "Teaspoons"),
    ("28.316846592", "Cubic feet"),
    ("0.016387064", "Cubic inches"),
    ("158.987294928", "Barrels (oil)"),
];
const AREA: super::Menu = &[
    ("1", "Square metres"),
    ("1000000", "Square kilometres"),
    ("0.0001", "Square centimetres"),
    ("10000", "Hectares"),
    ("4046.8564224", "Acres"),
    ("0.09290304", "Square feet"),
    ("0.00064516", "Square inches"),
    ("0.83612736", "Square yards"),
    ("2589988.110336", "Square miles"),
    ("100", "Ares"),
];
const SPEED: super::Menu = &[
    ("1", "Metres per second"),
    ("0.2777777777777778", "Kilometres per hour"),
    ("0.44704", "Miles per hour"),
    ("0.5144444444444445", "Knots"),
    ("0.3048", "Feet per second"),
    ("343", "Mach (at 20 °C)"),
    ("299792458", "Speed of light"),
];
const TIME: super::Menu = &[
    ("1", "Seconds"),
    ("0.001", "Milliseconds"),
    ("0.000001", "Microseconds"),
    ("1e-9", "Nanoseconds"),
    ("60", "Minutes"),
    ("3600", "Hours"),
    ("86400", "Days"),
    ("604800", "Weeks"),
    ("2629746", "Months (average)"),
    ("31556952", "Years (average)"),
    ("315569520", "Decades"),
    ("3155695200", "Centuries"),
];
const PRESSURE: super::Menu = &[
    ("1", "Pascals"),
    ("1000", "Kilopascals"),
    ("1000000", "Megapascals"),
    ("100000", "Bar"),
    ("100", "Millibar (hPa)"),
    ("101325", "Atmospheres"),
    ("6894.757293168", "PSI"),
    ("133.322387415", "mmHg (torr)"),
    ("3386.389", "Inches of mercury"),
    ("9.80665", "mm of water"),
];
const ENERGY: super::Menu = &[
    ("1", "Joules"),
    ("1000", "Kilojoules"),
    ("4.184", "Calories"),
    ("4184", "Kilocalories"),
    ("3600", "Watt-hours"),
    ("3600000", "Kilowatt-hours"),
    ("1055.05585262", "BTU"),
    ("1.602176634e-19", "Electronvolts"),
    ("1.3558179483314", "Foot-pounds"),
    ("105505585.262", "Therms"),
];
const POWER: super::Menu = &[
    ("1", "Watts"),
    ("1000", "Kilowatts"),
    ("1000000", "Megawatts"),
    ("745.69987158227", "Horsepower (mechanical)"),
    ("735.49875", "Horsepower (metric)"),
    ("0.29307107017222", "BTU per hour"),
    ("3516.8528420667", "Tons of refrigeration"),
    ("1.3558179483314", "Foot-pounds per second"),
];
const FORCE: super::Menu = &[
    ("1", "Newtons"),
    ("1000", "Kilonewtons"),
    ("0.00001", "Dynes"),
    ("9.80665", "Kilogram-force"),
    ("4.4482216152605", "Pound-force"),
    ("0.138254954376", "Poundals"),
];
const ANGLE: super::Menu = &[
    ("1", "Degrees"),
    ("57.29577951308232", "Radians"),
    ("0.9", "Gradians"),
    ("0.016666666666666666", "Arcminutes"),
    ("0.0002777777777777778", "Arcseconds"),
    ("360", "Turns"),
    ("0.05625", "Mils (NATO)"),
];
const STORAGE: super::Menu = &[
    ("1", "Bytes"),
    ("0.125", "Bits"),
    ("1000", "Kilobytes (kB)"),
    ("1024", "Kibibytes (KiB)"),
    ("1000000", "Megabytes (MB)"),
    ("1048576", "Mebibytes (MiB)"),
    ("1000000000", "Gigabytes (GB)"),
    ("1073741824", "Gibibytes (GiB)"),
    ("1000000000000", "Terabytes (TB)"),
    ("1099511627776", "Tebibytes (TiB)"),
    ("1e15", "Petabytes (PB)"),
    ("1125899906842624", "Pebibytes (PiB)"),
];
const TRANSFER: super::Menu = &[
    ("1", "Bits per second"),
    ("1000", "Kilobits per second"),
    ("1000000", "Megabits per second"),
    ("1000000000", "Gigabits per second"),
    ("8", "Bytes per second"),
    ("8000", "Kilobytes per second"),
    ("8000000", "Megabytes per second"),
    ("8000000000", "Gigabytes per second"),
    ("8388608", "Mebibytes per second"),
];
const FREQUENCY: super::Menu = &[
    ("1", "Hertz"),
    ("1000", "Kilohertz"),
    ("1000000", "Megahertz"),
    ("1000000000", "Gigahertz"),
    ("0.016666666666666666", "Per minute (rpm)"),
    ("0.15915494309189535", "Radians per second"),
];
const DENSITY: super::Menu = &[
    ("1", "kg/m³"),
    ("1000", "g/cm³ (g/ml)"),
    ("0.001", "g/L"),
    ("16.01846337", "lb/ft³"),
    ("27679.9047", "lb/in³"),
    ("119.8264273", "lb/US gal"),
];
const ACCELERATION: super::Menu = &[
    ("1", "m/s²"),
    ("9.80665", "g (standard gravity)"),
    ("0.3048", "ft/s²"),
    ("0.01", "Gal (cm/s²)"),
    ("0.0254", "in/s²"),
];
const AMOUNT: super::Menu = &[
    ("1", "Moles"),
    ("0.001", "Millimoles"),
    ("0.000001", "Micromoles"),
    ("1000", "Kilomoles"),
    ("1.6605390671738466e-24", "Molecules (count)"),
];
const CHARGE: super::Menu = &[
    ("1", "Coulombs"),
    ("0.001", "Millicoulombs"),
    ("3600", "Ampere-hours"),
    ("3.6", "Milliampere-hours"),
    ("1.602176634e-19", "Elementary charges"),
];
const CAPACITANCE: super::Menu = &[
    ("1", "Farads"),
    ("0.001", "Millifarads"),
    ("0.000001", "Microfarads"),
    ("1e-9", "Nanofarads"),
    ("1e-12", "Picofarads"),
];
const INDUCTANCE: super::Menu = &[
    ("1", "Henries"),
    ("0.001", "Millihenries"),
    ("0.000001", "Microhenries"),
    ("1e-9", "Nanohenries"),
];
const RESISTANCE: super::Menu = &[
    ("1", "Ohms"),
    ("1000", "Kilohms"),
    ("1000000", "Megohms"),
    ("0.001", "Milliohms"),
];
const ILLUMINANCE: super::Menu = &[
    ("1", "Lux"),
    ("10.763910417", "Foot-candles"),
    ("10000", "Phots"),
    ("0.001", "Millilux"),
];
const VISCOSITY: super::Menu = &[
    ("1", "Pascal-seconds"),
    ("0.001", "Centipoise (mPa·s)"),
    ("0.1", "Poise"),
    ("47.880258980336", "Pound-force seconds per ft²"),
];
const ASTRONOMICAL: super::Menu = &[
    ("1", "Kilometres"),
    ("149597870.7", "Astronomical units"),
    ("9460730472580.8", "Light years"),
    ("30856775814913.673", "Parsecs"),
    ("299792.458", "Light seconds"),
    ("17987547.48", "Light minutes"),
    ("1.609344", "Miles"),
    ("384400", "Earth–Moon distances"),
    ("6371", "Earth radii"),
];

units! {
    length, LENGTH, "length", "Length and distance", "Metres, feet, inches, miles and more.", "1";
    weight, WEIGHT, "weight", "Weight and mass", "Kilograms, pounds, ounces, stone and more.", "1";
    volume, VOLUME, "volume", "Volume", "Litres, gallons, cups, cubic feet and more.", "1";
    area, AREA, "area", "Area", "Square metres, acres, hectares, square feet and more.", "1";
    speed, SPEED, "speed", "Speed", "km/h, mph, knots, m/s and more.", "100";
    time, TIME, "time-units", "Time", "Seconds to centuries.", "1";
    pressure, PRESSURE, "pressure", "Pressure", "Pascals, bar, PSI, atmospheres, mmHg and more.", "1";
    energy, ENERGY, "energy", "Energy", "Joules, calories, kWh, BTU and more.", "1";
    power, POWER, "power", "Power", "Watts, horsepower, BTU per hour and more.", "1";
    force, FORCE, "force", "Force", "Newtons, pound-force, kilogram-force and more.", "1";
    angle, ANGLE, "angle", "Angle", "Degrees, radians, gradians, arcminutes and more.", "180";
    storage, STORAGE, "data-storage", "Data storage", "Bytes, KB, KiB, MB, GB, TB and more.", "1";
    transfer, TRANSFER, "data-transfer", "Data transfer rate", "Mbps, MB/s, Gbps and more.", "100";
    frequency, FREQUENCY, "frequency", "Frequency", "Hertz, kHz, MHz, GHz and rpm.", "1";
    density, DENSITY, "density", "Density", "kg/m³, g/cm³, lb/ft³ and more.", "1";
    acceleration, ACCELERATION, "acceleration", "Acceleration", "m/s², g and ft/s².", "1";
    amount, AMOUNT, "amount", "Amount of substance", "Moles and molecules.", "1";
    charge, CHARGE, "charge", "Electric charge", "Coulombs and ampere-hours.", "1";
    capacitance, CAPACITANCE, "capacitance", "Capacitance", "Farads down to picofarads.", "1";
    inductance, INDUCTANCE, "inductance", "Inductance", "Henries down to nanohenries.", "1";
    resistance, RESISTANCE, "resistance", "Electrical resistance", "Ohms, kilohms and megohms.", "1";
    illuminance, ILLUMINANCE, "illuminance", "Illuminance", "Lux and foot-candles.", "1";
    viscosity, VISCOSITY, "viscosity", "Viscosity", "Pascal-seconds, poise and centipoise.", "1";
    astronomical, ASTRONOMICAL, "astronomical", "Astronomical distances", "Astronomical units, light years and parsecs.", "1";
}

fn temperature(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.num(0)?;
    let k = match o.mode {
        "f" => (v - 32.0) / 1.8 + 273.15,
        "k" => v,
        "r" => v / 1.8,
        _ => v + 273.15,
    };
    if k < 0.0 {
        return Err("That is below absolute zero.".into());
    }
    Ok(table(&[
        ("Celsius", format!("{} °C", fmt(k - 273.15, 4))),
        (
            "Fahrenheit",
            format!("{} °F", fmt((k - 273.15) * 1.8 + 32.0, 4)),
        ),
        ("Kelvin", format!("{} K", fmt(k, 4))),
        ("Rankine", format!("{} °R", fmt(k * 1.8, 4))),
    ]))
}

fn fuel_economy(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.positive(0)?;
    // In kilometres per litre.
    let kpl = match o.mode {
        "l100" => 100.0 / v,
        "mpg-us" => v * 1.609344 / 3.785411784,
        "mpg-uk" => v * 1.609344 / 4.54609,
        _ => v,
    };
    Ok(table(&[
        ("L/100 km", fmt(100.0 / kpl, 3)),
        ("km/L", fmt(kpl, 3)),
        ("MPG (US)", fmt(kpl * 3.785411784 / 1.609344, 3)),
        ("MPG (UK)", fmt(kpl * 4.54609 / 1.609344, 3)),
    ]))
}

fn shoe_size(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.positive(0)?;
    // From the foot length in cm: the last is 1.5 cm longer; EU sizes are
    // Paris points (2/3 cm) of the last; UK sizes barleycorns (1/3 in).
    let foot = match o.mode {
        "eu" => v / 1.5 - 1.5,
        "uk" => (v + 23.0) / 3.0 * 2.54 - 1.5,
        "us-m" => (v - 1.0 + 23.0) / 3.0 * 2.54 - 1.5,
        "us-w" => (v - 2.0 + 23.0) / 3.0 * 2.54 - 1.5,
        _ => v,
    };
    let last = foot + 1.5;
    let uk = last / 2.54 * 3.0 - 23.0;
    let half = |x: f64| fmt((x * 2.0).round() / 2.0, 1);
    Ok(table(&[
        ("Foot length", format!("{} cm", fmt(foot, 1))),
        ("EU", half(last * 1.5)),
        ("UK", half(uk)),
        ("US men", half(uk + 1.0)),
        ("US women", half(uk + 2.0)),
        (
            "",
            "Sizing differs between brands; try shoes on when you can.".into(),
        ),
    ]))
}

fn ring_size(_: &str, o: &mut Opts) -> Result<Output, String> {
    let v = o.positive(0)?;
    let d = match o.mode {
        "us" => 11.63 + 0.8128 * v,
        "eu" => v / std::f64::consts::PI,
        _ => v,
    };
    let us = (d - 11.63) / 0.8128;
    // UK letters run A to Z in steps of about half a US size, from A ≈ US ½.
    let uk_index = ((us - 0.5) * 2.0).round();
    let uk = if (0.0..26.0).contains(&uk_index) {
        ((b'A' + uk_index as u8) as char).to_string()
    } else {
        "—".into()
    };
    Ok(table(&[
        ("Inside diameter", format!("{} mm", fmt(d, 2))),
        (
            "Circumference (EU)",
            format!("{} mm", fmt(d * std::f64::consts::PI, 1)),
        ),
        ("US and Canada", fmt((us * 4.0).round() / 4.0, 2)),
        ("UK and Australia", uk),
    ]))
}

const TEMPERATURE: Tool = tool(
    "temperature",
    "Temperature",
    "Celsius, Fahrenheit, Kelvin and Rankine.",
    temperature,
)
.form()
.fields(&[("Temperature", "100")])
.selects(&[(
    "In",
    &[
        ("c", "Celsius"),
        ("f", "Fahrenheit"),
        ("k", "Kelvin"),
        ("r", "Rankine"),
    ],
)]);
const FUEL_ECONOMY: Tool = tool(
    "fuel-economy",
    "Fuel economy",
    "L/100 km, km/L and miles per gallon.",
    fuel_economy,
)
.form()
.fields(&[("Value", "6.5")])
.selects(&[(
    "In",
    &[
        ("l100", "L/100 km"),
        ("kpl", "km/L"),
        ("mpg-us", "MPG (US)"),
        ("mpg-uk", "MPG (UK)"),
    ],
)]);
const SHOE_SIZE: Tool = tool(
    "shoe-size",
    "Shoe size converter",
    "EU, UK and US shoe sizes and foot length.",
    shoe_size,
)
.form()
.fields(&[("Size", "42")])
.selects(&[(
    "In",
    &[
        ("eu", "EU"),
        ("uk", "UK"),
        ("us-m", "US men"),
        ("us-w", "US women"),
        ("cm", "Foot length (cm)"),
    ],
)]);
const RING_SIZE: Tool = tool(
    "ring-size",
    "Ring size converter",
    "US, UK and EU ring sizes and diameters.",
    ring_size,
)
.form()
.fields(&[("Size", "7")])
.selects(&[(
    "In",
    &[
        ("us", "US size"),
        ("eu", "EU (circumference, mm)"),
        ("mm", "Diameter (mm)"),
    ],
)]);

#[cfg(test)]
mod tests {
    use super::super::Rng;
    use super::*;

    #[test]
    fn converts() {
        let mut rng = Rng(1);
        let mut o = Opts::new("1609.344", &mut rng).with(&["1"], &["1609.344", "1000"], &[]);
        o.number = 6;
        assert!(
            length("", &mut o)
                .unwrap()
                .plain()
                .starts_with("1 Miles: 1.609344 Kilometres")
        );
        let mut o = Opts::new("f", &mut rng).with(&["212"], &["f"], &[]);
        assert!(temperature("", &mut o).unwrap().plain().contains("100 °C"));
        // Menu values parse as factors, and are unique.
        for t in UNITS {
            for (_, menu) in t.selects {
                let mut ids: Vec<&str> = menu.iter().map(|m| m.0).collect();
                ids.sort();
                ids.dedup();
                assert_eq!(ids.len(), menu.len(), "{}", t.id);
            }
        }
    }
}
