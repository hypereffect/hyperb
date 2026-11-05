//! Encryption, signatures and Shamir secret sharing.
//!
//! The tools are for learning and for moving small secrets around. The random
//! numbers come from the demo's clock, not from the operating system, so keys
//! made here are not for anything valuable.

use super::{Opts, Output, Rng, Tool, tool, verdict};
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
use aes_gcm::aead::{Aead, KeyInit};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chacha20::ChaCha20;
use chacha20::cipher::StreamCipher;
use k256::ecdsa::signature::{Signer, Verifier};
use pbkdf2::pbkdf2_hmac;
use sha2::{Digest, Sha256, Sha512};

const CIPHERS: super::Menu = &[
    ("aes-256-cbc", "AES-256-CBC"),
    ("aes-128-cbc", "AES-128-CBC"),
    ("aes-256-gcm", "AES-256-GCM"),
    ("chacha20", "ChaCha20"),
    ("chacha20poly1305", "ChaCha20-Poly1305"),
    ("xchacha20poly1305", "XChaCha20-Poly1305"),
];

const DIRECTIONS: super::Menu = &[("encrypt", "Encrypt"), ("decrypt", "Decrypt")];

const SIGNATURES: super::Menu = &[
    ("ecdsa-secp256k1", "ECDSA secp256k1"),
    ("schnorr-secp256k1", "Schnorr secp256k1 (BIP340)"),
    ("ed25519", "Ed25519"),
    ("ristretto", "Ristretto255 Schnorr"),
    ("ecdsa-p256", "ECDSA P-256 (secp256r1)"),
];

const SHARES: super::Menu = &[
    ("2-of-3", "2 of 3"),
    ("3-of-5", "3 of 5"),
    ("2-of-5", "2 of 5"),
    ("3-of-3", "3 of 3"),
    ("4-of-7", "4 of 7"),
];

fn label(menu: super::Menu, id: &str) -> String {
    menu.iter()
        .find(|(value, _)| *value == id)
        .map_or(id, |(_, label)| *label)
        .to_owned()
}

/// Fills `out` from the demo's clock-seeded generator.
fn fill(rng: &mut Rng, out: &mut [u8]) {
    for chunk in out.chunks_mut(8) {
        let bytes = rng.next().to_le_bytes();
        chunk.copy_from_slice(&bytes[..chunk.len()]);
    }
}

// Encryption

fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 100_000, &mut key);
    key
}

fn encrypt_text(
    algorithm: &str,
    password: &str,
    plain: &str,
    rng: &mut Rng,
) -> Result<String, String> {
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 24];
    fill(rng, &mut salt);
    fill(rng, &mut nonce);
    let key = derive_key(password, &salt);
    let ciphertext = match algorithm {
        "aes-128-cbc" => cbc::Encryptor::<aes::Aes128>::new_from_slices(&key[..16], &nonce[..16])
            .map_err(|_| "AES could not take that key.".to_owned())?
            .encrypt_padded_vec_mut::<Pkcs7>(plain.as_bytes()),
        "aes-256-cbc" => cbc::Encryptor::<aes::Aes256>::new_from_slices(&key, &nonce[..16])
            .map_err(|_| "AES could not take that key.".to_owned())?
            .encrypt_padded_vec_mut::<Pkcs7>(plain.as_bytes()),
        "aes-256-gcm" => {
            let cipher = aes_gcm::Aes256Gcm::new_from_slice(&key)
                .map_err(|_| "AES-GCM could not take that key.".to_owned())?;
            cipher
                .encrypt(aes_gcm::Nonce::from_slice(&nonce[..12]), plain.as_bytes())
                .map_err(|_| "AES-GCM could not encrypt this text.".to_owned())?
        }
        "chacha20" => {
            // ChaCha20 alone does not authenticate, so a checksum makes a
            // wrong password fail cleanly instead of returning garbage.
            let mut bytes = Sha256::digest(plain.as_bytes()).to_vec();
            bytes.extend_from_slice(plain.as_bytes());
            let mut cipher = ChaCha20::new_from_slices(&key, &nonce[..12])
                .map_err(|_| "ChaCha20 could not take that key.".to_owned())?;
            cipher.apply_keystream(&mut bytes);
            bytes
        }
        "chacha20poly1305" => {
            let cipher = chacha20poly1305::ChaCha20Poly1305::new_from_slice(&key)
                .map_err(|_| "ChaCha20-Poly1305 could not take that key.".to_owned())?;
            cipher
                .encrypt(
                    chacha20poly1305::Nonce::from_slice(&nonce[..12]),
                    plain.as_bytes(),
                )
                .map_err(|_| "ChaCha20-Poly1305 could not encrypt this text.".to_owned())?
        }
        _ => {
            let cipher = chacha20poly1305::XChaCha20Poly1305::new_from_slice(&key)
                .map_err(|_| "XChaCha20-Poly1305 could not take that key.".to_owned())?;
            cipher
                .encrypt(
                    chacha20poly1305::XNonce::from_slice(&nonce),
                    plain.as_bytes(),
                )
                .map_err(|_| "XChaCha20-Poly1305 could not encrypt this text.".to_owned())?
        }
    };
    let mut envelope = Vec::with_capacity(16 + 24 + ciphertext.len());
    envelope.extend_from_slice(&salt);
    envelope.extend_from_slice(&nonce);
    envelope.extend_from_slice(&ciphertext);
    Ok(STANDARD.encode(envelope))
}

fn decrypt_text(algorithm: &str, password: &str, wrapped: &str) -> Result<String, String> {
    let clean: String = wrapped.chars().filter(|c| !c.is_whitespace()).collect();
    let envelope = STANDARD
        .decode(clean)
        .map_err(|e| format!("This is not a Base64 envelope: {e}."))?;
    if envelope.len() < 16 + 24 {
        return Err(
            "The envelope is too short: it needs a salt, a nonce and the ciphertext.".into(),
        );
    }
    let (salt, rest) = envelope.split_at(16);
    let (nonce, ciphertext) = rest.split_at(24);
    let key = derive_key(password, salt);
    let bytes = match algorithm {
        "aes-128-cbc" => cbc::Decryptor::<aes::Aes128>::new_from_slices(&key[..16], &nonce[..16])
            .map_err(|_| "AES could not take that key.".to_owned())?
            .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
            .map_err(|_| "The password is wrong, or the ciphertext is damaged.".to_owned())?,
        "aes-256-cbc" => cbc::Decryptor::<aes::Aes256>::new_from_slices(&key, &nonce[..16])
            .map_err(|_| "AES could not take that key.".to_owned())?
            .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
            .map_err(|_| "The password is wrong, or the ciphertext is damaged.".to_owned())?,
        "aes-256-gcm" => {
            let cipher = aes_gcm::Aes256Gcm::new_from_slice(&key)
                .map_err(|_| "AES-GCM could not take that key.".to_owned())?;
            cipher
                .decrypt(aes_gcm::Nonce::from_slice(&nonce[..12]), ciphertext)
                .map_err(|_| "The password is wrong, or the ciphertext is damaged.".to_owned())?
        }
        "chacha20" => {
            let mut bytes = ciphertext.to_vec();
            let mut cipher = ChaCha20::new_from_slices(&key, &nonce[..12])
                .map_err(|_| "ChaCha20 could not take that key.".to_owned())?;
            cipher.apply_keystream(&mut bytes);
            if bytes.len() < 32 || bytes[..32] != Sha256::digest(&bytes[32..])[..] {
                return Err("The password is wrong, or the ciphertext is damaged.".into());
            }
            bytes[32..].to_vec()
        }
        "chacha20poly1305" => {
            let cipher = chacha20poly1305::ChaCha20Poly1305::new_from_slice(&key)
                .map_err(|_| "ChaCha20-Poly1305 could not take that key.".to_owned())?;
            cipher
                .decrypt(
                    chacha20poly1305::Nonce::from_slice(&nonce[..12]),
                    ciphertext,
                )
                .map_err(|_| "The password is wrong, or the ciphertext is damaged.".to_owned())?
        }
        _ => {
            let cipher = chacha20poly1305::XChaCha20Poly1305::new_from_slice(&key)
                .map_err(|_| "XChaCha20-Poly1305 could not take that key.".to_owned())?;
            cipher
                .decrypt(chacha20poly1305::XNonce::from_slice(nonce), ciphertext)
                .map_err(|_| "The password is wrong, or the ciphertext is damaged.".to_owned())?
        }
    };
    String::from_utf8(bytes)
        .map_err(|_| "The decrypted bytes are not UTF-8 text; is the password right?".into())
}

fn cipher(text: &str, o: &mut Opts) -> Result<Output, String> {
    let algorithm = o.mode;
    let password = o.field(0);
    if password.is_empty() {
        return Err("Fill in a password (the key is derived from it).".into());
    }
    let name = label(CIPHERS, algorithm);
    if o.choice(1) == "decrypt" {
        let plain = decrypt_text(algorithm, password, text)?;
        return Ok(Output::default().code(plain).facts(&[
            ("Method", name),
            (
                "Key derivation",
                "PBKDF2-HMAC-SHA256, 100,000 rounds".into(),
            ),
        ]));
    }
    let wrapped = encrypt_text(algorithm, password, text, o.rng)?;
    Ok(Output::default()
        .code(wrapped)
        .facts(&[
            ("Method", name.clone()),
            ("Key derivation", "PBKDF2-HMAC-SHA256, 100,000 rounds".into()),
            ("Envelope", "salt (16 bytes) + nonce (24 bytes) + ciphertext, in Base64".into()),
        ])
        .note(format!(
            "{name} with this password. Paste the envelope back with the same method, set to Decrypt."
        ))
        .note("AES-CBC and plain ChaCha20 do not authenticate the message; the authenticated modes do."))
}

// Signatures

fn parse_secret(text: &str) -> Result<[u8; 32], String> {
    let raw = text.trim().trim_start_matches("0x");
    let bytes = hex::decode(raw).map_err(|e| format!("The secret key is not hex: {e}."))?;
    bytes
        .try_into()
        .map_err(|b: Vec<u8>| format!("A secret key is 32 bytes (64 hex digits), not {}.", b.len()))
}

fn new_secret(algorithm: &str, rng: &mut Rng) -> [u8; 32] {
    let mut secret = [0u8; 32];
    for _ in 0..100 {
        fill(rng, &mut secret);
        if sign_with(algorithm, &secret, b"").is_ok() {
            break;
        }
    }
    secret
}

fn sign_with(
    algorithm: &str,
    secret: &[u8; 32],
    message: &[u8],
) -> Result<(Vec<u8>, Vec<u8>), String> {
    match algorithm {
        "ecdsa-secp256k1" => {
            let key = k256::ecdsa::SigningKey::from_slice(secret)
                .map_err(|_| "That is not a valid secp256k1 secret key.".to_owned())?;
            let public = key
                .verifying_key()
                .to_encoded_point(true)
                .as_bytes()
                .to_vec();
            let signature: k256::ecdsa::Signature = key.sign(message);
            Ok((public, signature.to_bytes().to_vec()))
        }
        "schnorr-secp256k1" => {
            let key = k256::schnorr::SigningKey::from_bytes(secret)
                .map_err(|_| "That is not a valid Schnorr secret key.".to_owned())?;
            let public = key.verifying_key().to_bytes().to_vec();
            let signature: k256::schnorr::Signature = key.sign(message);
            Ok((public, signature.to_bytes().to_vec()))
        }
        "ed25519" => {
            let key = ed25519_dalek::SigningKey::from_bytes(secret);
            let public = key.verifying_key().to_bytes().to_vec();
            let signature = key.sign(message).to_bytes().to_vec();
            Ok((public, signature))
        }
        "ristretto" => {
            let scalar = curve25519_dalek::scalar::Scalar::from_bytes_mod_order(*secret);
            if scalar == curve25519_dalek::scalar::Scalar::ZERO {
                return Err("That secret key is zero.".into());
            }
            let point = scalar * curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
            let public = point.compress().to_bytes().to_vec();
            let signature = ristretto_sign(&scalar, &point, secret, message);
            Ok((public, signature))
        }
        _ => {
            let key = p256::ecdsa::SigningKey::from_slice(secret)
                .map_err(|_| "That is not a valid P-256 secret key.".to_owned())?;
            let public = key
                .verifying_key()
                .to_encoded_point(true)
                .as_bytes()
                .to_vec();
            let signature: p256::ecdsa::Signature = key.sign(message);
            Ok((public, signature.to_bytes().to_vec()))
        }
    }
}

fn verify_with(
    algorithm: &str,
    public: &[u8],
    signature: &[u8],
    message: &[u8],
) -> Result<bool, String> {
    match algorithm {
        "ecdsa-secp256k1" => {
            let key = k256::ecdsa::VerifyingKey::from_sec1_bytes(public)
                .map_err(|_| "That is not a secp256k1 public key.".to_owned())?;
            let signature = k256::ecdsa::Signature::from_slice(signature)
                .map_err(|_| "That is not a 64-byte ECDSA signature.".to_owned())?;
            Ok(key.verify(message, &signature).is_ok())
        }
        "schnorr-secp256k1" => {
            let key = k256::schnorr::VerifyingKey::from_bytes(public)
                .map_err(|_| "That is not a 32-byte Schnorr public key.".to_owned())?;
            let signature = k256::schnorr::Signature::try_from(signature)
                .map_err(|_| "That is not a 64-byte Schnorr signature.".to_owned())?;
            Ok(key.verify(message, &signature).is_ok())
        }
        "ed25519" => {
            let public: &[u8; 32] = public
                .try_into()
                .map_err(|_| "An Ed25519 public key is 32 bytes.".to_owned())?;
            let key = ed25519_dalek::VerifyingKey::from_bytes(public)
                .map_err(|_| "That is not an Ed25519 public key.".to_owned())?;
            let signature = ed25519_dalek::Signature::from_slice(signature)
                .map_err(|_| "That is not a 64-byte Ed25519 signature.".to_owned())?;
            Ok(key.verify_strict(message, &signature).is_ok())
        }
        "ristretto" => {
            let public: &[u8; 32] = public
                .try_into()
                .map_err(|_| "A Ristretto public key is 32 bytes.".to_owned())?;
            let point = curve25519_dalek::ristretto::CompressedRistretto::from_slice(public)
                .map_err(|_| "That is not a Ristretto public key.".to_owned())?
                .decompress()
                .ok_or_else(|| "That Ristretto public key is not a valid point.".to_owned())?;
            if signature.len() != 64 {
                return Err("A Ristretto signature is 64 bytes.".into());
            }
            let r = curve25519_dalek::ristretto::CompressedRistretto::from_slice(&signature[..32])
                .map_err(|_| "That is not a Ristretto signature.".to_owned())?
                .decompress()
                .ok_or_else(|| {
                    "The Ristretto signature's first half is not a valid point.".to_owned()
                })?;
            let s: [u8; 32] = signature[32..].try_into().unwrap_or([0; 32]);
            let s = Option::<curve25519_dalek::scalar::Scalar>::from(
                curve25519_dalek::scalar::Scalar::from_canonical_bytes(s),
            )
            .ok_or_else(|| "The Ristretto signature's second half is not a scalar.".to_owned())?;
            let e = ristretto_challenge(&r, &point, message);
            let base = curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
            Ok(s * base == r + e * point)
        }
        _ => {
            let key = p256::ecdsa::VerifyingKey::from_sec1_bytes(public)
                .map_err(|_| "That is not a P-256 public key.".to_owned())?;
            let signature = p256::ecdsa::Signature::from_slice(signature)
                .map_err(|_| "That is not a 64-byte ECDSA signature.".to_owned())?;
            Ok(key.verify(message, &signature).is_ok())
        }
    }
}

fn ristretto_challenge(
    r: &curve25519_dalek::ristretto::RistrettoPoint,
    public: &curve25519_dalek::ristretto::RistrettoPoint,
    message: &[u8],
) -> curve25519_dalek::scalar::Scalar {
    let mut hash = Sha512::new();
    hash.update(b"hyperb/ristretto-schnorr/challenge");
    hash.update(r.compress().as_bytes());
    hash.update(public.compress().as_bytes());
    hash.update(message);
    curve25519_dalek::scalar::Scalar::from_hash(hash)
}

fn ristretto_sign(
    secret: &curve25519_dalek::scalar::Scalar,
    public: &curve25519_dalek::ristretto::RistrettoPoint,
    secret_bytes: &[u8; 32],
    message: &[u8],
) -> Vec<u8> {
    let mut hash = Sha512::new();
    hash.update(b"hyperb/ristretto-schnorr/nonce");
    hash.update(secret_bytes);
    hash.update(message);
    let k = curve25519_dalek::scalar::Scalar::from_hash(hash);
    let r = k * curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
    let e = ristretto_challenge(&r, public, message);
    let s = k + e * secret;
    let mut signature = r.compress().to_bytes().to_vec();
    signature.extend_from_slice(&s.to_bytes());
    signature
}

fn sign(text: &str, o: &mut Opts) -> Result<Output, String> {
    let algorithm = o.mode;
    let given = o.field(0);
    let made = given.trim().is_empty();
    let secret = if made {
        new_secret(algorithm, o.rng)
    } else {
        parse_secret(given)?
    };
    let (public, signature) = sign_with(algorithm, &secret, text.as_bytes())?;
    let mut out = super::table(&[
        ("Algorithm", label(SIGNATURES, algorithm)),
        ("Public key", hex::encode(&public)),
        ("Signature", hex::encode(&signature)),
    ]);
    if made {
        out = out.facts(&[("Secret key", hex::encode(secret))]);
    }
    Ok(out.note(if made {
        "The key pair was made just now from the demo clock. For anything real, use a key from a secure generator or a wallet."
    } else {
        "Keep the secret key private: anyone with it can sign as you."
    }))
}

fn verify(text: &str, o: &mut Opts) -> Result<Output, String> {
    let algorithm = o.mode;
    let public = hex::decode(o.field(0).trim().trim_start_matches("0x"))
        .map_err(|e| format!("The public key is not hex: {e}."))?;
    let signature = hex::decode(o.field(1).trim().trim_start_matches("0x"))
        .map_err(|e| format!("The signature is not hex: {e}."))?;
    if public.is_empty() || signature.is_empty() {
        return Err("Fill in both the public key and the signature.".into());
    }
    let ok = verify_with(algorithm, &public, &signature, text.as_bytes())?;
    Ok(verdict(
        ok,
        if ok {
            "The signature is valid for this message and public key."
        } else {
            "The signature does not match this message and public key."
        },
    )
    .facts(&[("Algorithm", label(SIGNATURES, algorithm))]))
}

fn keypair(_: &str, o: &mut Opts) -> Result<Output, String> {
    let algorithm = o.mode;
    let secret = new_secret(algorithm, o.rng);
    let (public, _) = sign_with(algorithm, &secret, b"")?;
    Ok(super::table(&[
        ("Algorithm", label(SIGNATURES, algorithm)),
        ("Secret key", hex::encode(secret)),
        ("Public key", hex::encode(public)),
    ])
    .note(
        "The key pair comes from the demo clock, not from the operating system's secure generator.",
    ))
}

// Shamir secret sharing

fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut product = 0;
    for _ in 0..8 {
        if b & 1 != 0 {
            product ^= a;
        }
        let high = a & 0x80;
        a <<= 1;
        if high != 0 {
            a ^= 0x1b;
        }
        b >>= 1;
    }
    product
}

fn gf_pow(mut a: u8, mut n: u8) -> u8 {
    let mut result = 1;
    while n > 0 {
        if n & 1 != 0 {
            result = gf_mul(result, a);
        }
        a = gf_mul(a, a);
        n >>= 1;
    }
    result
}

fn gf_div(a: u8, b: u8) -> u8 {
    gf_mul(a, gf_pow(b, 254))
}

fn split_secret(text: &str, threshold: usize, count: usize, rng: &mut Rng) -> Vec<String> {
    let mut secret = text.as_bytes().to_vec();
    secret.extend_from_slice(&Sha256::digest(text.as_bytes())[..8]);
    let mut shares = vec![Vec::with_capacity(secret.len()); count];
    for byte in secret {
        let mut coefficients = Vec::with_capacity(threshold);
        coefficients.push(byte);
        for _ in 1..threshold {
            let mut random = [0u8; 1];
            fill(rng, &mut random);
            coefficients.push(random[0]);
        }
        for (i, share) in shares.iter_mut().enumerate() {
            let x = (i + 1) as u8;
            let mut y = 0;
            for coefficient in coefficients.iter().rev() {
                y = gf_mul(y, x) ^ coefficient;
            }
            share.push(y);
        }
    }
    shares
        .iter()
        .enumerate()
        .map(|(i, share)| {
            let x = (i + 1) as u8;
            let mut blob = Vec::with_capacity(share.len() + 1);
            blob.push(x);
            blob.extend_from_slice(share);
            format!("{x}:{}", STANDARD.encode(blob))
        })
        .collect()
}

/// The points of a share line: its x (1–255) and one byte per secret byte.
fn parse_share(line: &str) -> Result<(u8, Vec<u8>), String> {
    let line = line.trim();
    let (prefix, encoded) = match line.split_once(':') {
        Some((prefix, rest)) => (Some(prefix.trim()), rest.trim()),
        None => (None, line),
    };
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|e| format!("A share is not Base64: {e}."))?;
    if bytes.is_empty() {
        return Err("A share is empty.".into());
    }
    let (x, values) = match prefix {
        Some(prefix) => {
            let x = prefix
                .parse::<u8>()
                .map_err(|_| format!("“{prefix}” is not a share number."))?;
            if bytes.first() != Some(&x) {
                return Err(format!("Share {x} does not carry its number."));
            }
            (x, bytes[1..].to_vec())
        }
        // A bare Base64 share has its number as its first byte.
        None => (bytes[0], bytes[1..].to_vec()),
    };
    if x == 0 {
        return Err("A share number must be between 1 and 255.".into());
    }
    Ok((x, values))
}

fn combine_secret(text: &str) -> Result<String, String> {
    let parsed: Vec<(u8, Vec<u8>)> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(parse_share)
        .collect::<Result<_, _>>()?;
    if parsed.is_empty() {
        return Err("Paste at least one share, one per line.".into());
    }
    let len = parsed[0].1.len();
    if parsed.iter().any(|(_, y)| y.len() != len) {
        return Err(
            "The shares are of different lengths; are they all from the same split?".into(),
        );
    }
    if parsed
        .iter()
        .map(|(x, _)| x)
        .collect::<std::collections::HashSet<_>>()
        .len()
        != parsed.len()
    {
        return Err("Two shares have the same number.".into());
    }
    let mut secret = vec![0u8; len];
    for (j, byte) in secret.iter_mut().enumerate() {
        let mut value = 0u8;
        for (i, (x, y)) in parsed.iter().enumerate() {
            let mut basis = 1u8;
            for (m, (other, _)) in parsed.iter().enumerate() {
                if m != i {
                    basis = gf_mul(basis, gf_div(*other, *other ^ *x));
                }
            }
            value ^= gf_mul(y[j], basis);
        }
        *byte = value;
    }
    if secret.len() < 8
        || secret[secret.len() - 8..] != Sha256::digest(&secret[..secret.len() - 8])[..8]
    {
        return Err("These shares do not rebuild a secret. There may be too few of them, or one is from another split.".into());
    }
    secret.truncate(secret.len() - 8);
    String::from_utf8(secret)
        .map_err(|_| "The shares rebuild bytes that are not UTF-8 text.".into())
}

fn shamir_split(text: &str, o: &mut Opts) -> Result<Output, String> {
    let (threshold, count) = match o.mode {
        "3-of-5" => (3, 5),
        "2-of-5" => (2, 5),
        "3-of-3" => (3, 3),
        "4-of-7" => (4, 7),
        _ => (2, 3),
    };
    let shares = split_secret(text, threshold, count, o.rng);
    Ok(Output::default()
        .code(shares.join("\n"))
        .facts(&[("Scheme", format!("{threshold} of {count}")), ("Shares", count.to_string())])
        .note(format!("Any {threshold} of the {count} shares rebuild the text; fewer reveal nothing. Keep them apart.")))
}

fn shamir_combine(text: &str, _: &mut Opts) -> Result<Output, String> {
    let secret = combine_secret(text)?;
    Ok(Output::default()
        .code(secret)
        .note("Rebuilt from the shares."))
}

pub const SECURITY: &[Tool] = &[
    tool(
        "encrypt",
        "Encrypt or decrypt",
        "Encrypts text with a password: AES-CBC, AES-GCM, ChaCha20 or ChaCha20-Poly1305. The envelope is Base64.",
        cipher,
    )
    .text("Plain text, or an encrypted envelope")
    .fields(&[("Password", "")])
    .selects(&[("Method", CIPHERS), ("Direction", DIRECTIONS)]),
    tool(
        "sign",
        "Sign a message",
        "Signs text with ECDSA secp256k1, Schnorr, Ed25519 or Ristretto. Leave the secret key empty to make a key pair.",
        sign,
    )
    .text("Message")
    .fields(&[("Secret key (hex, blank to make one)", "")])
    .selects(&[("Algorithm", SIGNATURES)]),
    tool(
        "verify-signature",
        "Verify a signature",
        "Checks a signature against a public key and a message.",
        verify,
    )
    .text("Message")
    .fields(&[("Public key (hex)", ""), ("Signature (hex)", "")])
    .selects(&[("Algorithm", SIGNATURES)]),
    tool(
        "keypair",
        "Key pair generator",
        "Makes a signing key pair for ECDSA, Schnorr, Ed25519 or Ristretto.",
        keypair,
    )
    .form()
    .selects(&[("Algorithm", SIGNATURES)]),
    tool(
        "shamir-split",
        "Shamir split a secret",
        "Splits text into shares: any 2 of 3 or 3 of 5 (and other schemes) rebuild it.",
        shamir_split,
    )
    .text("Secret message")
    .selects(&[("Shares", SHARES)]),
    tool(
        "shamir-combine",
        "Shamir combine shares",
        "Rebuilds a secret from enough of its shares, one per line.",
        shamir_combine,
    )
    .text("One share per line"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ciphers_round_trip() {
        let mut rng = Rng(7);
        for (algorithm, _) in CIPHERS {
            let secret = "Hello, secret world! Ünïcödé.";
            let wrapped = encrypt_text(algorithm, "correct horse", secret, &mut rng).unwrap();
            assert_eq!(
                decrypt_text(algorithm, "correct horse", &wrapped).unwrap(),
                secret,
                "{algorithm}"
            );
            assert!(
                decrypt_text(algorithm, "wrong horse", &wrapped).is_err(),
                "{algorithm}"
            );
        }
    }

    #[test]
    fn signatures_round_trip() {
        let mut rng = Rng(11);
        for (algorithm, _) in SIGNATURES {
            let secret = new_secret(algorithm, &mut rng);
            let (public, signature) = sign_with(algorithm, &secret, b"signed text").unwrap();
            assert!(
                verify_with(algorithm, &public, &signature, b"signed text").unwrap(),
                "{algorithm}"
            );
            assert!(
                !verify_with(algorithm, &public, &signature, b"other text").unwrap(),
                "{algorithm}"
            );
            assert_eq!(
                sign_with(algorithm, &secret, b"signed text").unwrap().1,
                signature,
                "{algorithm} is deterministic"
            );
        }
    }

    #[test]
    fn shamir_rebuilds_with_any_enough_shares() {
        let mut rng = Rng(13);
        let shares = split_secret("the treasure is under the third tree", 2, 3, &mut rng);
        let rebuilt = combine_secret(&shares[..2].join("\n")).unwrap();
        assert_eq!(rebuilt, "the treasure is under the third tree");
        assert!(combine_secret(&shares[..1].join("\n")).is_err());
        assert_eq!(combine_secret(&shares.join("\n")).unwrap(), rebuilt);
    }

    #[test]
    fn galois_field_arithmetic() {
        for a in 1..=255u8 {
            assert_eq!(gf_div(gf_mul(a, 37), 37), a);
        }
    }
}
