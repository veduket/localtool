use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair, KeyUsagePurpose};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use x509_parser::prelude::*;

use crate::util;

pub struct CaStore {
    pub dir: PathBuf,
    pub key_path: PathBuf,
    pub cert_path: PathBuf,
}

impl CaStore {
    pub fn new() -> Self {
        let dir = if cfg!(target_os = "windows") {
            PathBuf::from(std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".into()))
                .join("local-ssl")
        } else {
            PathBuf::from("/etc/local-ssl")
        };
        CaStore {
            key_path: dir.join("ca-key.pem"),
            cert_path: dir.join("ca-cert.pem"),
            dir,
        }
    }

    pub fn exists(&self) -> bool {
        self.key_path.exists() && self.cert_path.exists()
    }

    pub fn ca_params() -> CertificateParams {
        let mut params = CertificateParams::new(vec!["local-ssl Development CA".to_string()])
            .expect("Cannot create CA params");
        params
            .distinguished_name
            .push(DnType::CommonName, "local-ssl Development CA");
        params
            .distinguished_name
            .push(DnType::OrganizationName, "local-ssl");
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        params
    }

    pub fn init(&self) -> Result<(), String> {
        fs::create_dir_all(&self.dir)
            .map_err(|e| format!("Cannot create {}: {e}", self.dir.display()))?;

        let key_pair = KeyPair::generate().map_err(|e| format!("Cannot generate CA key: {e}"))?;

        let mut params = Self::ca_params();
        params.not_before = ::time::OffsetDateTime::now_utc();
        params.not_after =
            ::time::OffsetDateTime::now_utc() + Duration::from_secs(10 * 365 * 86400);

        let cert = params
            .self_signed(&key_pair)
            .map_err(|e| format!("Cannot self-sign CA: {e}"))?;

        fs::write(&self.key_path, key_pair.serialize_pem())
            .map_err(|e| format!("Cannot write CA key: {e}"))?;
        fs::write(&self.cert_path, cert.pem()).map_err(|e| format!("Cannot write CA cert: {e}"))?;

        Ok(())
    }

    pub fn load_key(&self) -> Result<KeyPair, String> {
        let pem =
            fs::read_to_string(&self.key_path).map_err(|e| format!("Cannot read CA key: {e}"))?;
        KeyPair::from_pem(&pem).map_err(|e| format!("Cannot parse CA key: {e}"))
    }

    pub fn status(&self) -> Result<String, String> {
        if !self.exists() {
            return Ok("CA not initialized. Run `local-ssl init`.".to_string());
        }

        let pem = fs::read_to_string(&self.cert_path).map_err(|e| format!("{e}"))?;
        let der = util::pem_decode(&pem)?;
        let parsed = X509Certificate::from_der(&der)
            .map_err(|e| format!("Cannot parse CA cert: {e}"))?
            .1;

        let cn = parsed
            .subject()
            .iter_common_name()
            .next()
            .and_then(|a| a.as_str().ok())
            .unwrap_or("(unknown)");
        let not_before = parsed.validity().not_before.to_datetime();
        let not_after = parsed.validity().not_after.to_datetime();
        let issuer = parsed
            .issuer()
            .iter_common_name()
            .next()
            .and_then(|a| a.as_str().ok())
            .unwrap_or("(unknown)");
        let serial = &parsed.tbs_certificate.serial;

        let mut out = format!("Subject:       {cn}\n");
        out.push_str(&format!("Issuer:        {issuer}\n"));
        out.push_str(&format!("Serial:        {serial}\n"));
        out.push_str(&format!("Valid from:    {not_before}\n"));
        out.push_str(&format!("Valid until:   {not_after}\n"));
        out.push_str(&format!("Location:      {}\n", self.dir.display()));

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcgen::{BasicConstraints, IsCa};
    use tempfile::tempdir;

    fn temp_store() -> (CaStore, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let store = CaStore {
            dir: dir.path().to_path_buf(),
            key_path: dir.path().join("ca-key.pem"),
            cert_path: dir.path().join("ca-cert.pem"),
        };
        (store, dir)
    }

    #[test]
    fn test_exists_returns_false_for_nonexistent() {
        let (store, _dir) = temp_store();
        assert!(!store.exists());
    }

    #[test]
    fn test_init_creates_files() {
        let (store, _dir) = temp_store();
        store.init().unwrap();
        assert!(store.exists());
        assert!(store.key_path.exists());
        assert!(store.cert_path.exists());
    }

    #[test]
    fn test_load_key_after_init() {
        let (store, _dir) = temp_store();
        store.init().unwrap();
        let key = store.load_key().unwrap();
        let pem = key.serialize_pem();
        assert!(pem.starts_with("-----BEGIN PRIVATE KEY-----"));
    }

    #[test]
    fn test_status_before_init() {
        let (store, _dir) = temp_store();
        let status = store.status().unwrap();
        assert_eq!(status, "CA not initialized. Run `local-ssl init`.");
    }

    #[test]
    fn test_status_after_init() {
        let (store, _dir) = temp_store();
        store.init().unwrap();
        let status = store.status().unwrap();
        assert!(status.contains("Subject:       local-ssl Development CA"));
        assert!(status.contains("Issuer:        local-ssl Development CA"));
        assert!(status.contains("Serial:"));
        assert!(status.contains("Valid from:"));
        assert!(status.contains("Valid until:"));
        assert!(status.contains("Location:"));
    }

    #[test]
    fn test_ca_params_is_ca() {
        let params = CaStore::ca_params();
        assert!(matches!(
            params.is_ca,
            IsCa::Ca(BasicConstraints::Unconstrained)
        ));
    }

    #[test]
    fn test_ca_params_key_usages() {
        let params = CaStore::ca_params();
        assert_eq!(
            params.key_usages,
            vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign]
        );
    }
}
