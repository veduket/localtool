use colored::Colorize;
use std::path::Path;
use std::process::Command;

pub fn install_ca(cert_path: &Path) -> Result<(), String> {
    if cfg!(target_os = "macos") {
        macos_install(cert_path)
    } else if cfg!(target_os = "windows") {
        windows_install(cert_path)
    } else {
        linux_install(cert_path)
    }
}

pub fn is_ca_trusted(cert_path: &Path) -> bool {
    if cfg!(target_os = "macos") {
        macos_trust_check(cert_path)
    } else if cfg!(target_os = "linux") {
        linux_trust_check(cert_path)
    } else {
        false
    }
}

fn macos_trust_check(cert_path: &Path) -> bool {
    let pem = match std::fs::read_to_string(cert_path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let cn = pem.lines()
        .find(|l| l.contains("CN="))
        .and_then(|l| l.split("CN=").nth(1))
        .map(|s| s.trim_end_matches('"'))
        .unwrap_or("unknown");
    Command::new("security")
        .args(["find-certificate", "-c", cn, "/Library/Keychains/System.keychain"])
        .output()
        .ok()
        .map_or(false, |o| o.status.success())
}

fn linux_trust_check(cert_path: &Path) -> bool {
    let our_pem = match std::fs::read_to_string(cert_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let subject = our_pem.lines()
        .find(|l| l.contains("Subject: "))
        .unwrap_or("")
        .to_string();
    if subject.is_empty() {
        return false;
    }

    let bundles = [
        "/etc/ssl/certs/ca-certificates.crt",
        "/etc/pki/tls/certs/ca-bundle.crt",
        "/etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem",
        "/usr/share/pki/trust/trusted.pem",
        "/etc/ssl/ca-bundle.pem",
        "/etc/pki/tls/cacert.pem",
    ];

    bundles.iter().any(|b| {
        std::fs::read_to_string(b).ok()
            .map_or(false, |content| content.contains(&subject))
    })
}

fn linux_install(cert_path: &Path) -> Result<(), String> {
    let cert = std::fs::read_to_string(cert_path).map_err(|e| format!("Cannot read CA cert: {e}"))?;

    let dest = if Path::new("/usr/local/share/ca-certificates").is_dir() {
        "/usr/local/share/ca-certificates/local-ssl.crt"
    } else if Path::new("/etc/pki/ca-trust/source/anchors").is_dir() {
        "/etc/pki/ca-trust/source/anchors/local-ssl.pem"
    } else if Path::new("/usr/share/pki/trust/anchors").is_dir() {
        "/usr/share/pki/trust/anchors/local-ssl.pem"
    } else if Path::new("/etc/ca-certificates/trust-source/anchors").is_dir() {
        "/etc/ca-certificates/trust-source/anchors/local-ssl.crt"
    } else {
        return Err("Unsupported Linux distro — install CA manually.".into());
    };

    std::fs::write(dest, &cert).map_err(|e| format!("Cannot write CA cert to {dest}: {e}"))?;
    println!("  {} Copied CA to {}", "✓".green(), dest.cyan());

    let update_cmds: &[&[&str]] = &[
        &["update-ca-certificates"],
        &["update-ca-trust"],
        &["trust", "extract-compat"],
    ];

    for args in update_cmds {
        if which(args[0]) {
            let out = Command::new(args[0]).args(&args[1..]).output()
                .map_err(|e| format!("Cannot run {}: {e}", args[0]))?;
            if out.status.success() {
                return Ok(());
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                eprintln!("  {} {} failed, trying next...", "⚠".yellow(), args[0]);
                if !stderr.is_empty() {
                    eprintln!("    {}", stderr.trim());
                }
            }
        }
    }

    Err("CA cert copied but trust DB update failed — run manually:\n  sudo update-ca-certificates".into())
}

fn macos_install(cert_path: &Path) -> Result<(), String> {
    let s = Command::new("security")
        .args([
            "add-trusted-cert",
            "-d",
            "-r",
            "trustRoot",
            "-k",
            "/Library/Keychains/System.keychain",
            &cert_path.to_string_lossy(),
        ])
        .status()
        .map_err(|e| format!("Cannot add CA to keychain: {e}"))?;
    if s.success() {
        Ok(())
    } else {
        Err("Failed to install CA in System keychain".into())
    }
}

fn windows_install(cert_path: &Path) -> Result<(), String> {
    let s = Command::new("certutil")
        .args(["-addstore", "Root", &cert_path.to_string_lossy()])
        .status()
        .map_err(|e| format!("Cannot install CA: {e}"))?;
    if s.success() {
        Ok(())
    } else {
        Err("Failed to install CA in Root store".into())
    }
}

fn which(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .ok()
        .map_or(false, |o| o.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_which_existing_command() {
        assert!(which("echo"));
    }

    #[test]
    fn test_which_nonexistent_command() {
        assert!(!which("this-command-definitely-does-not-exist"));
    }
}
