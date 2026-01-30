use base64::prelude::*;
use rsa::pkcs1v15::{Signature, SigningKey, VerifyingKey};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePublicKey};
use rsa::signature::{SignatureEncoding, Signer, Verifier};
use rsa::{RsaPrivateKey, RsaPublicKey};
use sha2::Sha256;

use crate::error::S2SError;

/// Generate a 2048-bit RSA keypair for HTTP Signatures.
pub fn generate_rsa_keypair() -> Result<(RsaPrivateKey, RsaPublicKey), S2SError> {
    let mut rng = rand::thread_rng();
    let private_key = RsaPrivateKey::new(&mut rng, 2048)
        .map_err(|e| S2SError::Crypto(format!("key generation: {e}")))?;
    let public_key = RsaPublicKey::from(&private_key);
    Ok((private_key, public_key))
}

/// Export an RSA public key as a PEM string (SPKI / SubjectPublicKeyInfo).
pub fn public_key_to_pem(key: &RsaPublicKey) -> Result<String, S2SError> {
    key.to_public_key_pem(rsa::pkcs8::LineEnding::Lf)
        .map_err(|e| S2SError::Crypto(format!("PEM export: {e}")))
}

/// Export an RSA private key as a PEM string (PKCS#8).
pub fn private_key_to_pem(key: &RsaPrivateKey) -> Result<String, S2SError> {
    use rsa::pkcs8::EncodePrivateKey;
    key.to_pkcs8_pem(rsa::pkcs8::LineEnding::Lf)
        .map(|s| s.to_string())
        .map_err(|e| S2SError::Crypto(format!("private PEM export: {e}")))
}

/// Parse an RSA public key from a PEM string.
pub fn public_key_from_pem(pem: &str) -> Result<RsaPublicKey, S2SError> {
    RsaPublicKey::from_public_key_pem(pem)
        .map_err(|e| S2SError::Crypto(format!("PEM import: {e}")))
}

/// Parse an RSA private key from a PKCS#8 PEM string.
pub fn private_key_from_pem(pem: &str) -> Result<RsaPrivateKey, S2SError> {
    RsaPrivateKey::from_pkcs8_pem(pem)
        .map_err(|e| S2SError::Crypto(format!("private PEM import: {e}")))
}

/// Build the HTTP Signature signing string per draft-cavage-http-signatures.
///
/// `method` and `path` are used for the `(request-target)` pseudo-header.
/// `headers` is a list of `(name, value)` pairs from the HTTP request.
/// `signed_headers` lists which header names to include, in order.
pub fn build_signing_string(
    method: &str,
    path: &str,
    headers: &[(String, String)],
    signed_headers: &[&str],
) -> String {
    let mut lines = Vec::new();
    for name in signed_headers {
        if *name == "(request-target)" {
            lines.push(format!(
                "(request-target): {} {}",
                method.to_lowercase(),
                path
            ));
        } else {
            let lower = name.to_lowercase();
            if let Some((_, value)) = headers.iter().find(|(k, _)| k.to_lowercase() == lower) {
                lines.push(format!("{}: {}", lower, value));
            }
        }
    }
    lines.join("\n")
}

/// Sign a string with RSA-SHA256 (PKCS#1 v1.5), returning a base64-encoded signature.
pub fn sign(signing_string: &str, private_key: &RsaPrivateKey) -> Result<String, S2SError> {
    let signing_key = SigningKey::<Sha256>::new(private_key.clone());
    let signature: Signature = signing_key.sign(signing_string.as_bytes());
    Ok(BASE64_STANDARD.encode(signature.to_bytes()))
}

/// Verify an RSA-SHA256 signature given the signing string, base64-encoded signature,
/// and PEM-encoded public key.
pub fn verify(
    signing_string: &str,
    signature_b64: &str,
    public_key_pem: &str,
) -> Result<bool, S2SError> {
    let public_key = public_key_from_pem(public_key_pem)?;
    let verifying_key = VerifyingKey::<Sha256>::new(public_key);
    let sig_bytes = BASE64_STANDARD
        .decode(signature_b64)
        .map_err(|e| S2SError::Crypto(format!("base64 decode: {e}")))?;
    let signature = Signature::try_from(sig_bytes.as_slice())
        .map_err(|e| S2SError::Crypto(format!("invalid signature: {e}")))?;
    Ok(verifying_key.verify(signing_string.as_bytes(), &signature).is_ok())
}

/// Compute the SHA-256 digest of a body, formatted as `SHA-256=base64...`.
pub fn compute_digest(body: &[u8]) -> String {
    use sha2::Digest;
    let hash = Sha256::digest(body);
    format!("SHA-256={}", BASE64_STANDARD.encode(hash))
}

/// Format an HTTP Signature header value.
pub fn build_signature_header(
    key_id: &str,
    signed_headers: &[&str],
    signature_b64: &str,
) -> String {
    format!(
        r#"keyId="{}",algorithm="rsa-sha256",headers="{}",signature="{}""#,
        key_id,
        signed_headers.join(" "),
        signature_b64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let (_, public_key) = generate_rsa_keypair().unwrap();
        let pem = public_key_to_pem(&public_key).unwrap();
        assert!(pem.contains("BEGIN PUBLIC KEY"));
        assert!(pem.contains("END PUBLIC KEY"));
    }

    #[test]
    fn test_sign_verify_roundtrip() {
        let (private_key, public_key) = generate_rsa_keypair().unwrap();
        let pem = public_key_to_pem(&public_key).unwrap();
        let signing_string = "(request-target): post /users/bob/inbox\nhost: example.com";
        let sig = sign(signing_string, &private_key).unwrap();
        assert!(verify(signing_string, &sig, &pem).unwrap());
    }

    #[test]
    fn test_verify_wrong_key_fails() {
        let (key_a, _) = generate_rsa_keypair().unwrap();
        let (_, pub_b) = generate_rsa_keypair().unwrap();
        let pem_b = public_key_to_pem(&pub_b).unwrap();
        let sig = sign("test data", &key_a).unwrap();
        assert!(!verify("test data", &sig, &pem_b).unwrap());
    }

    #[test]
    fn test_signing_string_construction() {
        let headers = vec![
            ("Host".to_string(), "example.com".to_string()),
            ("Date".to_string(), "Fri, 30 Jan 2026 12:00:00 GMT".to_string()),
            ("Digest".to_string(), "SHA-256=abc123".to_string()),
        ];
        let result = build_signing_string(
            "POST",
            "/users/bob/inbox",
            &headers,
            &["(request-target)", "host", "date", "digest"],
        );
        let expected = "(request-target): post /users/bob/inbox\n\
                        host: example.com\n\
                        date: Fri, 30 Jan 2026 12:00:00 GMT\n\
                        digest: SHA-256=abc123";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_digest_computation() {
        let digest = compute_digest(b"hello world");
        assert!(digest.starts_with("SHA-256="));
        // SHA-256 of "hello world" is known
        assert_eq!(digest, "SHA-256=uU0nuZNNPgilLlLX2n2r+sSE7+N6U4DukIj3rOLvzek=");
    }

    #[test]
    fn test_signature_header_formatting() {
        let header = build_signature_header(
            "https://example.com/users/alice#main-key",
            &["(request-target)", "host", "date", "digest"],
            "abc123sig==",
        );
        assert!(header.contains(r#"keyId="https://example.com/users/alice#main-key""#));
        assert!(header.contains(r#"algorithm="rsa-sha256""#));
        assert!(header.contains(r#"headers="(request-target) host date digest""#));
        assert!(header.contains(r#"signature="abc123sig==""#));
    }

    #[test]
    fn test_private_key_pem_roundtrip() {
        let (private_key, _) = generate_rsa_keypair().unwrap();
        let pem = private_key_to_pem(&private_key).unwrap();
        assert!(pem.contains("BEGIN PRIVATE KEY"));
        let restored = private_key_from_pem(&pem).unwrap();
        // Sign with original, verify with restored's public key
        let pub_key = RsaPublicKey::from(&restored);
        let pub_pem = public_key_to_pem(&pub_key).unwrap();
        let sig = sign("test", &private_key).unwrap();
        assert!(verify("test", &sig, &pub_pem).unwrap());
    }
}
