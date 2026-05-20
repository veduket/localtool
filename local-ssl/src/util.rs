use base64::Engine;

pub fn pem_decode(pem: &str) -> Result<Vec<u8>, String> {
    let mut b64 = String::new();
    for line in pem.lines() {
        if line.starts_with("-----") {
            continue;
        }
        b64.push_str(line.trim());
    }
    let engine = base64::engine::general_purpose::STANDARD;
    engine
        .decode(b64.as_bytes())
        .map_err(|e| format!("Base64 decode error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pem_decode_valid() {
        let data = "SGVsbG8gV29ybGQ=";
        let pem = format!(
            "-----BEGIN CERTIFICATE-----\n{data}\n-----END CERTIFICATE-----"
        );
        let result = pem_decode(&pem).unwrap();
        assert_eq!(result, b"Hello World");
    }

    #[test]
    fn test_pem_decode_no_headers() {
        let result = pem_decode("SGVsbG8gV29ybGQ=").unwrap();
        assert_eq!(result, b"Hello World");
    }

    #[test]
    fn test_pem_decode_invalid_base64() {
        let pem = "\
-----BEGIN CERTIFICATE-----\n\
!!!invalid base64!!!\n\
-----END CERTIFICATE-----";
        let result = pem_decode(pem);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Base64 decode error"));
    }

    #[test]
    fn test_pem_decode_empty() {
        let result = pem_decode("");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_pem_decode_only_headers() {
        let pem = "\
-----BEGIN CERTIFICATE-----\n\
-----END CERTIFICATE-----";
        let result = pem_decode(pem).unwrap();
        assert!(result.is_empty());
    }
}
