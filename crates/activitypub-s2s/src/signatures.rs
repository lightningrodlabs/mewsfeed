use serde::{Deserialize, Serialize};

/// Parsed components of an HTTP Signature header.
///
/// HTTP Signatures (draft-cavage-http-signatures) are used by ActivityPub
/// servers to authenticate requests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignatureHeaders {
    /// The key ID URI (points to the actor's publicKey).
    pub key_id: String,

    /// The signing algorithm (typically "rsa-sha256").
    #[serde(default = "default_algorithm")]
    pub algorithm: String,

    /// Space-separated list of headers included in the signature.
    pub headers: String,

    /// The base64-encoded signature value.
    pub signature: String,
}

fn default_algorithm() -> String {
    "rsa-sha256".to_string()
}

impl SignatureHeaders {
    /// Parse an HTTP Signature header string into its components.
    ///
    /// Input format:
    /// `keyId="...",algorithm="...",headers="...",signature="..."`
    pub fn parse(header: &str) -> Result<Self, SignatureParseError> {
        let mut key_id = None;
        let mut algorithm = None;
        let mut headers = None;
        let mut signature = None;

        for part in split_signature_params(header) {
            let part = part.trim();
            if let Some((key, value)) = parse_key_value(part) {
                match key {
                    "keyId" => key_id = Some(value.to_string()),
                    "algorithm" => algorithm = Some(value.to_string()),
                    "headers" => headers = Some(value.to_string()),
                    "signature" => signature = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        Ok(SignatureHeaders {
            key_id: key_id.ok_or(SignatureParseError::MissingField("keyId"))?,
            algorithm: algorithm.unwrap_or_else(default_algorithm),
            headers: headers.ok_or(SignatureParseError::MissingField("headers"))?,
            signature: signature.ok_or(SignatureParseError::MissingField("signature"))?,
        })
    }

    /// Format as an HTTP Signature header value.
    pub fn to_header_string(&self) -> String {
        format!(
            r#"keyId="{}",algorithm="{}",headers="{}",signature="{}""#,
            self.key_id, self.algorithm, self.headers, self.signature
        )
    }
}

/// Errors from parsing an HTTP Signature header.
#[derive(Debug, Clone, PartialEq)]
pub enum SignatureParseError {
    MissingField(&'static str),
}

impl std::fmt::Display for SignatureParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "missing required field: {field}"),
        }
    }
}

impl std::error::Error for SignatureParseError {}

/// Split a signature header on commas, respecting quoted strings.
fn split_signature_params(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;

    for (i, c) in input.char_indices() {
        match c {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                parts.push(&input[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < input.len() {
        parts.push(&input[start..]);
    }
    parts
}

/// Parse a `key="value"` pair, stripping the quotes from value.
fn parse_key_value(input: &str) -> Option<(&str, &str)> {
    let eq_pos = input.find('=')?;
    let key = input[..eq_pos].trim();
    let value = input[eq_pos + 1..].trim();
    let value = value.strip_prefix('"').unwrap_or(value);
    let value = value.strip_suffix('"').unwrap_or(value);
    Some((key, value))
}
