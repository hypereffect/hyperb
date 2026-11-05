//! Developer references and small tools for the web, network and Unix:
//! status codes, MIME types, validators, meta tags, CIDR maths, permissions
//! and the lookup tables that are handy next to them.

use super::Output;
use super::{Opts, Tool, table, tool, verdict};
use base64::Engine;
use regex_lite::Regex;
use std::net::{IpAddr, Ipv4Addr};

const HTTP_STATUS: &[(u16, &str, &str)] = &[
    (
        100,
        "Continue",
        "The request headers were received; send the body.",
    ),
    (
        101,
        "Switching Protocols",
        "The server agrees to change protocols.",
    ),
    (200, "OK", "The request succeeded."),
    (201, "Created", "A new resource was created."),
    (
        202,
        "Accepted",
        "The request was accepted for later processing.",
    ),
    (
        204,
        "No Content",
        "The request succeeded and there is no body.",
    ),
    (
        206,
        "Partial Content",
        "Part of the resource, as a range request asked.",
    ),
    (
        301,
        "Moved Permanently",
        "The resource has a new permanent address.",
    ),
    (302, "Found", "A temporary redirect."),
    (303, "See Other", "Look at another URL, usually with GET."),
    (
        304,
        "Not Modified",
        "The client's cached copy is still current.",
    ),
    (
        307,
        "Temporary Redirect",
        "A temporary redirect that keeps the method.",
    ),
    (
        308,
        "Permanent Redirect",
        "A permanent redirect that keeps the method.",
    ),
    (400, "Bad Request", "The request is malformed."),
    (401, "Unauthorized", "Authentication is required."),
    (403, "Forbidden", "The server refuses to show the resource."),
    (404, "Not Found", "Nothing lives at that address."),
    (
        405,
        "Method Not Allowed",
        "The URL exists, but not for that method.",
    ),
    (408, "Request Timeout", "The client took too long."),
    (
        409,
        "Conflict",
        "The request clashes with the current state.",
    ),
    (410, "Gone", "The resource existed and is gone for good."),
    (
        413,
        "Content Too Large",
        "The body is larger than the server accepts.",
    ),
    (
        415,
        "Unsupported Media Type",
        "The server does not handle that format.",
    ),
    (418, "I'm a Teapot", "The server is, in fact, a teapot."),
    (
        422,
        "Unprocessable Content",
        "The syntax is fine but the meaning is not.",
    ),
    (
        429,
        "Too Many Requests",
        "The client is being rate limited.",
    ),
    (
        500,
        "Internal Server Error",
        "Something went wrong on the server.",
    ),
    (
        501,
        "Not Implemented",
        "The server does not implement that feature.",
    ),
    (
        502,
        "Bad Gateway",
        "An upstream server sent an invalid answer.",
    ),
    (
        503,
        "Service Unavailable",
        "The server is overloaded or down for now.",
    ),
    (
        504,
        "Gateway Timeout",
        "An upstream server did not answer in time.",
    ),
];

const MIME_TYPES: &[(&str, &str, &str)] = &[
    ("aac", "audio/aac", "audio"),
    ("avi", "video/x-msvideo", "video"),
    ("avif", "image/avif", "image"),
    ("bin", "application/octet-stream", "binary"),
    ("bmp", "image/bmp", "image"),
    ("bz2", "application/x-bzip2", "archive"),
    ("css", "text/css", "text"),
    ("csv", "text/csv", "text"),
    ("doc", "application/msword", "document"),
    (
        "docx",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "document",
    ),
    ("gif", "image/gif", "image"),
    ("gz", "application/gzip", "archive"),
    ("htm", "text/html", "text"),
    ("html", "text/html", "text"),
    ("ico", "image/x-icon", "image"),
    ("ics", "text/calendar", "text"),
    ("jpeg", "image/jpeg", "image"),
    ("jpg", "image/jpeg", "image"),
    ("js", "text/javascript", "text"),
    ("json", "application/json", "data"),
    ("jsonld", "application/ld+json", "data"),
    ("md", "text/markdown", "text"),
    ("mjs", "text/javascript", "text"),
    ("mp3", "audio/mpeg", "audio"),
    ("mp4", "video/mp4", "video"),
    ("mpeg", "video/mpeg", "video"),
    (
        "odp",
        "application/vnd.oasis.opendocument.presentation",
        "document",
    ),
    (
        "ods",
        "application/vnd.oasis.opendocument.spreadsheet",
        "document",
    ),
    ("odt", "application/vnd.oasis.opendocument.text", "document"),
    ("oga", "audio/ogg", "audio"),
    ("ogg", "audio/ogg", "audio"),
    ("ogv", "video/ogg", "video"),
    ("otf", "font/otf", "font"),
    ("pdf", "application/pdf", "document"),
    ("png", "image/png", "image"),
    ("ppt", "application/vnd.ms-powerpoint", "document"),
    (
        "pptx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "document",
    ),
    ("rar", "application/vnd.rar", "archive"),
    ("rtf", "application/rtf", "document"),
    ("svg", "image/svg+xml", "image"),
    ("tar", "application/x-tar", "archive"),
    ("tif", "image/tiff", "image"),
    ("tiff", "image/tiff", "image"),
    ("ts", "video/mp2t", "video"),
    ("ttf", "font/ttf", "font"),
    ("txt", "text/plain", "text"),
    ("wav", "audio/wav", "audio"),
    ("webm", "video/webm", "video"),
    ("webp", "image/webp", "image"),
    ("woff", "font/woff", "font"),
    ("woff2", "font/woff2", "font"),
    ("xls", "application/vnd.ms-excel", "document"),
    (
        "xlsx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "document",
    ),
    ("xml", "application/xml", "data"),
    ("zip", "application/zip", "archive"),
];

const PORTS: &[(u16, &str, &str)] = &[
    (20, "ftp-data", "FTP data"),
    (21, "ftp", "FTP control"),
    (22, "ssh", "Secure shell"),
    (23, "telnet", "Telnet"),
    (25, "smtp", "Mail transfer"),
    (53, "domain", "DNS"),
    (67, "dhcp-server", "DHCP server"),
    (68, "dhcp-client", "DHCP client"),
    (69, "tftp", "Trivial file transfer"),
    (80, "http", "World Wide Web"),
    (110, "pop3", "Post office protocol"),
    (123, "ntp", "Network time"),
    (143, "imap", "Internet message access"),
    (161, "snmp", "Simple network management"),
    (194, "irc", "Internet relay chat"),
    (389, "ldap", "Directory access"),
    (443, "https", "HTTP over TLS"),
    (445, "microsoft-ds", "SMB over TCP"),
    (465, "submissions", "SMTP over TLS"),
    (514, "syslog", "System logging"),
    (587, "submission", "Mail submission"),
    (631, "ipp", "Internet printing"),
    (636, "ldaps", "Directory access over TLS"),
    (993, "imaps", "IMAP over TLS"),
    (995, "pop3s", "POP3 over TLS"),
    (1433, "ms-sql-s", "Microsoft SQL Server"),
    (1521, "oracle", "Oracle database"),
    (1883, "mqtt", "Message queuing telemetry"),
    (2049, "nfs", "Network file system"),
    (2375, "docker", "Docker daemon"),
    (3306, "mysql", "MySQL database"),
    (3389, "ms-wbt-server", "Remote desktop"),
    (5432, "postgresql", "PostgreSQL database"),
    (5672, "amqp", "Advanced message queuing"),
    (6379, "redis", "Redis"),
    (8080, "http-alt", "Alternate HTTP"),
    (8443, "https-alt", "Alternate HTTPS"),
    (9000, "cslistener", "Often PHP-FPM or an app server"),
    (9200, "elasticsearch", "Elasticsearch"),
    (11211, "memcache", "Memcached"),
    (27017, "mongodb", "MongoDB"),
];

const HTML_ENTITIES: &[(&str, &str, u32)] = &[
    ("amp", "&amp;", 38),
    ("lt", "&lt;", 60),
    ("gt", "&gt;", 62),
    ("quot", "&quot;", 34),
    ("apos", "&apos;", 39),
    ("nbsp", "&nbsp;", 160),
    ("copy", "&copy;", 169),
    ("reg", "&reg;", 174),
    ("trade", "&trade;", 8482),
    ("euro", "&euro;", 8364),
    ("pound", "&pound;", 163),
    ("yen", "&yen;", 165),
    ("cent", "&cent;", 162),
    ("sect", "&sect;", 167),
    ("deg", "&deg;", 176),
    ("plusmn", "&plusmn;", 177),
    ("frac14", "&frac14;", 188),
    ("frac12", "&frac12;", 189),
    ("frac34", "&frac34;", 190),
    ("times", "&times;", 215),
    ("divide", "&divide;", 247),
    ("micro", "&micro;", 181),
    ("para", "&para;", 182),
    ("middot", "&middot;", 183),
    ("bull", "&bull;", 8226),
    ("hellip", "&hellip;", 8230),
    ("ndash", "&ndash;", 8211),
    ("mdash", "&mdash;", 8212),
    ("lsquo", "&lsquo;", 8216),
    ("rsquo", "&rsquo;", 8217),
    ("ldquo", "&ldquo;", 8220),
    ("rdquo", "&rdquo;", 8221),
    ("laquo", "&laquo;", 171),
    ("raquo", "&raquo;", 187),
    ("larr", "&larr;", 8592),
    ("rarr", "&rarr;", 8594),
    ("uarr", "&uarr;", 8593),
    ("darr", "&darr;", 8595),
    ("hearts", "&hearts;", 9829),
    ("check", "&check;", 10003),
];

const COUNTRY_CODES: &[(&str, &str, &str, &str)] = &[
    ("Argentina", "AR", "ARG", "+54"),
    ("Australia", "AU", "AUS", "+61"),
    ("Austria", "AT", "AUT", "+43"),
    ("Belgium", "BE", "BEL", "+32"),
    ("Brazil", "BR", "BRA", "+55"),
    ("Canada", "CA", "CAN", "+1"),
    ("Chile", "CL", "CHL", "+56"),
    ("China", "CN", "CHN", "+86"),
    ("Colombia", "CO", "COL", "+57"),
    ("Czechia", "CZ", "CZE", "+420"),
    ("Denmark", "DK", "DNK", "+45"),
    ("Egypt", "EG", "EGY", "+20"),
    ("Finland", "FI", "FIN", "+358"),
    ("France", "FR", "FRA", "+33"),
    ("Germany", "DE", "DEU", "+49"),
    ("Greece", "GR", "GRC", "+30"),
    ("India", "IN", "IND", "+91"),
    ("Indonesia", "ID", "IDN", "+62"),
    ("Ireland", "IE", "IRL", "+353"),
    ("Israel", "IL", "ISR", "+972"),
    ("Italy", "IT", "ITA", "+39"),
    ("Japan", "JP", "JPN", "+81"),
    ("Kenya", "KE", "KEN", "+254"),
    ("Mexico", "MX", "MEX", "+52"),
    ("Netherlands", "NL", "NLD", "+31"),
    ("New Zealand", "NZ", "NZL", "+64"),
    ("Nigeria", "NG", "NGA", "+234"),
    ("Norway", "NO", "NOR", "+47"),
    ("Pakistan", "PK", "PAK", "+92"),
    ("Peru", "PE", "PER", "+51"),
    ("Philippines", "PH", "PHL", "+63"),
    ("Poland", "PL", "POL", "+48"),
    ("Portugal", "PT", "PRT", "+351"),
    ("Romania", "RO", "ROU", "+40"),
    ("Saudi Arabia", "SA", "SAU", "+966"),
    ("Singapore", "SG", "SGP", "+65"),
    ("South Africa", "ZA", "ZAF", "+27"),
    ("South Korea", "KR", "KOR", "+82"),
    ("Spain", "ES", "ESP", "+34"),
    ("Sweden", "SE", "SWE", "+46"),
    ("Switzerland", "CH", "CHE", "+41"),
    ("Thailand", "TH", "THA", "+66"),
    ("Turkey", "TR", "TUR", "+90"),
    ("Ukraine", "UA", "UKR", "+380"),
    ("United Kingdom", "GB", "GBR", "+44"),
    ("United States", "US", "USA", "+1"),
];

const CURRENCIES: &[(&str, &str, &str)] = &[
    ("AED", "UAE dirham", "United Arab Emirates"),
    ("ARS", "Argentine peso", "Argentina"),
    ("AUD", "Australian dollar", "Australia"),
    ("BRL", "Brazilian real", "Brazil"),
    ("CAD", "Canadian dollar", "Canada"),
    ("CHF", "Swiss franc", "Switzerland"),
    ("CLP", "Chilean peso", "Chile"),
    ("CNY", "Chinese yuan", "China"),
    ("COP", "Colombian peso", "Colombia"),
    ("CZK", "Czech koruna", "Czechia"),
    ("DKK", "Danish krone", "Denmark"),
    ("EGP", "Egyptian pound", "Egypt"),
    ("EUR", "Euro", "Euro area"),
    ("GBP", "Pound sterling", "United Kingdom"),
    ("HKD", "Hong Kong dollar", "Hong Kong"),
    ("HUF", "Hungarian forint", "Hungary"),
    ("IDR", "Indonesian rupiah", "Indonesia"),
    ("ILS", "Israeli new shekel", "Israel"),
    ("INR", "Indian rupee", "India"),
    ("JPY", "Japanese yen", "Japan"),
    ("KES", "Kenyan shilling", "Kenya"),
    ("KRW", "South Korean won", "South Korea"),
    ("MXN", "Mexican peso", "Mexico"),
    ("MYR", "Malaysian ringgit", "Malaysia"),
    ("NGN", "Nigerian naira", "Nigeria"),
    ("NOK", "Norwegian krone", "Norway"),
    ("NZD", "New Zealand dollar", "New Zealand"),
    ("PEN", "Peruvian sol", "Peru"),
    ("PHP", "Philippine peso", "Philippines"),
    ("PKR", "Pakistani rupee", "Pakistan"),
    ("PLN", "Polish zloty", "Poland"),
    ("RON", "Romanian leu", "Romania"),
    ("RUB", "Russian ruble", "Russia"),
    ("SAR", "Saudi riyal", "Saudi Arabia"),
    ("SEK", "Swedish krona", "Sweden"),
    ("SGD", "Singapore dollar", "Singapore"),
    ("THB", "Thai baht", "Thailand"),
    ("TRY", "Turkish lira", "Turkey"),
    ("UAH", "Ukrainian hryvnia", "Ukraine"),
    ("USD", "United States dollar", "United States"),
    ("ZAR", "South African rand", "South Africa"),
];

const FILE_SIGNATURES: &[(&str, &str, &str)] = &[
    ("bmp", "42 4D", "Bitmap image"),
    ("class", "CA FE BA BE", "Java class"),
    ("doc", "D0 CF 11 E0 A1 B1 1A E1", "OLE compound document"),
    ("docx", "50 4B 03 04", "ZIP container (Office Open XML)"),
    ("gif", "47 49 46 38", "GIF image"),
    ("gz", "1F 8B", "Gzip archive"),
    ("ico", "00 00 01 00", "Windows icon"),
    ("jpeg", "FF D8 FF", "JPEG image"),
    ("jpg", "FF D8 FF", "JPEG image"),
    ("mkv", "1A 45 DF A3", "Matroska container"),
    ("mp3", "49 44 33", "MP3 with an ID3 tag"),
    (
        "mp4",
        "00 00 00 18 66 74 79 70",
        "ISO base media (often MP4)",
    ),
    ("ogg", "4F 67 67 53", "Ogg container"),
    ("pdf", "25 50 44 46 2D", "%PDF-"),
    ("png", "89 50 4E 47 0D 0A 1A 0A", "PNG image"),
    ("rar", "52 61 72 21 1A 07", "RAR archive"),
    (
        "sqlite",
        "53 51 4C 69 74 65 20 66 6F 72 6D 61 74 20 33 00",
        "SQLite database",
    ),
    ("tar", "75 73 74 61 72", "\"ustar\" at offset 257"),
    ("webp", "52 49 46 46", "RIFF container (WebP)"),
    ("xlsx", "50 4B 03 04", "ZIP container (Office Open XML)"),
    ("zip", "50 4B 03 04", "ZIP archive"),
];

const HTTP_METHODS: &[(&str, &str)] = &[
    ("GET", "Read a resource; safe and repeatable."),
    ("HEAD", "Like GET, but only the headers."),
    ("POST", "Send data, usually to create something."),
    ("PUT", "Replace a resource, or create it at that URL."),
    ("PATCH", "Change part of a resource."),
    ("DELETE", "Remove a resource."),
    ("OPTIONS", "Ask which methods a resource allows."),
    ("TRACE", "Echo the request back; rarely enabled."),
    ("CONNECT", "Open a tunnel, usually for TLS proxies."),
];

const HTTP_HEADERS: &[(&str, &str)] = &[
    ("Accept", "Media types the client can read."),
    ("Accept-Encoding", "Compression the client can read."),
    ("Accept-Language", "Languages the client prefers."),
    ("Authorization", "Credentials for the request."),
    ("Cache-Control", "How caches may keep the response."),
    ("Content-Encoding", "Compression used for the body."),
    ("Content-Length", "Size of the body, in bytes."),
    ("Content-Type", "Media type of the body."),
    ("Cookie", "Cookies the client sends back."),
    ("ETag", "A version tag for a cached resource."),
    ("Host", "The site the client asked for."),
    (
        "If-Modified-Since",
        "Only send it if it changed after this date.",
    ),
    (
        "If-None-Match",
        "Only send it if the ETag no longer matches.",
    ),
    ("Last-Modified", "When the resource last changed."),
    ("Location", "Where a redirect or a new resource lives."),
    ("Referer", "The page that linked to the request."),
    ("Set-Cookie", "A cookie for the client to keep."),
    ("User-Agent", "The client's name and version."),
    ("Vary", "Which request headers change the response."),
    ("X-Forwarded-For", "The client address, through a proxy."),
];

const TIMEZONES: &[&str] = &[
    "Africa/Cairo",
    "Africa/Johannesburg",
    "Africa/Lagos",
    "Africa/Nairobi",
    "America/Anchorage",
    "America/Bogota",
    "America/Chicago",
    "America/Denver",
    "America/Los_Angeles",
    "America/Mexico_City",
    "America/New_York",
    "America/Phoenix",
    "America/Sao_Paulo",
    "America/Toronto",
    "Asia/Bangkok",
    "Asia/Dubai",
    "Asia/Hong_Kong",
    "Asia/Jakarta",
    "Asia/Jerusalem",
    "Asia/Kolkata",
    "Asia/Seoul",
    "Asia/Shanghai",
    "Asia/Singapore",
    "Asia/Tokyo",
    "Australia/Adelaide",
    "Australia/Brisbane",
    "Australia/Melbourne",
    "Australia/Perth",
    "Australia/Sydney",
    "Europe/Amsterdam",
    "Europe/Athens",
    "Europe/Berlin",
    "Europe/Brussels",
    "Europe/Dublin",
    "Europe/Helsinki",
    "Europe/Istanbul",
    "Europe/Lisbon",
    "Europe/London",
    "Europe/Madrid",
    "Europe/Moscow",
    "Europe/Paris",
    "Europe/Prague",
    "Europe/Rome",
    "Europe/Stockholm",
    "Europe/Vienna",
    "Europe/Warsaw",
    "Europe/Zurich",
    "Pacific/Auckland",
    "Pacific/Honolulu",
    "UTC",
];

fn status(code: u16) -> Option<&'static (u16, &'static str, &'static str)> {
    HTTP_STATUS.iter().find(|s| s.0 == code)
}

fn mime(key: &str) -> Option<&'static (&'static str, &'static str, &'static str)> {
    let key = key.trim().trim_start_matches('.').to_ascii_lowercase();
    MIME_TYPES
        .iter()
        .find(|m| m.0.eq_ignore_ascii_case(&key) || m.1.eq_ignore_ascii_case(&key))
}

fn entity(key: &str) -> Option<&'static (&'static str, &'static str, u32)> {
    let key = key.trim().trim_start_matches('&').trim_end_matches(';');
    HTML_ENTITIES
        .iter()
        .find(|e| e.0.eq_ignore_ascii_case(key) || e.2.to_string() == key)
}

fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn http_status(_: &str, o: &mut Opts) -> Result<Output, String> {
    let code = o.int(0)?;
    let code =
        u16::try_from(code).map_err(|_| "A status code is between 100 and 599.".to_owned())?;
    let (code, reason, about) =
        status(code).ok_or_else(|| format!("{code} is not a standard HTTP status code."))?;
    Ok(table(&[
        ("Code", code.to_string()),
        ("Reason", (*reason).to_owned()),
        ("About", (*about).to_owned()),
        ("Class", format!("{}xx", code / 100)),
    ]))
}

fn mime_type(_: &str, o: &mut Opts) -> Result<Output, String> {
    let key = o.field(0);
    let (ext, mime, kind) = mime(key).ok_or_else(|| {
        format!(
            "“{key}” is not in the table; try an extension such as png or a type such as image/png."
        )
    })?;
    Ok(table(&[
        ("Extension", format!(".{ext}")),
        ("Media type", (*mime).to_owned()),
        ("Kind", (*kind).to_owned()),
        (
            "Text",
            if mime.starts_with("text/") || mime.ends_with("json") || mime.ends_with("xml") {
                "yes".to_owned()
            } else {
                "bytes".to_owned()
            },
        ),
    ]))
}

fn jwt(text: &str, _: &mut Opts) -> Result<Output, String> {
    let mut parts = text.trim().split('.');
    let (Some(header), Some(payload), Some(signature)) = (parts.next(), parts.next(), parts.next())
    else {
        return Err("A JWT has three dot-separated parts.".into());
    };
    if parts.next().is_some() {
        return Err("A JWT has exactly three parts.".into());
    }
    let decode = |part: &str| {
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(part)
            .map(|v| String::from_utf8_lossy(&v).into_owned())
            .map_err(|_| "A part is not valid base64url.".to_owned())
    };
    // Pretty JSON when a part is JSON, as it should be.
    let pretty = |s: String| match hypereffect::serde_json::from_str::<hypereffect::Value>(&s) {
        Ok(v) => super::code::json_text(&v, 2),
        Err(_) => s,
    };
    Ok(Output::default()
        .note("Header")
        .code(pretty(decode(header)?))
        .note("Payload")
        .code(pretty(decode(payload)?))
        .facts(&[(
            "Signature",
            format!("{signature} ({} bytes)", signature.len() * 3 / 4),
        )]))
}

fn uuid(_: &str, o: &mut Opts) -> Result<Output, String> {
    let n = o.number.max(1);
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let mut b = [0u8; 16];
        for chunk in b.chunks_mut(8) {
            let v = o.rng.next().to_le_bytes();
            chunk.copy_from_slice(&v[..chunk.len()]);
        }
        b[6] = (b[6] & 0x0f) | 0x40;
        b[8] = (b[8] & 0x3f) | 0x80;
        let s = format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            b[0],
            b[1],
            b[2],
            b[3],
            b[4],
            b[5],
            b[6],
            b[7],
            b[8],
            b[9],
            b[10],
            b[11],
            b[12],
            b[13],
            b[14],
            b[15]
        );
        out.push(if o.flag { s.to_uppercase() } else { s });
    }
    Ok(if n == 1 {
        table(&[("UUID", out.remove(0))])
    } else {
        super::list(out)
    })
}

fn password(_: &str, o: &mut Opts) -> Result<Output, String> {
    let len = o.number.max(1);
    let mut chars: Vec<u8> = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ".to_vec();
    if o.check(0) {
        chars.extend_from_slice(b"0123456789");
    }
    if o.check(1) {
        chars.extend_from_slice(b"!@#$%^&*()-_=+[]{};:,.?/");
    }
    if let Some(exclude) = o.field(0).strip_prefix('-') {
        let exclude = exclude.as_bytes();
        chars.retain(|c| !exclude.contains(c));
    }
    if chars.is_empty() {
        return Err("There are no characters left to use.".into());
    }
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        out.push(chars[o.rng.below(chars.len())] as char);
    }
    Ok(table(&[("Password", out)]))
}

fn cron(text: &str, _: &mut Opts) -> Result<Output, String> {
    let fields: Vec<&str> = text.split_whitespace().collect();
    if fields.len() != 5 {
        return Err("A cron expression has five fields: minute hour day month weekday.".into());
    }
    let names = ["Minute", "Hour", "Day of month", "Month", "Day of week"];
    let ranges = [(0, 59), (0, 23), (1, 31), (1, 12), (0, 6)];
    let mut rows = Vec::new();
    for (i, (field, (lo, hi))) in fields.iter().zip(ranges).enumerate() {
        if *field != "*" {
            for part in field.split(',') {
                for n in part.split('-') {
                    let n = n.trim();
                    if !n.is_empty() && n != "*/1" {
                        let v: i32 = n.parse().map_err(|_| {
                            format!(
                                "“{n}” is not a number in the {} field.",
                                names[i].to_lowercase()
                            )
                        })?;
                        if v < lo || v > hi {
                            return Err(format!(
                                "The {} field goes from {lo} to {hi}.",
                                names[i].to_lowercase()
                            ));
                        }
                    }
                }
            }
        }
        rows.push((names[i], (*field).to_owned()));
    }
    Ok(table(&[("Expression", fields[..5].join(" "))]).facts(&rows))
}

fn regex_test(text: &str, o: &mut Opts) -> Result<Output, String> {
    let pattern = o.first;
    let re = Regex::new(pattern).map_err(|e| format!("Not a regular expression: {e}"))?;
    let found: Vec<Vec<String>> = re
        .find_iter(text)
        .map(|m| vec![m.as_str().to_owned(), m.start().to_string()])
        .collect();
    if found.is_empty() {
        return Ok(verdict(false, "No matches."));
    }
    let n = found.len();
    Ok(verdict(
        true,
        format!("{n} match{}.", if n == 1 { "" } else { "es" }),
    )
    .table(&["Match", "At"], found))
}

fn curl(text: &str, _: &mut Opts) -> Result<Output, String> {
    let mut method = "GET";
    let mut url = None;
    let mut headers = Vec::new();
    let mut body: Option<&str> = None;
    let mut args = text.split_whitespace().peekable();
    while let Some(arg) = args.next() {
        match arg {
            "-X" | "--request" => method = args.next().ok_or("Missing the method after -X.")?,
            "-H" | "--header" => headers.push(args.next().ok_or("Missing a header after -H.")?),
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                body = Some(args.next().ok_or("Missing data after -d.")?);
                if method == "GET" {
                    method = "POST";
                }
            }
            "-I" | "--head" => method = "HEAD",
            "-L" | "--location" | "-s" | "--silent" | "-k" | "--insecure" => {}
            s if s.starts_with("http://") || s.starts_with("https://") => url = Some(s),
            _ => {}
        }
    }
    let url = url.ok_or("No URL in the command.")?;
    let header = if headers.is_empty() {
        String::new()
    } else {
        format!(
            ",\n  headers: {{{}}}",
            headers
                .iter()
                .map(|h| format!("{:?}", h))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    Ok(super::snippet(format!(
        "fetch({url:?}, {{\n  method: {method:?}{header}{}\n}});",
        body.map_or(String::new(), |b| format!(",\n  body: {b:?}"))
    )))
}

fn user_agent(text: &str, _: &mut Opts) -> Result<Output, String> {
    let ua = text.trim();
    if ua.is_empty() {
        return Err("Paste a user agent string.".into());
    }
    let browser = [
        ("Edg/", "Microsoft Edge"),
        ("OPR/", "Opera"),
        ("Chrome/", "Chrome"),
        ("Firefox/", "Firefox"),
        ("Safari/", "Safari"),
        ("curl/", "curl"),
        ("python-requests/", "Python requests"),
    ]
    .iter()
    .find(|(needle, _)| ua.contains(needle))
    .map_or("unknown", |(_, name)| *name);
    let os = if ua.contains("Windows") {
        "Windows"
    } else if ua.contains("Android") {
        "Android"
    } else if ua.contains("iPhone") || ua.contains("iPad") {
        "iOS"
    } else if ua.contains("Mac OS X") {
        "macOS"
    } else if ua.contains("Linux") {
        "Linux"
    } else {
        "unknown"
    };
    let version = ua
        .split('/')
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .unwrap_or("");
    Ok(table(&[
        ("Browser", browser.to_owned()),
        ("Version", version.to_owned()),
        ("System", os.to_owned()),
        ("String", ua.to_owned()),
    ]))
}

fn valid_email(text: &str, _: &mut Opts) -> Result<Output, String> {
    let re = Regex::new(r"^[A-Za-z0-9.!#$%&'*+/=?^_`{|}~-]+@[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?(?:\.[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?)+$").unwrap();
    if re.is_match(text.trim()) {
        Ok(verdict(true, "Valid email address."))
    } else {
        Err("Not a valid email address.".into())
    }
}

fn valid_url(text: &str, _: &mut Opts) -> Result<Output, String> {
    let url = text.trim();
    let Some((scheme, rest)) = url.split_once("://") else {
        return Err("A URL starts with a scheme such as https://.".into());
    };
    if !matches!(scheme, "http" | "https" | "ftp" | "ftps" | "ws" | "wss") {
        return Err(format!("“{scheme}” is not a web scheme."));
    }
    if rest.is_empty() || rest.starts_with('/') {
        return Err("A URL needs a host.".into());
    }
    Ok(verdict(true, "Valid URL."))
}

fn valid_ip(text: &str, _: &mut Opts) -> Result<Output, String> {
    match text.trim().parse::<IpAddr>() {
        Ok(ip) => Ok(verdict(
            true,
            format!(
                "Valid {} address: {}.",
                if ip.is_ipv4() { "IPv4" } else { "IPv6" },
                ip
            ),
        )),
        Err(_) => Err("Not a valid IPv4 or IPv6 address.".into()),
    }
}

fn valid_uuid(text: &str, _: &mut Opts) -> Result<Output, String> {
    let re =
        Regex::new(r"(?i)^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();
    if re.is_match(text.trim()) {
        Ok(verdict(true, "Valid UUID."))
    } else {
        Err("A UUID looks like 123e4567-e89b-12d3-a456-426614174000.".into())
    }
}

fn luhn(s: &str) -> bool {
    let digits: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() < 2 {
        return false;
    }
    digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, d)| {
            let mut d = *d;
            if i % 2 == 1 {
                d *= 2;
                if d > 9 {
                    d -= 9;
                }
            }
            d
        })
        .sum::<u32>()
        % 10
        == 0
}

fn valid_card(text: &str, _: &mut Opts) -> Result<Output, String> {
    let digits: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    if !digits.chars().all(|c| c.is_ascii_digit()) || !(12..=19).contains(&digits.len()) {
        return Err("A card number has 12 to 19 digits.".into());
    }
    if luhn(&digits) {
        Ok(verdict(
            true,
            format!("Valid by the Luhn check ({} digits).", digits.len()),
        ))
    } else {
        Err("The Luhn check fails: a digit is wrong.".into())
    }
}

fn iban_valid(s: &str) -> bool {
    let compact: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase();
    if compact.len() < 5 || compact.len() > 34 {
        return false;
    }
    let rearranged = format!("{}{}", &compact[4..], &compact[..4]);
    let mut rem = 0u32;
    for c in rearranged.chars() {
        if let Some(d) = c.to_digit(10) {
            rem = (rem * 10 + d) % 97;
        } else if c.is_ascii_uppercase() {
            rem = (rem * 100 + (c as u32 - 'A' as u32 + 10)) % 97;
        } else {
            return false;
        }
    }
    rem == 1
}

fn valid_iban(text: &str, _: &mut Opts) -> Result<Output, String> {
    if iban_valid(text) {
        Ok(verdict(true, "Valid IBAN by the mod-97 check."))
    } else {
        Err("Not a valid IBAN.".into())
    }
}

fn valid_domain(text: &str, _: &mut Opts) -> Result<Output, String> {
    let re = Regex::new(r"(?i)^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$").unwrap();
    let d = text.trim().trim_end_matches('.');
    if re.is_match(d) {
        Ok(verdict(
            true,
            format!(
                "Valid domain name, with {} label{}.",
                d.split('.').count(),
                if d.split('.').count() == 1 { "" } else { "s" }
            ),
        ))
    } else {
        Err("Not a valid domain name.".into())
    }
}

fn valid_semver(text: &str, _: &mut Opts) -> Result<Output, String> {
    let re = Regex::new(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$").unwrap();
    if re.is_match(text.trim()) {
        Ok(verdict(true, "Valid semantic version."))
    } else {
        Err("A semantic version looks like 1.2.3, 1.2.3-beta.1 or 1.2.3+build.".into())
    }
}

fn valid_json(text: &str, _: &mut Opts) -> Result<Output, String> {
    match hypereffect::serde_json::from_str::<hypereffect::Value>(text) {
        Ok(v) => Ok(verdict(
            true,
            format!(
                "Valid JSON: {} at the top.",
                match v {
                    hypereffect::Value::Array(_) => "an array",
                    hypereffect::Value::Object(_) => "an object",
                    hypereffect::Value::String(_) => "a string",
                    hypereffect::Value::Number(_) => "a number",
                    hypereffect::Value::Bool(_) => "a boolean",
                    hypereffect::Value::Null => "null",
                }
            ),
        )),
        Err(e) => Err(format!("Not JSON: {e}.")),
    }
}

fn meta_attr(attrs: &str, name: &str) -> Option<String> {
    let re = Regex::new(&format!(r#"(?i){name}\s*=\s*["']([^"']*)["']"#)).ok()?;
    re.captures(attrs).map(|c| c[1].to_owned())
}

fn meta_tags(text: &str, _: &mut Opts) -> Result<Output, String> {
    let title = Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap();
    let meta = Regex::new(r"(?is)<meta\s+([^>]+)>").unwrap();
    let mut rows = Vec::new();
    if let Some(c) = title.captures(text) {
        let value = c[1].trim();
        rows.push((
            "Title".to_owned(),
            format!("{value} ({} characters)", value.chars().count()),
        ));
    }
    for c in meta.captures_iter(text) {
        let attrs = &c[1];
        let Some(value) = meta_attr(attrs, "content") else {
            continue;
        };
        let key = meta_attr(attrs, "name")
            .or_else(|| meta_attr(attrs, "property"))
            .or_else(|| meta_attr(attrs, "http-equiv"));
        if let Some(key) = key {
            rows.push((key, value));
        }
    }
    if rows.is_empty() {
        Ok(verdict(false, "No title or meta tags found."))
    } else {
        Ok(table(
            &rows
                .iter()
                .map(|(a, b)| (a.as_str(), b.clone()))
                .collect::<Vec<_>>(),
        ))
    }
}

fn open_graph(text: &str, _: &mut Opts) -> Result<Output, String> {
    let meta = Regex::new(r"(?is)<meta\s+([^>]+)>").unwrap();
    let mut rows = Vec::new();
    for c in meta.captures_iter(text) {
        let attrs = &c[1];
        let Some(key) = meta_attr(attrs, "property").or_else(|| meta_attr(attrs, "name")) else {
            continue;
        };
        if key.to_ascii_lowercase().starts_with("og:") {
            if let Some(value) = meta_attr(attrs, "content") {
                rows.push((key, value));
            }
        }
    }
    if rows.is_empty() {
        Ok("No Open Graph tags found; add <meta property=\"og:title\" ...>.".into())
    } else {
        Ok(table(
            &rows
                .iter()
                .map(|(a, b)| (a.as_str(), b.clone()))
                .collect::<Vec<_>>(),
        ))
    }
}

fn sitemap(text: &str, _: &mut Opts) -> Result<Output, String> {
    let urls: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if urls.is_empty() {
        return Err("Write one URL per line.".into());
    }
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for url in &urls {
        let escaped = url
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");
        out.push_str(&format!("  <url><loc>{escaped}</loc></url>\n"));
    }
    out.push_str("</urlset>");
    Ok(super::snippet(out))
}

fn utm(_: &str, o: &mut Opts) -> Result<Output, String> {
    let base = o.field(0);
    if base.is_empty() {
        return Err("Fill in the URL.".into());
    }
    let pairs = [
        ("utm_source", o.field(1)),
        ("utm_medium", o.field(2)),
        ("utm_campaign", o.field(3)),
        ("utm_term", o.field(4)),
        ("utm_content", o.field(5)),
    ];
    let query: Vec<String> = pairs
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| format!("{k}={}", encode(v)))
        .collect();
    if query.is_empty() {
        return Err("Fill in at least one campaign field.".into());
    }
    Ok(super::snippet(format!(
        "{}{}{}",
        base,
        if base.contains('?') { "&" } else { "?" },
        query.join("&")
    )))
}

fn serp(_: &str, o: &mut Opts) -> Result<Output, String> {
    let title = o.field(0);
    let description = o.field(1);
    let url = o.field(2);
    let (t, d) = (title.chars().count(), description.chars().count());
    Ok(Output::default()
        .facts(&[("", title.to_owned()), ("", url.to_owned())])
        .text(description)
        .facts(&[
            (
                "Title",
                format!(
                    "{t} characters{}",
                    if t > 60 {
                        ", too long: about 60 fit"
                    } else {
                        ""
                    }
                ),
            ),
            (
                "Description",
                format!(
                    "{d} characters{}",
                    if d > 160 {
                        ", too long: about 160 fit"
                    } else {
                        ""
                    }
                ),
            ),
            ("URL", format!("{} characters", url.chars().count())),
        ]))
}

fn robots(_: &str, o: &mut Opts) -> Result<Output, String> {
    let mut out = format!(
        "User-agent: {}\n",
        if o.field(0).is_empty() {
            "*"
        } else {
            o.field(0)
        }
    );
    if !o.field(1).is_empty() {
        out.push_str(&format!("Disallow: {}\n", o.field(1)));
    }
    if !o.field(2).is_empty() {
        out.push_str(&format!("Allow: {}\n", o.field(2)));
    }
    if !o.field(3).is_empty() {
        out.push_str(&format!("Sitemap: {}\n", o.field(3)));
    }
    Ok(super::snippet(out))
}

fn cidr(_: &str, o: &mut Opts) -> Result<Output, String> {
    let (addr, prefix) = match o.field(0).split_once('/') {
        Some((a, p)) => (
            a,
            p.parse::<u8>()
                .map_err(|_| "The prefix after / is a number from 0 to 32.")?,
        ),
        None => (o.field(0), 32),
    };
    let ip: Ipv4Addr = addr
        .parse()
        .map_err(|_| "Write an IPv4 address such as 192.168.1.10/24.")?;
    if prefix > 32 {
        return Err("An IPv4 prefix goes from 0 to 32.".into());
    }
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    let value = u32::from(ip);
    let network = value & mask;
    let broadcast = network | !mask;
    let hosts = if prefix >= 31 {
        0
    } else {
        (1u64 << (32 - prefix)) - 2
    };
    Ok(table(&[
        ("Address", ip.to_string()),
        ("Prefix", format!("/{prefix}")),
        ("Netmask", Ipv4Addr::from(mask).to_string()),
        ("Network", Ipv4Addr::from(network).to_string()),
        ("Broadcast", Ipv4Addr::from(broadcast).to_string()),
        (
            "First host",
            Ipv4Addr::from(network + u32::from(prefix < 31)).to_string(),
        ),
        (
            "Last host",
            Ipv4Addr::from(broadcast - u32::from(prefix < 31)).to_string(),
        ),
        ("Hosts", hosts.to_string()),
    ]))
}

fn ip_convert(text: &str, _: &mut Opts) -> Result<Output, String> {
    let ip: Ipv4Addr = text
        .trim()
        .parse()
        .map_err(|_| "Write an IPv4 address such as 192.168.1.10.")?;
    let n = u32::from(ip);
    Ok(table(&[
        ("Dotted", ip.to_string()),
        ("Integer", n.to_string()),
        ("Hex", format!("0x{n:08X}")),
        ("Binary", format!("{n:032b}")),
        (
            "Class",
            match ip.octets()[0] {
                0..=127 => "A",
                128..=191 => "B",
                192..=223 => "C",
                224..=239 => "D (multicast)",
                _ => "E (reserved)",
            }
            .to_owned(),
        ),
        (
            "Private",
            if ip.is_private() { "yes" } else { "no" }.to_owned(),
        ),
        (
            "Loopback",
            if ip.is_loopback() { "yes" } else { "no" }.to_owned(),
        ),
    ]))
}

fn mac(text: &str, _: &mut Opts) -> Result<Output, String> {
    let hex: String = text.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() != 12 {
        return Err("A MAC address has twelve hexadecimal digits.".into());
    }
    let b: Vec<&str> = (0..6).map(|i| &hex[i * 2..i * 2 + 2]).collect();
    Ok(table(&[
        ("Colon", b.join(":").to_uppercase()),
        ("Hyphen", b.join("-").to_uppercase()),
        (
            "Cisco",
            format!(
                "{}.{}.{}",
                b[0..2].join(""),
                b[2..4].join(""),
                b[4..6].join("")
            ),
        ),
        ("OUI", b[0..3].join(":").to_uppercase()),
        (
            "Multicast",
            if u8::from_str_radix(b[0], 16).unwrap_or(0) & 1 == 1 {
                "yes"
            } else {
                "no"
            }
            .to_owned(),
        ),
        (
            "Local",
            if u8::from_str_radix(b[0], 16).unwrap_or(0) & 2 == 2 {
                "yes"
            } else {
                "no"
            }
            .to_owned(),
        ),
    ]))
}

fn port(_: &str, o: &mut Opts) -> Result<Output, String> {
    let key = o.field(0).to_ascii_lowercase();
    let found = if let Ok(n) = key.parse::<u16>() {
        PORTS.iter().find(|p| p.0 == n)
    } else {
        PORTS.iter().find(|p| p.1.eq_ignore_ascii_case(&key))
    };
    let (n, service, about) =
        found.ok_or_else(|| format!("“{}” is not in the table.", o.field(0)))?;
    Ok(table(&[
        ("Port", n.to_string()),
        ("Service", (*service).to_owned()),
        ("About", (*about).to_owned()),
        (
            "Transport",
            if *n == 53 {
                "UDP and TCP".to_owned()
            } else {
                "TCP".to_owned()
            },
        ),
    ]))
}

fn chmod(_: &str, o: &mut Opts) -> Result<Output, String> {
    let raw = o.field(0);
    let (mode, text) = if raw.chars().all(|c| c.is_ascii_digit()) {
        let mode =
            u32::from_str_radix(raw, 8).map_err(|_| "A numeric mode is octal, such as 755.")?;
        if mode > 0o7777 {
            return Err("A permission mode is at most 7777 in octal.".into());
        }
        let mut text = String::new();
        for (i, who) in ["user", "group", "other"].iter().enumerate() {
            let bits = (mode >> (6 - i * 3)) & 7;
            text.push_str(&format!(
                "{who}: {}{}{}\n",
                if bits & 4 != 0 { 'r' } else { '-' },
                if bits & 2 != 0 { 'w' } else { '-' },
                if bits & 1 != 0 { 'x' } else { '-' }
            ));
        }
        (mode, text)
    } else {
        let chars: Vec<char> = raw.chars().collect();
        if chars.len() != 9 {
            return Err("Write a mode such as 755 or rwxr-xr-x.".into());
        }
        let mut mode = 0u32;
        for (i, c) in chars.iter().enumerate() {
            let bit = match *c {
                'r' if i % 3 == 0 => 4,
                'w' if i % 3 == 1 => 2,
                'x' if i % 3 == 2 => 1,
                '-' => 0,
                _ => return Err("Use r, w, x and - in symbolic modes.".into()),
            };
            mode |= bit << (8 - i);
        }
        (mode, raw.to_owned())
    };
    Ok(table(&[("Octal", format!("{mode:o}")), ("Symbolic", text)]))
}

fn ascii(_: &str, o: &mut Opts) -> Result<Output, String> {
    let key = o.field(0);
    if key.is_empty() {
        let rows = (32u8..=126).map(|c| {
            vec![
                (c as char).to_string(),
                c.to_string(),
                format!("0x{c:02X}"),
                format!("{c:08b}"),
            ]
        });
        return Ok(super::grid(
            &["Character", "Decimal", "Hex", "Binary"],
            rows,
        ));
    }
    let code = if key.chars().count() == 1 {
        key.chars().next().unwrap() as u32
    } else {
        key.parse::<u32>()
            .map_err(|_| "Write one character or a code point.".to_owned())?
    };
    let c = char::from_u32(code).ok_or("That is not a Unicode code point.")?;
    Ok(table(&[
        ("Character", c.to_string()),
        ("Decimal", code.to_string()),
        ("Hex", format!("U+{code:04X}")),
        ("HTML", format!("&#{code};")),
        (
            "UTF-8",
            c.to_string()
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" "),
        ),
    ]))
}

fn html_entity(_: &str, o: &mut Opts) -> Result<Output, String> {
    let key = o.field(0);
    if key.is_empty() {
        return Ok(table(
            &HTML_ENTITIES
                .iter()
                .map(|(name, entity, code)| (*name, format!("{entity}  U+{code:04X}  {code}")))
                .collect::<Vec<_>>(),
        ));
    }
    let (name, entity, code) =
        entity(key).ok_or_else(|| format!("“{key}” is not in the table."))?;
    Ok(table(&[
        ("Name", (*name).to_owned()),
        ("Entity", (*entity).to_owned()),
        ("Code point", format!("U+{code:04X}")),
        (
            "Character",
            char::from_u32(*code).map_or(String::new(), |c| c.to_string()),
        ),
    ]))
}

fn country_code(_: &str, o: &mut Opts) -> Result<Output, String> {
    let key = o.field(0).to_ascii_lowercase();
    let matches: Vec<&(&str, &str, &str, &str)> = COUNTRY_CODES
        .iter()
        .filter(|c| {
            c.0.to_ascii_lowercase().contains(&key)
                || c.1.eq_ignore_ascii_case(&key)
                || c.2.eq_ignore_ascii_case(&key)
        })
        .collect();
    if matches.is_empty() {
        return Err(format!("No country matches “{}”.", o.field(0)));
    }
    Ok(table(
        &matches
            .iter()
            .map(|c| (c.0, format!("{}  {}  {}", c.1, c.2, c.3)))
            .collect::<Vec<_>>(),
    ))
}

fn currency_code(_: &str, o: &mut Opts) -> Result<Output, String> {
    let key = o.field(0).to_ascii_lowercase();
    let matches: Vec<&(&str, &str, &str)> = CURRENCIES
        .iter()
        .filter(|c| {
            c.0.eq_ignore_ascii_case(&key)
                || c.1.to_ascii_lowercase().contains(&key)
                || c.2.to_ascii_lowercase().contains(&key)
        })
        .collect();
    if matches.is_empty() {
        return Err(format!("No currency matches “{}”.", o.field(0)));
    }
    Ok(table(
        &matches
            .iter()
            .map(|c| (c.0, format!("{} — {}", c.1, c.2)))
            .collect::<Vec<_>>(),
    ))
}

fn file_signature(_: &str, o: &mut Opts) -> Result<Output, String> {
    let key = o.field(0).trim_start_matches('.').to_ascii_lowercase();
    let matches: Vec<&(&str, &str, &str)> = FILE_SIGNATURES
        .iter()
        .filter(|f| f.0.eq_ignore_ascii_case(&key))
        .collect();
    if matches.is_empty() {
        return Err(format!("No signature for “{}”.", o.field(0)));
    }
    Ok(table(
        &matches
            .iter()
            .map(|f| (f.0, format!("{}  ({})", f.1, f.2)))
            .collect::<Vec<_>>(),
    ))
}

fn timezone(_: &str, o: &mut Opts) -> Result<Output, String> {
    let key = o.field(0).to_ascii_lowercase();
    if key.is_empty() {
        return Ok(super::list(TIMEZONES.iter().copied()));
    }
    let matches: Vec<&&str> = TIMEZONES
        .iter()
        .filter(|z| z.to_ascii_lowercase().contains(&key))
        .collect();
    if matches.is_empty() {
        return Err(format!("No time zone matches “{}”.", o.field(0)));
    }
    Ok(super::list(matches.iter().map(|z| **z)))
}

pub const DEVELOPER: &[Tool] = &[
    tool(
        "web-http-status",
        "HTTP status code",
        "What a status code means.",
        http_status,
    )
    .form()
    .fields(&[("Code", "200")]),
    tool(
        "web-mime",
        "MIME type",
        "The media type for an extension, or the extension for a media type.",
        mime_type,
    )
    .form()
    .fields(&[("Extension or media type", "json")]),
    tool(
        "web-jwt",
        "Decode a JWT",
        "Read the header and payload of a JSON web token.",
        jwt,
    )
    .text("JSON web token"),
    tool(
        "web-uuid",
        "UUID generator",
        "Random version 4 UUIDs.",
        uuid,
    )
    .form()
    .checks(&["Uppercase"])
    .slider("How many", 1.0, 20.0, 1.0),
    tool(
        "web-password",
        "Password generator",
        "Random passwords from letters, digits and symbols.",
        password,
    )
    .form()
    .fields(&[("Exclude characters (start with -)", "")])
    .checks(&["Digits", "Symbols"])
    .slider("Length", 8.0, 64.0, 16.0),
    tool(
        "web-cron",
        "Cron expression",
        "Checks a five-field cron expression and explains its parts.",
        cron,
    )
    .text("Cron expression, such as 0 9 * * 1-5"),
    tool(
        "web-regex",
        "Regex tester",
        "Matches and positions for a regular expression.",
        regex_test,
    )
    .fields(&[("Pattern", "\\w+")]),
    tool(
        "web-curl",
        "curl to fetch",
        "Turns a curl command into JavaScript fetch code.",
        curl,
    )
    .text("curl command"),
    tool(
        "web-user-agent",
        "User agent",
        "The browser, version and system in a user agent string.",
        user_agent,
    )
    .text("User agent string"),
    tool(
        "web-http-methods",
        "HTTP methods",
        "What each request method is for.",
        methods,
    )
    .form(),
    tool(
        "web-http-headers",
        "HTTP headers",
        "Common request and response headers.",
        headers,
    )
    .form(),
];

fn methods(_: &str, _: &mut Opts) -> Result<Output, String> {
    Ok(table(
        &HTTP_METHODS
            .iter()
            .map(|(m, d)| (*m, (*d).to_owned()))
            .collect::<Vec<_>>(),
    ))
}

fn headers(_: &str, _: &mut Opts) -> Result<Output, String> {
    Ok(table(
        &HTTP_HEADERS
            .iter()
            .map(|(h, d)| (*h, (*d).to_owned()))
            .collect::<Vec<_>>(),
    ))
}

pub const VALIDATE: &[Tool] = &[
    tool(
        "web-valid-email",
        "Email validator",
        "Checks the shape of an email address.",
        valid_email,
    ),
    tool(
        "web-valid-url",
        "URL validator",
        "Checks a web address and its scheme.",
        valid_url,
    ),
    tool(
        "web-valid-ip",
        "IP validator",
        "Checks an IPv4 or IPv6 address.",
        valid_ip,
    ),
    tool(
        "web-valid-uuid",
        "UUID validator",
        "Checks the shape of a UUID.",
        valid_uuid,
    ),
    tool(
        "web-valid-card",
        "Card validator",
        "The Luhn check for card numbers.",
        valid_card,
    ),
    tool(
        "web-valid-iban",
        "IBAN validator",
        "The mod-97 check for IBANs.",
        valid_iban,
    ),
    tool(
        "web-valid-domain",
        "Domain validator",
        "Checks a domain name and its labels.",
        valid_domain,
    ),
    tool(
        "web-valid-semver",
        "Semantic version",
        "Checks a semantic version such as 1.2.3-beta.1.",
        valid_semver,
    ),
    tool(
        "web-valid-json",
        "JSON validator",
        "Checks that text is JSON.",
        valid_json,
    ),
];

pub const WEB: &[Tool] = &[
    tool(
        "web-meta-tags",
        "Meta tags",
        "Title and meta tags in HTML.",
        meta_tags,
    )
    .text("HTML"),
    tool(
        "web-open-graph",
        "Open Graph tags",
        "The og: tags in HTML.",
        open_graph,
    )
    .text("HTML"),
    tool(
        "web-sitemap",
        "Sitemap",
        "Turns a list of URLs into a sitemap.xml.",
        sitemap,
    )
    .text("URLs, one per line"),
    tool(
        "web-utm",
        "UTM builder",
        "Builds campaign URLs for analytics.",
        utm,
    )
    .form()
    .fields(&[
        ("URL", "https://example.com"),
        ("Source", "newsletter"),
        ("Medium", "email"),
        ("Campaign", "launch"),
        ("Term", ""),
        ("Content", ""),
    ]),
    tool(
        "web-serp",
        "Search result preview",
        "How a title, description and URL look in a search result, with lengths.",
        serp,
    )
    .form()
    .fields(&[
        ("Title", "Text Utilities"),
        (
            "Description",
            "A pocket knife of small tools for text, data, numbers and more.",
        ),
        ("URL", "https://example.com/textutils"),
    ]),
    tool(
        "web-robots",
        "robots.txt",
        "Builds a robots.txt file.",
        robots,
    )
    .form()
    .fields(&[
        ("User-agent", "*"),
        ("Disallow", "/admin"),
        ("Allow", ""),
        ("Sitemap", "https://example.com/sitemap.xml"),
    ]),
];

pub const NETWORK: &[Tool] = &[
    tool(
        "web-cidr",
        "CIDR calculator",
        "Network, broadcast and host range for an IPv4 prefix.",
        cidr,
    )
    .form()
    .fields(&[("IPv4 address and prefix", "192.168.1.10/24")]),
    tool(
        "web-ip-convert",
        "IPv4 converter",
        "Dotted, integer, hexadecimal and binary forms.",
        ip_convert,
    )
    .text("IPv4 address"),
    tool(
        "web-mac",
        "MAC address",
        "Formats a MAC address and reads its OUI.",
        mac,
    )
    .text("MAC address"),
    tool(
        "web-port",
        "Port lookup",
        "The service usually found on a port.",
        port,
    )
    .form()
    .fields(&[("Port or service", "443")]),
    tool(
        "web-chmod",
        "chmod calculator",
        "Numeric and symbolic Unix permissions.",
        chmod,
    )
    .form()
    .fields(&[("Mode", "755")]),
];

pub const REFERENCE: &[Tool] = &[
    tool(
        "web-ascii",
        "ASCII table",
        "Codes, HTML and UTF-8 for a character, or the printable table.",
        ascii,
    )
    .form()
    .fields(&[("Character or code (empty: all)", "A")]),
    tool(
        "web-entities",
        "HTML entities",
        "Named entities and their code points.",
        html_entity,
    )
    .form()
    .fields(&[("Entity (empty: all)", "amp")]),
    tool(
        "web-country-codes",
        "Country codes",
        "ISO 3166 codes and calling codes.",
        country_code,
    )
    .form()
    .fields(&[("Country, ISO code or calling code", "US")]),
    tool(
        "web-currencies",
        "Currency codes",
        "ISO 4217 currency codes.",
        currency_code,
    )
    .form()
    .fields(&[("Code, name or country", "USD")]),
    tool(
        "web-file-signatures",
        "File signatures",
        "The first bytes of common file types.",
        file_signature,
    )
    .form()
    .fields(&[("Extension", "png")]),
    tool(
        "web-timezones",
        "Time zones",
        "IANA time zone names.",
        timezone,
    )
    .form()
    .fields(&[("Search (empty: all)", "")]),
    tool(
        "web-status-table",
        "HTTP status table",
        "All the status codes in the table.",
        statuses,
    )
    .form(),
];

fn statuses(_: &str, _: &mut Opts) -> Result<Output, String> {
    Ok(table(
        &HTTP_STATUS
            .iter()
            .map(|(code, reason, _)| (*reason, format!("{code} — {reason}")))
            .collect::<Vec<_>>(),
    ))
}
