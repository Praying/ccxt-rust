//! API authentication and cryptographic signing utilities.
//!
//! Provides cryptographic functions for exchange API authentication:
//! - HMAC signing (SHA256/SHA512/SHA384/MD5)
//! - Hash functions (SHA256/SHA512/SHA384/SHA1/MD5/Keccak)
//! - RSA signing (PKCS1v15)
//! - EdDSA signing (Ed25519)
//! - JWT token generation
//! - Base64 encoding/decoding utilities

use crate::error::{Error, Result};
use base64::{Engine as _, engine::general_purpose};
use hmac::{Hmac, Mac};
// use rsa::{
//     RsaPrivateKey,
//     pkcs1::DecodeRsaPrivateKey,
//     pkcs1v15::SigningKey,
//     signature::{RandomizedSigner, SignatureEncoding},
// };
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};
use sha3::Keccak256;
use std::fmt;

/// Supported cryptographic hash algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// SHA-1 hash algorithm
    Sha1,
    /// SHA-256 hash algorithm
    Sha256,
    /// SHA-384 hash algorithm
    Sha384,
    /// SHA-512 hash algorithm
    Sha512,
    /// MD5 hash algorithm
    Md5,
    /// Keccak-256 (SHA-3) hash algorithm
    Keccak,
}

impl HashAlgorithm {
    /// Parses a hash algorithm from a string.
    ///
    /// # Arguments
    /// * `s` - Algorithm name (case-insensitive)
    ///
    /// # Returns
    /// Parsed [`HashAlgorithm`] or error if unsupported.
    // Lint: should_implement_trait
    // Reason: This method returns Result<Self> with custom error type, not compatible with FromStr trait
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "sha1" => Ok(HashAlgorithm::Sha1),
            "sha256" => Ok(HashAlgorithm::Sha256),
            "sha384" => Ok(HashAlgorithm::Sha384),
            "sha512" => Ok(HashAlgorithm::Sha512),
            "md5" => Ok(HashAlgorithm::Md5),
            "keccak" | "sha3" => Ok(HashAlgorithm::Keccak),
            _ => Err(Error::invalid_argument(format!(
                "Unsupported hash algorithm: {s}"
            ))),
        }
    }
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            HashAlgorithm::Sha1 => "sha1",
            HashAlgorithm::Sha256 => "sha256",
            HashAlgorithm::Sha384 => "sha384",
            HashAlgorithm::Sha512 => "sha512",
            HashAlgorithm::Md5 => "md5",
            HashAlgorithm::Keccak => "keccak",
        };
        write!(f, "{s}")
    }
}

/// Output encoding format for cryptographic digests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestFormat {
    /// Hexadecimal encoding
    Hex,
    /// Base64 encoding
    Base64,
    /// Raw binary format
    Binary,
}

impl DigestFormat {
    /// Parses a digest format from a string.
    ///
    /// # Arguments
    /// * `s` - Format name (case-insensitive)
    ///
    /// # Returns
    /// Parsed [`DigestFormat`], defaults to `Hex` if unrecognized.
    // Lint: should_implement_trait
    // Reason: This method has different semantics than FromStr - it never fails and has a default
    #[allow(clippy::should_implement_trait)]
    // Lint: match_same_arms
    // Reason: Explicit "hex" match documents the supported format, wildcard is the fallback default
    #[allow(clippy::match_same_arms)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "hex" => DigestFormat::Hex,
            "base64" => DigestFormat::Base64,
            "binary" => DigestFormat::Binary,
            _ => DigestFormat::Hex,
        }
    }
}

/// Generates an HMAC signature for a message.
///
/// # Arguments
/// * `message` - Message to sign
/// * `secret` - Secret key for HMAC
/// * `algorithm` - Hash algorithm to use
/// * `digest` - Output encoding format
///
/// # Returns
/// Encoded signature string.
///
/// # Errors
/// Returns error if algorithm is not supported for HMAC.
///
/// # Examples
/// ```
/// use ccxt_core::auth::{hmac_sign, HashAlgorithm, DigestFormat};
///
/// let signature = hmac_sign(
///     "test message",
///     "secret_key",
///     HashAlgorithm::Sha256,
///     DigestFormat::Hex
/// ).unwrap();
/// ```
pub fn hmac_sign(
    message: &str,
    secret: &str,
    algorithm: HashAlgorithm,
    digest: DigestFormat,
) -> Result<String> {
    let signature = match algorithm {
        HashAlgorithm::Sha256 => hmac_sha256(message.as_bytes(), secret.as_bytes()),
        HashAlgorithm::Sha512 => hmac_sha512(message.as_bytes(), secret.as_bytes()),
        HashAlgorithm::Sha384 => hmac_sha384(message.as_bytes(), secret.as_bytes()),
        HashAlgorithm::Md5 => hmac_md5(message.as_bytes(), secret.as_bytes()),
        _ => {
            return Err(Error::invalid_argument(format!(
                "HMAC does not support {algorithm} algorithm"
            )));
        }
    };

    Ok(encode_bytes(&signature, digest))
}

/// Computes HMAC-SHA256 signature.
///
/// # Panics
///
/// This function will never panic. The `expect()` call is safe because HMAC algorithms
/// accept keys of any length (including empty keys). The `InvalidLength` error from
/// `new_from_slice` is only returned for algorithms with fixed key requirements,
/// which does not apply to HMAC.
fn hmac_sha256(data: &[u8], secret: &[u8]) -> Vec<u8> {
    type HmacSha256 = Hmac<Sha256>;
    // SAFETY: HMAC accepts keys of any length - this cannot fail
    let mut mac = HmacSha256::new_from_slice(secret)
        .expect("HMAC-SHA256 accepts keys of any length; this is an infallible operation");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Computes HMAC-SHA512 signature.
///
/// # Panics
///
/// This function will never panic. See `hmac_sha256` for rationale.
fn hmac_sha512(data: &[u8], secret: &[u8]) -> Vec<u8> {
    type HmacSha512 = Hmac<Sha512>;
    // SAFETY: HMAC accepts keys of any length - this cannot fail
    let mut mac = HmacSha512::new_from_slice(secret)
        .expect("HMAC-SHA512 accepts keys of any length; this is an infallible operation");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Computes HMAC-SHA384 signature.
///
/// # Panics
///
/// This function will never panic. See `hmac_sha256` for rationale.
fn hmac_sha384(data: &[u8], secret: &[u8]) -> Vec<u8> {
    type HmacSha384 = Hmac<Sha384>;
    // SAFETY: HMAC accepts keys of any length - this cannot fail
    let mut mac = HmacSha384::new_from_slice(secret)
        .expect("HMAC-SHA384 accepts keys of any length; this is an infallible operation");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Computes HMAC-MD5 signature.
///
/// # Panics
///
/// This function will never panic. See `hmac_sha256` for rationale.
fn hmac_md5(data: &[u8], secret: &[u8]) -> Vec<u8> {
    use md5::Md5;
    type HmacMd5 = Hmac<Md5>;
    // SAFETY: HMAC accepts keys of any length - this cannot fail
    let mut mac = HmacMd5::new_from_slice(secret)
        .expect("HMAC-MD5 accepts keys of any length; this is an infallible operation");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Computes a cryptographic hash (one-way, keyless).
///
/// # Arguments
/// * `data` - Data to hash
/// * `algorithm` - Hash algorithm to use
/// * `digest` - Output encoding format
///
/// # Returns
/// Encoded hash string.
///
/// # Examples
/// ```
/// use ccxt_core::auth::{hash, HashAlgorithm, DigestFormat};
///
/// let hashed = hash("test", HashAlgorithm::Sha256, DigestFormat::Hex).unwrap();
/// ```
pub fn hash(data: &str, algorithm: HashAlgorithm, digest: DigestFormat) -> Result<String> {
    let hash_bytes = match algorithm {
        HashAlgorithm::Sha256 => hash_sha256(data.as_bytes()),
        HashAlgorithm::Sha512 => hash_sha512(data.as_bytes()),
        HashAlgorithm::Sha384 => hash_sha384(data.as_bytes()),
        HashAlgorithm::Sha1 => hash_sha1(data.as_bytes()),
        HashAlgorithm::Md5 => hash_md5(data.as_bytes()),
        HashAlgorithm::Keccak => hash_keccak(data.as_bytes()),
    };

    Ok(encode_bytes(&hash_bytes, digest))
}

/// Computes SHA-256 hash.
fn hash_sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Computes SHA-512 hash.
fn hash_sha512(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha512::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Computes SHA-384 hash.
fn hash_sha384(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha384::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Computes SHA-1 hash.
fn hash_sha1(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Computes MD5 hash.
fn hash_md5(data: &[u8]) -> Vec<u8> {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Computes Keccak-256 hash (Ethereum-compatible).
fn hash_keccak(data: &[u8]) -> Vec<u8> {
    let mut hasher = Keccak256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

// /// Generates an RSA signature using PKCS1v15 padding.
// ///
// /// # Arguments
// /// * `data` - Data to sign
// /// * `private_key_pem` - RSA private key in PEM format
// /// * `algorithm` - Hash algorithm for digest
// ///
// /// # Returns
// /// Base64-encoded signature.
// ///
// /// # Errors
// /// Returns error if key is invalid or algorithm unsupported.
// pub fn rsa_sign(data: &str, private_key_pem: &str, algorithm: HashAlgorithm) -> Result<String> {
//     let private_key = RsaPrivateKey::from_pkcs1_pem(private_key_pem)
//         .map_err(|e| Error::invalid_argument(format!("Invalid RSA private key: {}", e)))?;
//
//     let hashed = match algorithm {
//         HashAlgorithm::Sha256 => hash_sha256(data.as_bytes()),
//         HashAlgorithm::Sha384 => hash_sha384(data.as_bytes()),
//         HashAlgorithm::Sha512 => hash_sha512(data.as_bytes()),
//         HashAlgorithm::Sha1 => hash_sha1(data.as_bytes()),
//         HashAlgorithm::Md5 => hash_md5(data.as_bytes()),
//         _ => {
//             return Err(Error::invalid_argument(format!(
//                 "RSA does not support {} algorithm",
//                 algorithm
//             )));
//         }
//     };
//
//     let signature = match algorithm {
//         HashAlgorithm::Sha256 => {
//             let signing_key = SigningKey::<Sha256>::new_unprefixed(private_key);
//             let mut rng = rand::rngs::ThreadRng::default();
//             signing_key.sign_with_rng(&mut rng, &hashed).to_bytes()
//         }
//         HashAlgorithm::Sha384 => {
//             let signing_key = SigningKey::<Sha384>::new_unprefixed(private_key);
//             let mut rng = rand::rngs::ThreadRng::default();
//             signing_key.sign_with_rng(&mut rng, &hashed).to_bytes()
//         }
//         HashAlgorithm::Sha512 => {
//             let signing_key = SigningKey::<Sha512>::new_unprefixed(private_key);
//             let mut rng = rand::rngs::ThreadRng::default();
//             signing_key.sign_with_rng(&mut rng, &hashed).to_bytes()
//         }
//         _ => {
//             return Err(Error::invalid_argument(format!(
//                 "Unsupported RSA hash algorithm: {}",
//                 algorithm
//             )));
//         }
//     };
//
//     Ok(general_purpose::STANDARD.encode(signature.as_ref()))
// }

/// Generates an EdDSA signature using Ed25519.
///
/// # Arguments
/// * `data` - Data to sign
/// * `secret_key` - 32-byte Ed25519 seed key
///
/// # Returns
/// Base64 URL-encoded signature (without padding).
///
/// # Errors
/// Returns error if secret key is not exactly 32 bytes.
pub fn eddsa_sign(data: &str, secret_key: &[u8]) -> Result<String> {
    use ed25519_dalek::{Signature, Signer, SigningKey};

    if secret_key.len() != 32 {
        return Err(Error::invalid_argument(format!(
            "Ed25519 secret key must be 32 bytes, got {}",
            secret_key.len()
        )));
    }

    let signing_key = SigningKey::from_bytes(
        secret_key
            .try_into()
            .map_err(|_| Error::invalid_argument("Invalid Ed25519 key".to_string()))?,
    );

    let signature: Signature = signing_key.sign(data.as_bytes());
    let encoded = general_purpose::STANDARD.encode(signature.to_bytes());

    Ok(base64_to_base64url(&encoded, true))
}

/// Generates a JWT (JSON Web Token).
///
/// # Arguments
/// * `payload` - JWT payload as JSON object
/// * `secret` - Secret key for signing
/// * `algorithm` - Hash algorithm for HMAC
/// * `header_options` - Optional additional header fields
///
/// # Returns
/// Complete JWT string (header.payload.signature).
///
/// # Errors
/// Returns error if JSON serialization or signing fails.
///
/// # Examples
/// ```
/// use ccxt_core::auth::{jwt_sign, HashAlgorithm};
/// use serde_json::json;
///
/// let payload = json!({
///     "user_id": "123",
///     "exp": 1234567890
/// });
///
/// let token = jwt_sign(&payload, "secret", HashAlgorithm::Sha256, None).unwrap();
/// ```
pub fn jwt_sign(
    payload: &serde_json::Value,
    secret: &str,
    algorithm: HashAlgorithm,
    header_options: Option<serde_json::Map<String, serde_json::Value>>,
) -> Result<String> {
    let mut header = serde_json::Map::new();
    header.insert(
        "alg".to_string(),
        serde_json::Value::String("HS256".to_string()),
    );
    header.insert(
        "typ".to_string(),
        serde_json::Value::String("JWT".to_string()),
    );

    if let Some(options) = header_options {
        for (key, value) in options {
            header.insert(key, value);
        }
    }

    let header_json = serde_json::to_string(&header)?;
    let payload_json = serde_json::to_string(payload)?;

    let encoded_header = general_purpose::URL_SAFE_NO_PAD.encode(header_json.as_bytes());
    let encoded_payload = general_purpose::URL_SAFE_NO_PAD.encode(payload_json.as_bytes());

    let token = format!("{encoded_header}.{encoded_payload}");

    let signature = hmac_sign(&token, secret, algorithm, DigestFormat::Base64)?;

    let signature_url = base64_to_base64url(&signature, true);

    Ok(format!("{token}.{signature_url}"))
}

/// Encodes bytes to the specified format.
fn encode_bytes(bytes: &[u8], format: DigestFormat) -> String {
    match format {
        DigestFormat::Hex => hex::encode(bytes),
        DigestFormat::Base64 => general_purpose::STANDARD.encode(bytes),
        DigestFormat::Binary => String::from_utf8_lossy(bytes).to_string(),
    }
}

/// Converts standard Base64 to Base64 URL format.
///
/// # Arguments
/// * `base64_str` - Standard Base64 string
/// * `strip_padding` - Whether to remove padding (`=`)
///
/// # Returns
/// Base64 URL-encoded string.
pub fn base64_to_base64url(base64_str: &str, strip_padding: bool) -> String {
    let mut result = base64_str.replace('+', "-").replace('/', "_");
    if strip_padding {
        result = result.trim_end_matches('=').to_string();
    }
    result
}

/// Decodes a Base64 URL-encoded string.
///
/// # Arguments
/// * `base64url` - Base64 URL-encoded string
///
/// # Returns
/// Decoded bytes.
///
/// # Errors
/// Returns error if decoding fails.
pub fn base64url_decode(base64url: &str) -> Result<Vec<u8>> {
    let base64 = base64url.replace('-', "+").replace('_', "/");

    let padding = match base64.len() % 4 {
        2 => "==",
        3 => "=",
        _ => "",
    };
    let base64_padded = format!("{base64}{padding}");

    general_purpose::STANDARD
        .decode(base64_padded.as_bytes())
        .map_err(|e| Error::invalid_argument(format!("Base64 decode error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sha256_hex() {
        let result = hmac_sign("test", "secret", HashAlgorithm::Sha256, DigestFormat::Hex).unwrap();
        assert_eq!(
            result,
            "0329a06b62cd16b33eb6792be8c60b158d89a2ee3a876fce9a881ebb488c0914"
        );
    }

    #[test]
    fn test_hmac_sha256_base64() {
        let result = hmac_sign(
            "test",
            "secret",
            HashAlgorithm::Sha256,
            DigestFormat::Base64,
        )
        .unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_hash_sha256() {
        let result = hash("test", HashAlgorithm::Sha256, DigestFormat::Hex).unwrap();
        assert_eq!(
            result,
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
    }

    #[test]
    fn test_hash_keccak() {
        let result = hash("test", HashAlgorithm::Keccak, DigestFormat::Hex).unwrap();
        assert_eq!(result.len(), 64); // Keccak256输出32字节=64个hex字符
    }

    #[test]
    fn test_base64_to_base64url() {
        let base64 = "abc+def/ghi==";
        let base64url = base64_to_base64url(base64, true);
        assert_eq!(base64url, "abc-def_ghi");
    }

    #[test]
    fn test_base64url_decode() {
        let base64url = "abc-def_ghg";
        let decoded = base64url_decode(base64url).unwrap();
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_jwt_sign() {
        use serde_json::json;

        let payload = json!({
            "user_id": "123",
            "exp": 1234567890
        });

        let token = jwt_sign(&payload, "secret", HashAlgorithm::Sha256, None).unwrap();

        // JWT应该有3部分，用.分隔
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_hash_algorithm_from_str() {
        assert_eq!(
            HashAlgorithm::from_str("sha256").unwrap(),
            HashAlgorithm::Sha256
        );
        assert_eq!(
            HashAlgorithm::from_str("SHA256").unwrap(),
            HashAlgorithm::Sha256
        );
        assert!(HashAlgorithm::from_str("invalid").is_err());
    }

    #[test]
    fn test_digest_format_from_str() {
        assert_eq!(DigestFormat::from_str("hex"), DigestFormat::Hex);
        assert_eq!(DigestFormat::from_str("base64"), DigestFormat::Base64);
        assert_eq!(DigestFormat::from_str("binary"), DigestFormat::Binary);
        assert_eq!(DigestFormat::from_str("unknown"), DigestFormat::Hex); // 默认
    }

    #[test]
    fn test_hmac_sha512() {
        let result = hmac_sign("test", "secret", HashAlgorithm::Sha512, DigestFormat::Hex).unwrap();
        assert_eq!(result.len(), 128); // SHA512输出64字节=128个hex字符
    }

    #[test]
    fn test_hash_md5() {
        let result = hash("test", HashAlgorithm::Md5, DigestFormat::Hex).unwrap();
        assert_eq!(result.len(), 32); // MD5输出16字节=32个hex字符
    }
}
