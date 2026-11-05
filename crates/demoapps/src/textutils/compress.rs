//! Compression and decompression, with the compressed bytes written as text.

use super::Output;
use super::encode::{base96_decode, base96_encode};
use super::{Opts, Tool, tool};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::io::{Read, Write};

const ALGORITHMS: super::Menu = &[
    ("zstd", "Zstandard (zstd)"),
    ("brotli", "Brotli"),
    ("lzma", "LZMA"),
    ("lz4", "LZ4"),
];

const DIRECTIONS: super::Menu = &[("compress", "Compress"), ("decompress", "Decompress")];

const ENCODINGS: super::Menu = &[("base96", "Base96"), ("base64", "Base64"), ("hex", "Hex")];

fn zip(algorithm: &str, data: &[u8], level: usize) -> Result<Vec<u8>, String> {
    match algorithm {
        "brotli" => {
            let mut out = Vec::new();
            {
                let mut writer =
                    brotli::CompressorWriter::new(&mut out, 4096, level.clamp(1, 11) as u32, 22);
                writer
                    .write_all(data)
                    .map_err(|e| format!("Brotli failed: {e}"))?;
                writer.flush().map_err(|e| format!("Brotli failed: {e}"))?;
            }
            Ok(out)
        }
        "lzma" => {
            let mut out = Vec::new();
            lzma_rs::lzma_compress(&mut &data[..], &mut out)
                .map_err(|e| format!("LZMA failed: {e}"))?;
            Ok(out)
        }
        "lz4" => Ok(lz4_flex::block::compress_prepend_size(data)),
        _ => zstd::stream::encode_all(data, level.clamp(1, 19) as i32)
            .map_err(|e| format!("zstd failed: {e}")),
    }
}

fn unzip(algorithm: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    match algorithm {
        "brotli" => brotli::Decompressor::new(data, 4096)
            .read_to_end(&mut out)
            .map(|_| out)
            .map_err(|e| format!("This is not Brotli data, or it is damaged: {e}")),
        "lzma" => {
            lzma_rs::lzma_decompress(&mut &data[..], &mut out)
                .map_err(|e| format!("This is not LZMA data, or it is damaged: {e}"))?;
            Ok(out)
        }
        "lz4" => lz4_flex::block::decompress_size_prepended(data)
            .map_err(|e| format!("This is not LZ4 data, or it is damaged: {e}")),
        _ => zstd::stream::decode_all(data)
            .map_err(|e| format!("This is not zstd data, or it is damaged: {e}")),
    }
}

fn encode(data: &[u8], encoding: &str) -> String {
    match encoding {
        "base64" => STANDARD.encode(data),
        "hex" => data.iter().map(|b| format!("{b:02x}")).collect(),
        _ => base96_encode(data),
    }
}

fn decode(text: &str, encoding: &str) -> Result<Vec<u8>, String> {
    match encoding {
        "base64" => {
            let clean: String = text.chars().filter(|c| !c.is_whitespace()).collect();
            STANDARD
                .decode(clean)
                .map_err(|e| format!("This is not Base64: {e}."))
        }
        "hex" => super::encode::unhex(text),
        _ => base96_decode(text),
    }
}

fn ratio(before: usize, after: usize) -> String {
    if before == 0 {
        return "—".into();
    }
    let percent = after as f64 * 100.0 / before as f64;
    if after <= before {
        format!(
            "{} saved ({percent:.1}% of the original)",
            super::fmt((before - after) as f64, 0)
        )
    } else {
        format!(
            "{} larger ({percent:.1}% of the original)",
            super::fmt((after - before) as f64, 0)
        )
    }
}

fn compression(text: &str, o: &mut Opts) -> Result<Output, String> {
    let algorithm = o.mode;
    let decompress = o.choice(1) == "decompress";
    let encoding = o.choice(2);
    let name = ALGORITHMS
        .iter()
        .find(|(id, _)| *id == algorithm)
        .map_or(algorithm, |(_, label)| label);

    if decompress {
        let packed = decode(text, encoding)?;
        let bytes = unzip(algorithm, &packed)?;
        let plain = String::from_utf8(bytes)
            .map_err(|_| "The decompressed bytes are not UTF-8 text.".to_owned())?;
        return Ok(Output::default().code(plain.clone()).facts(&[
            ("Method", name.to_owned()),
            (
                "Compressed size",
                format!("{} bytes", super::fmt(packed.len() as f64, 0)),
            ),
            (
                "Decompressed size",
                format!("{} bytes", super::fmt(plain.len() as f64, 0)),
            ),
            ("Change", ratio(packed.len(), plain.len())),
        ]));
    }

    let compressed = zip(algorithm, text.as_bytes(), o.number)?;
    let written = encode(&compressed, encoding);
    Ok(Output::default()
        .code(written)
        .facts(&[
            ("Method", name.to_owned()),
            ("Original size", format!("{} bytes", super::fmt(text.len() as f64, 0))),
            ("Compressed size", format!("{} bytes", super::fmt(compressed.len() as f64, 0))),
            ("Change", ratio(text.len(), compressed.len())),
            ("Written as", ENCODINGS.iter().find(|(id, _)| *id == encoding).map_or(encoding, |(_, label)| label).to_owned()),
        ])
        .note("The code block holds the compressed bytes as text. Paste it back here with the same method and encoding, set to Decompress."))
}

pub const COMPRESSION: &[Tool] = &[tool(
    "compress",
    "Compress text",
    "Compresses text with zstd, Brotli, LZMA or LZ4, as Base96, Base64 or hex, and back.",
    compression,
)
.selects(&[
    ("Method", ALGORITHMS),
    ("Direction", DIRECTIONS),
    ("Encoded as", ENCODINGS),
])
.slider("Level", 1.0, 19.0, 3.0)];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_algorithm_round_trips() {
        let text = "Hello, compression! Hello, compression! Hello, compression!";
        for (algorithm, _) in ALGORITHMS {
            let packed = zip(algorithm, text.as_bytes(), 6).unwrap();
            assert_eq!(
                unzip(algorithm, &packed).unwrap(),
                text.as_bytes(),
                "{algorithm}"
            );
        }
    }

    #[test]
    fn every_encoding_round_trips() {
        let data: Vec<u8> = (0..=255).collect();
        for (encoding, _) in ENCODINGS {
            let text = encode(&data, encoding);
            assert_eq!(decode(&text, encoding).unwrap(), data, "{encoding}");
        }
    }
}
