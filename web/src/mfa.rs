//! Local TOTP and recovery-code primitives for optional account MFA.
//!
//! The caller owns persistence, rate limits, replay prevention, and session
//! transitions. TOTP uses the RFC 6238 default 30-second step and six digits.

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand_core::{OsRng, RngCore};
use ring::{aead, hmac, rand::SecureRandom};
use std::time::{SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

const TOTP_STEP_SECONDS: u64 = 30;
const TOTP_DIGITS: u32 = 6;
const TOTP_MODULUS: u32 = 10u32.pow(TOTP_DIGITS);
const TOTP_SECRET_BYTES: usize = 20;
const TOTP_AAD: &[u8] = b"zelyra:mfa:totp:v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MfaError {
    EntropyUnavailable,
    InvalidSecret,
    InvalidCode,
    InvalidEncryptionKey,
    EncryptionFailed,
    InvalidRecoveryCode,
}

impl std::fmt::Display for MfaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::EntropyUnavailable => "secure random source is unavailable",
            Self::InvalidSecret => "TOTP secret is invalid",
            Self::InvalidCode => "TOTP code is invalid",
            Self::InvalidEncryptionKey => "MFA encryption key is invalid",
            Self::EncryptionFailed => "MFA secret encryption failed",
            Self::InvalidRecoveryCode => "recovery code is invalid",
        })
    }
}

/// Generate a 160-bit TOTP secret in unpadded RFC 4648 Base32 form.
pub(crate) fn generate_totp_secret() -> Result<String, MfaError> {
    let mut secret = Zeroizing::new([0u8; TOTP_SECRET_BYTES]);
    OsRng
        .try_fill_bytes(secret.as_mut())
        .map_err(|_| MfaError::EntropyUnavailable)?;
    Ok(base32_encode(secret.as_ref()))
}

/// Return the current Unix time in seconds without panicking before the epoch.
pub(crate) fn unix_time_seconds() -> Result<u64, MfaError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| MfaError::InvalidCode)
}

/// Verify a six-digit code in the current, previous, or next 30-second step.
/// The returned counter must be committed atomically as the user's last
/// accepted counter; reject any counter less than or equal to that value.
pub(crate) fn matching_totp_step(
    encoded_secret: &str,
    supplied_code: &str,
    unix_seconds: u64,
) -> Result<Option<u64>, MfaError> {
    if supplied_code.len() != TOTP_DIGITS as usize
        || !supplied_code.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(MfaError::InvalidCode);
    }
    let secret = Zeroizing::new(base32_decode(encoded_secret)?);
    let current = unix_seconds / TOTP_STEP_SECONDS;
    let candidates = [
        Some(current),
        current.checked_sub(1),
        current.checked_add(1),
    ];
    for candidate in candidates.into_iter().flatten() {
        let expected = totp_for_step(secret.as_ref(), candidate)?;
        if bool::from(expected.as_bytes().ct_eq(supplied_code.as_bytes())) {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

/// Parse the operator-managed MFA encryption key from 64 hexadecimal digits.
pub(crate) fn encryption_key_from_hex(value: &str) -> Result<Zeroizing<[u8; 32]>, MfaError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(MfaError::InvalidEncryptionKey);
    }
    let mut key = Zeroizing::new([0u8; 32]);
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        key[index] = (hex_nibble(pair[0]).ok_or(MfaError::InvalidEncryptionKey)? << 4)
            | hex_nibble(pair[1]).ok_or(MfaError::InvalidEncryptionKey)?;
    }
    Ok(key)
}

/// Encrypt a Base32 secret with AES-256-GCM and return a hexadecimal envelope.
/// Each envelope begins with its fresh 96-bit nonce and includes the tag.
pub(crate) fn encrypt_totp_secret(
    key_bytes: &[u8; 32],
    encoded_secret: &str,
) -> Result<String, MfaError> {
    let _decoded_secret = base32_decode(encoded_secret)?;
    let key = aead::UnboundKey::new(&aead::AES_256_GCM, key_bytes)
        .map(aead::LessSafeKey::new)
        .map_err(|_| MfaError::InvalidEncryptionKey)?;
    let mut nonce_bytes = [0u8; 12];
    ring::rand::SystemRandom::new()
        .fill(&mut nonce_bytes)
        .map_err(|_| MfaError::EntropyUnavailable)?;
    let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);
    let mut ciphertext = Zeroizing::new(encoded_secret.as_bytes().to_vec());
    key.seal_in_place_append_tag(nonce, aead::Aad::from(TOTP_AAD), &mut *ciphertext)
        .map_err(|_| MfaError::EncryptionFailed)?;
    let mut envelope = Vec::with_capacity(nonce_bytes.len() + ciphertext.len());
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(ciphertext.as_ref());
    Ok(hex_encode(&envelope))
}

/// Authenticate and decrypt an encrypted Base32 TOTP secret.
pub(crate) fn decrypt_totp_secret(
    key_bytes: &[u8; 32],
    envelope_hex: &str,
) -> Result<Zeroizing<String>, MfaError> {
    let mut envelope = Zeroizing::new(hex_decode(envelope_hex)?);
    if envelope.len() < 12 + aead::AES_256_GCM.tag_len() + 1 {
        return Err(MfaError::EncryptionFailed);
    }
    let nonce_bytes: [u8; 12] = envelope[..12]
        .try_into()
        .map_err(|_| MfaError::EncryptionFailed)?;
    let nonce = aead::Nonce::assume_unique_for_key(nonce_bytes);
    let key = aead::UnboundKey::new(&aead::AES_256_GCM, key_bytes)
        .map(aead::LessSafeKey::new)
        .map_err(|_| MfaError::InvalidEncryptionKey)?;
    let plaintext = key
        .open_in_place(nonce, aead::Aad::from(TOTP_AAD), &mut envelope[12..])
        .map_err(|_| MfaError::EncryptionFailed)?;
    let secret = std::str::from_utf8(plaintext).map_err(|_| MfaError::EncryptionFailed)?;
    base32_decode(secret)?;
    Ok(Zeroizing::new(secret.to_owned()))
}

/// Generate ten 128-bit single-use recovery codes. Display them only once.
pub(crate) fn generate_recovery_codes() -> Result<Vec<Zeroizing<String>>, MfaError> {
    let mut codes = Vec::with_capacity(10);
    for _ in 0..10 {
        let mut random = Zeroizing::new([0u8; 16]);
        OsRng
            .try_fill_bytes(random.as_mut())
            .map_err(|_| MfaError::EntropyUnavailable)?;
        let encoded = hex_encode(random.as_ref());
        codes.push(Zeroizing::new(format!(
            "{}-{}-{}-{}",
            &encoded[..8],
            &encoded[8..16],
            &encoded[16..24],
            &encoded[24..32]
        )));
    }
    Ok(codes)
}

/// Hash one recovery code with Argon2id before database storage.
pub(crate) fn hash_recovery_code(code: &str) -> Result<String, MfaError> {
    let normalized = normalize_recovery_code(code)?;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(normalized.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| MfaError::InvalidRecoveryCode)
}

/// Verify a recovery code against its Argon2id verifier.
pub(crate) fn verify_recovery_code(code: &str, encoded_hash: &str) -> bool {
    let Ok(normalized) = normalize_recovery_code(code) else {
        return false;
    };
    let normalized = Zeroizing::new(normalized);
    PasswordHash::new(encoded_hash).is_ok_and(|hash| {
        Argon2::default()
            .verify_password(normalized.as_bytes(), &hash)
            .is_ok()
    })
}

fn totp_for_step(secret: &[u8], step: u64) -> Result<String, MfaError> {
    let key = hmac::Key::new(hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY, secret);
    let digest = hmac::sign(&key, &step.to_be_bytes());
    let bytes = digest.as_ref();
    let offset = (bytes[bytes.len() - 1] & 0x0f) as usize;
    let binary = ((u32::from(bytes[offset]) & 0x7f) << 24)
        | (u32::from(bytes[offset + 1]) << 16)
        | (u32::from(bytes[offset + 2]) << 8)
        | u32::from(bytes[offset + 3]);
    Ok(format!("{:06}", binary % TOTP_MODULUS))
}

fn normalize_recovery_code(code: &str) -> Result<Zeroizing<String>, MfaError> {
    let normalized = code
        .bytes()
        .filter(|byte| *byte != b'-')
        .map(|byte| byte.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if normalized.len() != 32 || !normalized.iter().all(u8::is_ascii_hexdigit) {
        return Err(MfaError::InvalidRecoveryCode);
    }
    String::from_utf8(normalized)
        .map(Zeroizing::new)
        .map_err(|_| MfaError::InvalidRecoveryCode)
}

fn base32_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut output = String::with_capacity(bytes.len().div_ceil(5) * 8);
    let mut buffer = 0u32;
    let mut bits = 0u8;
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            output.push(ALPHABET[((buffer >> bits) & 0x1f) as usize] as char);
        }
        buffer &= (1u32 << bits) - 1;
    }
    if bits > 0 {
        output.push(ALPHABET[((buffer << (5 - bits)) & 0x1f) as usize] as char);
    }
    output
}

fn base32_decode(value: &str) -> Result<Zeroizing<Vec<u8>>, MfaError> {
    if value.is_empty() || value.len() > 128 || value.contains('=') {
        return Err(MfaError::InvalidSecret);
    }
    let mut output = Zeroizing::new(Vec::with_capacity(value.len() * 5 / 8));
    let mut buffer = 0u32;
    let mut bits = 0u8;
    for byte in value.bytes() {
        let value = match byte.to_ascii_uppercase() {
            b'A'..=b'Z' => byte.to_ascii_uppercase() - b'A',
            b'2'..=b'7' => byte.to_ascii_uppercase() - b'2' + 26,
            _ => return Err(MfaError::InvalidSecret),
        };
        buffer = (buffer << 5) | u32::from(value);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            output.push(((buffer >> bits) & 0xff) as u8);
        }
        buffer &= (1u32 << bits) - 1;
    }
    if output.is_empty() || (bits > 0 && buffer & ((1u32 << bits) - 1) != 0) {
        return Err(MfaError::InvalidSecret);
    }
    Ok(output)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn hex_decode(value: &str) -> Result<Zeroizing<Vec<u8>>, MfaError> {
    if value.is_empty() || value.len() % 2 != 0 || value.len() > 1024 {
        return Err(MfaError::EncryptionFailed);
    }
    let mut output = Zeroizing::new(Vec::with_capacity(value.len() / 2));
    for pair in value.as_bytes().chunks_exact(2) {
        let high = hex_nibble(pair[0]).ok_or(MfaError::EncryptionFailed)?;
        let low = hex_nibble(pair[1]).ok_or(MfaError::EncryptionFailed)?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte.to_ascii_lowercase() {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte.to_ascii_lowercase() - b'a' + 10),
        _ => None,
    }
}
