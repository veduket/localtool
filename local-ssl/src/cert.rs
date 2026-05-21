use colored::Colorize;
use rcgen::{CertificateParams, DnType, ExtendedKeyUsagePurpose, Issuer, KeyPair, KeyUsagePurpose};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use x509_parser::extensions::GeneralName;
use x509_parser::prelude::*;

use crate::ca::CaStore;
use crate::util;

pub struct CertBundle {
    #[allow(dead_code)]
    pub domain: String,
    pub cert_path: String,
    pub key_path: String,
}

struct ParsedCert {
    cn: String,
    not_before: ::time::OffsetDateTime,
    not_after: ::time::OffsetDateTime,
    cert_path: PathBuf,
    key_path: PathBuf,
}

pub fn generate(domain: &str, ca_store: &CaStore, sans: &[String]) -> Result<CertBundle, String> {
    let out_dir = ca_store.dir.join("certs").join(domain);
    fs::create_dir_all(&out_dir).map_err(|e| format!("Cannot create {out_dir:?}: {e}"))?;

    let key_pair = KeyPair::generate().map_err(|e| format!("Cannot generate key: {e}"))?;

    let mut alt_names = vec![domain.to_string()];
    for san in sans {
        if san != domain {
            alt_names.push(san.clone());
        }
    }
    if !domain.starts_with("*.") {
        alt_names.push(format!("*.{domain}"));
    }

    let mut params =
        CertificateParams::new(alt_names).map_err(|e| format!("Cannot create params: {e}"))?;
    params.distinguished_name.push(DnType::CommonName, domain);
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyEncipherment,
    ];
    params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ServerAuth,
        ExtendedKeyUsagePurpose::ClientAuth,
    ];
    params.not_before = ::time::OffsetDateTime::now_utc();
    params.not_after = ::time::OffsetDateTime::now_utc() + Duration::from_secs(365 * 86400);

    let ca_key = ca_store.load_key()?;
    let ca_params = crate::ca::CaStore::ca_params();
    let issuer = Issuer::new(ca_params, &ca_key);

    let cert = params
        .signed_by(&key_pair, &issuer)
        .map_err(|e| format!("Cannot sign cert: {e}"))?;

    let cert_path = out_dir.join("cert.pem");
    let key_path = out_dir.join("key.pem");

    fs::write(&cert_path, cert.pem()).map_err(|e| format!("Cannot write cert: {e}"))?;
    fs::write(&key_path, key_pair.serialize_pem()).map_err(|e| format!("Cannot write key: {e}"))?;

    Ok(CertBundle {
        domain: domain.to_string(),
        cert_path: cert_path.to_string_lossy().to_string(),
        key_path: key_path.to_string_lossy().to_string(),
    })
}

pub fn list(ca_store: &CaStore) -> Result<Vec<String>, String> {
    let certs_dir = ca_store.dir.join("certs");
    if !certs_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut domains = Vec::new();
    for entry in fs::read_dir(&certs_dir).map_err(|e| format!("{e}"))? {
        let entry = entry.map_err(|e| format!("{e}"))?;
        if entry.path().is_dir() && entry.path().join("cert.pem").exists() {
            domains.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    domains.sort();
    Ok(domains)
}

fn parse_local_cert(domain: &str, ca_store: &CaStore) -> Result<ParsedCert, String> {
    let cert_path = ca_store.dir.join("certs").join(domain).join("cert.pem");
    if !cert_path.exists() {
        return Err(format!("No certificate for '{domain}'"));
    }

    let pem_data = fs::read_to_string(&cert_path).map_err(|e| format!("Cannot read cert: {e}"))?;
    let der = util::pem_decode(&pem_data)?;
    let (_, parsed) =
        X509Certificate::from_der(&der).map_err(|e| format!("Cannot parse cert: {e}"))?;

    let cn = parsed
        .subject()
        .iter_common_name()
        .next()
        .and_then(|a| a.as_str().ok())
        .unwrap_or("(unknown)")
        .to_string();

    Ok(ParsedCert {
        cn,
        not_before: parsed.validity().not_before.to_datetime(),
        not_after: parsed.validity().not_after.to_datetime(),
        cert_path: cert_path.clone(),
        key_path: ca_store.dir.join("certs").join(domain).join("key.pem"),
    })
}

pub fn show(domain: &str, ca_store: &CaStore) -> Result<String, String> {
    let p = parse_local_cert(domain, ca_store)?;

    let issuer_str = {
        let pem_data =
            fs::read_to_string(&p.cert_path).map_err(|e| format!("Cannot read cert: {e}"))?;
        let der = util::pem_decode(&pem_data)?;
        let (_, parsed) =
            X509Certificate::from_der(&der).map_err(|e| format!("Cannot parse cert: {e}"))?;
        let out = parsed
            .issuer()
            .iter_common_name()
            .next()
            .and_then(|a| a.as_str().ok())
            .unwrap_or("(unknown)")
            .to_string();
        out
    };

    let sans: Vec<String> = {
        let pem_data =
            fs::read_to_string(&p.cert_path).map_err(|e| format!("Cannot read cert: {e}"))?;
        let der = util::pem_decode(&pem_data)?;
        let (_, parsed) =
            X509Certificate::from_der(&der).map_err(|e| format!("Cannot parse cert: {e}"))?;
        parsed
            .subject_alternative_name()
            .map_err(|e| format!("Cannot parse SANs: {e}"))?
            .map(|ext| {
                ext.value
                    .general_names
                    .iter()
                    .filter_map(|gn| {
                        if let GeneralName::DNSName(s) = gn {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    let mut out = format!("Domain:        {}\n", p.cn);
    out.push_str(&format!("Issuer:        {issuer_str}\n"));
    out.push_str(&format!("Valid from:    {}\n", p.not_before));
    out.push_str(&format!("Valid until:   {}\n", p.not_after));
    out.push_str(&format!("SANs:          {}\n", sans.join(", ")));
    out.push_str(&format!("Cert:          {}\n", p.cert_path.display()));
    out.push_str(&format!("Key:           {}\n", p.key_path.display()));

    Ok(out)
}

pub fn check_local(domain: &str, ca_store: &CaStore) -> Result<String, String> {
    let p = parse_local_cert(domain, ca_store)?;
    let now = ::time::OffsetDateTime::now_utc();

    let validity_str = if now < p.not_before {
        format!("{}", "not yet valid".red().bold())
    } else if now > p.not_after {
        format!("{}", "expired".red().bold())
    } else {
        let days_left = (p.not_after - now).whole_days();
        if days_left < 30 {
            format!("{} ({days_left} days remaining)", "valid".yellow())
        } else {
            format!("{} ({days_left} days remaining)", "valid".green())
        }
    };

    let gen_date = fs::metadata(&p.cert_path)
        .ok()
        .and_then(|m| m.created().or(m.modified()).ok())
        .map(|t| {
            let dt: ::time::OffsetDateTime = t.into();
            dt.to_string()
        })
        .unwrap_or_else(|| "(unknown)".to_string());

    let key_ok = p.key_path.exists();

    let mut out = format!("{} {}\n", "Domain:".cyan().bold(), p.cn.green());
    out.push_str(&format!("{} {}\n", "Validity:".bold(), validity_str));
    out.push_str(&format!("{} {}\n", "Generated:".bold(), gen_date));
    out.push_str(&format!("{} {}\n", "Expires:".bold(), p.not_after));
    let key_status = if key_ok { "present".green() } else { "missing".red() };
    out.push_str(&format!("{} {}\n", "Key file:".bold(), key_status));
    out.push_str(&format!(
        "{} {}\n",
        "Reload needed:".bold(),
        "no (certs are file-based, restart your server)".green()
    ));

    Ok(out)
}

pub fn check_remote(host: &str, port: u16) -> Result<String, String> {
    let addr = format!("{host}:{port}");
    let mut tcp = std::net::TcpStream::connect(&addr)
        .map_err(|e| format!("Cannot connect to {addr}: {e}"))?;

    let server_name = rustls::pki_types::ServerName::try_from(host)
        .map_err(|_| format!("Invalid hostname: {host}"))?
        .to_owned();

    let root_store =
        rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let mut conn = rustls::ClientConnection::new(std::sync::Arc::new(config), server_name)
        .map_err(|e| format!("TLS error: {e}"))?;

    let mut tls = rustls::Stream::new(&mut conn, &mut tcp);
    tls.flush()
        .map_err(|e| format!("TLS handshake error: {e}"))?;

    let certs = conn.peer_certificates().unwrap_or(&[]);
    if certs.is_empty() {
        return Err("No certificate presented by server".into());
    }

    let mut out = format!(
        "{} {}:{}\n",
        "Remote server:".cyan().bold(),
        host.cyan(),
        port.to_string().cyan()
    );
    out.push_str(&format!(
        "{} {}\n\n",
        "Certificate chain:".bold(),
        format!("{} certs", certs.len()).cyan()
    ));

    for (i, cert_der) in certs.iter().enumerate() {
        let (_, parsed) = X509Certificate::from_der(cert_der)
            .map_err(|e| format!("Cannot parse cert #{i}: {e}"))?;

        let cn = parsed
            .subject()
            .iter_common_name()
            .next()
            .and_then(|a| a.as_str().ok())
            .unwrap_or("(unknown)");

        let issuer = parsed
            .issuer()
            .iter_common_name()
            .next()
            .and_then(|a| a.as_str().ok())
            .unwrap_or("(unknown)");

        let not_before = parsed.validity().not_before.to_datetime();
        let not_after = parsed.validity().not_after.to_datetime();
        let now = ::time::OffsetDateTime::now_utc();

        let validity = if now < not_before {
            format!("{}", "not yet valid".red())
        } else if now > not_after {
            format!("{}", "expired".red().bold())
        } else {
            let days = (not_after - now).whole_days();
            format!("{} ({days} days)", "valid".green())
        };

        let sans: Vec<String> = parsed
            .subject_alternative_name()
            .ok()
            .flatten()
            .map(|ext| {
                ext.value
                    .general_names
                    .iter()
                    .filter_map(|gn| {
                        if let GeneralName::DNSName(s) = gn {
                            Some(s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        out.push_str(&format!(
            "  {} {} (index {i})\n",
            "Certificate".bold(),
            (i + 1).to_string().cyan()
        ));
        out.push_str(&format!("    {} {}\n", "Subject:".bold(), cn));
        out.push_str(&format!("    {} {}\n", "Issuer:".bold(), issuer));
        out.push_str(&format!("    {} {}\n", "Validity:".bold(), validity));
        out.push_str(&format!("    {} {}\n", "Valid from:".bold(), not_before));
        out.push_str(&format!("    {} {}\n", "Valid until:".bold(), not_after));
        if !sans.is_empty() {
            out.push_str(&format!("    {} {}\n", "SANs:".bold(), sans.join(", ")));
        }
        out.push('\n');
    }

    Ok(out.trim().to_string())
}

pub fn check_all_local(ca_store: &CaStore) -> Result<String, String> {
    let domains = list(ca_store)?;
    if domains.is_empty() {
        return Ok(format!("{}", "No certificates found.".yellow()));
    }

    let mut out = String::new();
    for d in &domains {
        let result = check_local(d, ca_store)?;
        out.push_str(&result);
        out.push('\n');
    }
    Ok(out.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ca::CaStore;
    use std::path::Path;
    use tempfile::tempdir;

    fn setup() -> (CaStore, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let store = CaStore {
            dir: dir.path().to_path_buf(),
            key_path: dir.path().join("ca-key.pem"),
            cert_path: dir.path().join("ca-cert.pem"),
        };
        store.init().unwrap();
        (store, dir)
    }

    #[test]
    fn test_generate_creates_files() {
        let (store, _dir) = setup();
        let bundle = generate("test.local", &store, &[]).unwrap();
        assert!(Path::new(&bundle.cert_path).exists());
        assert!(Path::new(&bundle.key_path).exists());
        assert_eq!(bundle.domain, "test.local");
    }

    #[test]
    fn test_generate_with_sans() {
        let (store, _dir) = setup();
        let _bundle = generate(
            "primary.test",
            &store,
            &["san1.test".to_string(), "san2.test".to_string()],
        )
        .unwrap();
        let info = show("primary.test", &store).unwrap();
        assert!(info.contains("san1.test"));
        assert!(info.contains("san2.test"));
    }

    #[test]
    fn test_list_returns_domains() {
        let (store, _dir) = setup();
        generate("beta.test", &store, &[]).unwrap();
        generate("alpha.test", &store, &[]).unwrap();
        let domains = list(&store).unwrap();
        assert_eq!(domains, vec!["alpha.test", "beta.test"]);
    }

    #[test]
    fn test_list_empty_when_no_certs() {
        let (store, _dir) = setup();
        let domains = list(&store).unwrap();
        assert!(domains.is_empty());
    }

    #[test]
    fn test_show_returns_info() {
        let (store, _dir) = setup();
        generate("show.test", &store, &[]).unwrap();
        let info = show("show.test", &store).unwrap();
        assert!(info.contains("Domain:        show.test"));
        assert!(info.contains("Issuer:        local-ssl Development CA"));
        assert!(info.contains("Valid from:"));
        assert!(info.contains("Valid until:"));
        assert!(info.contains("SANs:"));
        assert!(info.contains("Cert:"));
        assert!(info.contains("Key:"));
    }

    #[test]
    fn test_show_nonexistent_domain() {
        let dir = tempdir().unwrap();
        let store = CaStore {
            dir: dir.path().to_path_buf(),
            key_path: dir.path().join("ca-key.pem"),
            cert_path: dir.path().join("ca-cert.pem"),
        };
        let result = show("nonexistent.test", &store);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "No certificate for 'nonexistent.test'");
    }

    #[test]
    fn test_check_local_returns_all_fields() {
        let (store, _dir) = setup();
        generate("check.test", &store, &[]).unwrap();
        let result = check_local("check.test", &store).unwrap();
        assert!(result.contains("Domain:"));
        assert!(result.contains("Validity:"));
        assert!(result.contains("Generated:"));
        assert!(result.contains("Expires:"));
        assert!(result.contains("Key file:"));
        assert!(result.contains("Reload needed:"));
    }

    #[test]
    fn test_check_all_local_returns_all() {
        let (store, _dir) = setup();
        generate("alpha.test", &store, &[]).unwrap();
        generate("beta.test", &store, &[]).unwrap();
        let result = check_all_local(&store).unwrap();
        assert!(result.contains("alpha.test"));
        assert!(result.contains("beta.test"));
    }

    #[test]
    fn test_check_local_nonexistent_domain() {
        let (store, _dir) = setup();
        let result = check_local("ghost.test", &store);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_all_local_empty() {
        let (store, _dir) = setup();
        let result = check_all_local(&store).unwrap();
        assert!(result.contains("No certificates found"));
    }
}
