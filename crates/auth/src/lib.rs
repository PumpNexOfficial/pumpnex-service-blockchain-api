/// Authentication utilities for Solana wallet verification

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid base58 encoding: {0}")]
    InvalidBase58(String),
    #[error("Invalid public key length: expected 32 bytes, got {0}")]
    InvalidPubkeyLength(usize),
    #[error("Invalid signature length: expected 64 bytes, got {0}")]
    InvalidSignatureLength(usize),
    #[error("Signature verification failed")]
    VerificationFailed,
    #[error("Invalid public key: {0}")]
    InvalidPubkey(String),
}

/// Decode Solana public key from base58 string
pub fn decode_pubkey_b58(addr: &str) -> Result<[u8; 32], AuthError> {
    let bytes = bs58::decode(addr)
        .into_vec()
        .map_err(|e| AuthError::InvalidBase58(e.to_string()))?;
    
    if bytes.len() != 32 {
        return Err(AuthError::InvalidPubkeyLength(bytes.len()));
    }
    
    let mut result = [0u8; 32];
    result.copy_from_slice(&bytes);
    Ok(result)
}

/// Decode signature from base58 string
pub fn decode_sig_b58(sig: &str) -> Result<[u8; 64], AuthError> {
    let bytes = bs58::decode(sig)
        .into_vec()
        .map_err(|e| AuthError::InvalidBase58(e.to_string()))?;
    
    if bytes.len() != 64 {
        return Err(AuthError::InvalidSignatureLength(bytes.len()));
    }
    
    let mut result = [0u8; 64];
    result.copy_from_slice(&bytes);
    Ok(result)
}

/// Decode signature from base64 string
#[allow(dead_code)]
pub fn decode_sig_b64(sig: &str) -> Result<[u8; 64], AuthError> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(sig)
        .map_err(|e| AuthError::InvalidBase58(format!("base64 decode error: {}", e)))?;
    
    if bytes.len() != 64 {
        return Err(AuthError::InvalidSignatureLength(bytes.len()));
    }
    
    let mut result = [0u8; 64];
    result.copy_from_slice(&bytes);
    Ok(result)
}

/// Build the canonical signing string
pub fn build_signing_string(
    method: &str,
    path_qs: &str,
    nonce: &str,
    canon_method: &str,
    canon_path: &str,
) -> String {
    let canonical_method = match canon_method {
        "upper" => method.to_uppercase(),
        "lower" => method.to_lowercase(),
        _ => method.to_string(),
    };
    
    let canonical_path = match canon_path {
        "lower" => path_qs.to_lowercase(),
        _ => path_qs.to_string(), // "as-is"
    };
    
    format!("{}\n{}\n{}", canonical_method, canonical_path, nonce)
}

/// Verify Ed25519 signature
pub fn verify_ed25519(pubkey: &[u8; 32], message: &[u8], sig: &[u8; 64]) -> Result<bool, AuthError> {
    let verifying_key = VerifyingKey::from_bytes(pubkey)
        .map_err(|e| AuthError::InvalidPubkey(e.to_string()))?;
    
    let signature = Signature::from_bytes(sig);
    
    match verifying_key.verify(message, &signature) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Generate a random nonce as base58 string
pub fn generate_nonce() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_bytes: [u8; 16] = rng.gen();
    bs58::encode(random_bytes).into_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_pubkey() {
        // Valid Solana pubkey (32 bytes)
        let pubkey = "11111111111111111111111111111111";
        let result = decode_pubkey_b58(pubkey);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_signing_string() {
        let msg = build_signing_string("GET", "/api/test?foo=bar", "nonce123", "upper", "as-is");
        assert_eq!(msg, "GET\n/api/test?foo=bar\nnonce123");
    }
}
