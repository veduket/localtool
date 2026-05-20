# Developer Guide — local-ssl

Architecture, building, extending, and contributing to local-ssl.

## Table of Contents

- [Project Structure](#project-structure)
- [Architecture](#architecture)
- [Building](#building)
- [Module Overview](#module-overview)
- [Dependency Rationale](#dependency-rationale)
- [Extending local-ssl](#extending-local-ssl)
- [Cross-Platform Notes](#cross-platform-notes)
- [Testing](#testing)
- [Release Process](#release-process)
- [Contributing](#contributing)

## Project Structure

```
local-ssl/
├── Cargo.toml          # Rust dependencies and metadata
├── src/
│   ├── main.rs         # CLI entry point (commands, parser, dispatch)
│   ├── ca.rs           # Certificate Authority lifecycle (init, status, load)
│   ├── cert.rs         # Server certificate generation, listing, inspection
│   ├── trust.rs        # System trust store integration (Linux, macOS, Windows)
│   └── util.rs         # PEM decoding helper
├── README.md           # Project landing page
├── ADMIN_GUIDE.md      # Deployment and operations guide
├── DEVELOPER_GUIDE.md  # This file
├── CONTRIBUTING.md     # Contribution guidelines
├── LICENSE             # MIT license
└── .gitignore
```

### Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `clap` | 4.x (derive) | CLI argument parsing with subcommands |
| `colored` | 2.x | Terminal output colors |
| `rcgen` | 0.14 | X.509 certificate generation (CA + server certs) |
| `time` | 0.3 | Timestamp handling for certificate validity |
| `pem` | 3.x | PEM encoding/decoding |
| `x509-parser` | 0.17 | Parse and inspect X.509 certificate fields |
| `base64` | 0.22 | Base64 decode PEM body content |

This project has **zero system dependencies** — no OpenSSL, no libssl-dev, no Node.js. Everything is pure Rust.

## Architecture

### Data Flow

```
CLI Input
    │
    ▼
CLI Parser (clap derive)  ────►  execute()
    │
    ├──► init
    │       └──► ca.rs::CaStore::init()
    │               ├── Generate RSA key pair (rcgen)
    │               ├── Create self-signed CA cert (10-year validity)
    │               ├── Write ca-key.pem + ca-cert.pem to /etc/local-ssl/
    │               └──► trust.rs::install_ca()
    │                       ├── Linux: copy to trust anchor dir + update-ca-*
    │                       ├── macOS: security add-trusted-cert
    │                       └── Windows: certutil -addstore Root
    │
    ├──► generate <domains...>
    │       └──► cert.rs::generate()
    │               ├── Load CA key from disk
    │               ├── Generate server key pair (rcgen)
    │               ├── Build SAN list (primary + *.primary + extra domains)
    │               ├── Sign with CA via rcgen::Issuer
    │               └── Write cert.pem + key.pem to /etc/local-ssl/certs/<domain>/
    │
    ├──► list
    │       └──► cert.rs::list()
    │               └── List subdirectories under /etc/local-ssl/certs/ containing cert.pem
    │
    ├──► show <domain>
    │       └──► cert.rs::show()
    │               ├── Read cert.pem
    │               ├── Decode PEM → DER (util::pem_decode)
    │               └── Parse with x509-parser → print CN, issuer, validity, SANs
    │
    ├──► trust
    │       └──► trust.rs::install_ca()
    │               └── (Re)install CA into system trust store
    │
    └──► status
            ├──► ca.rs::CaStore::status()  → parsed CA certificate details
            ├──► trust.rs::is_ca_trusted() → check if CA is in system trust
            └──► cert.rs::list()           → count of generated certs
```

### Key Design Decisions

1. **Single CA per machine** — one root CA generates all development certs. This means one trust-store entry for unlimited domains.
2. **File-based storage** — no database. CA key/cert and generated server certs are just PEM files on disk. Simple to back up, inspect, and debug.
3. **Wildcard SANs** — every generated cert automatically includes `*.<domain>` as a SAN, so you can use `https://api.myapp.test` and `https://myapp.test` from the same cert.
4. **10-year CA / 1-year server cert** — the CA is long-lived (rare to regenerate), server certs are short-lived (easy to automate regeneration).

## Module Overview

### `main.rs` — CLI Entry Point

Defines the `Cli` struct and `Commands` enum using clap derive. The `execute()` function dispatches to handler functions (`cmd_init`, `cmd_generate`, etc.). All handlers return `Result<String, String>`. Error messages are printed in red via `colored`.

### `ca.rs` — CA Lifecycle

- **`CaStore`** struct holds paths to `/etc/local-ssl/`, `ca-key.pem`, `ca-cert.pem`. On Windows, uses `%PROGRAMDATA%\local-ssl\`.
- **`CaStore::init()`** generates a 4096-bit RSA key pair via `rcgen::KeyPair`, creates a self-signed CA with `BasicConstraints::Ca(true)` and `KeyCertSign` + `CrlSign` usages, then writes PEM files.
- **`CaStore::status()`** reads the CA cert, decodes it via `util::pem_decode`, and parses it with `x509-parser` to display subject, issuer, serial, and validity dates.
- **`CaStore::load_key()`** reads the CA private key for signing server certs.

### `cert.rs` — Certificate Generation & Inspection

- **`generate()`** — The core function. Builds `CertificateParams` with the primary domain as CN and all domains + wildcard as SANs. Sets `DigitalSignature` + `KeyEncipherment` usages and `ServerAuth` + `ClientAuth` EKUs. Signs with the CA key via `rcgen::Issuer`. Writes `cert.pem` and `key.pem` to `/etc/local-ssl/certs/<domain>/`.
- **`list()`** — Scans `/etc/local-ssl/certs/` for subdirectories containing `cert.pem`.
- **`show()`** — Reads and parses a certificate with `x509-parser`, displaying CN, issuer, validity, all SANs, and file paths.

### `trust.rs` — System Trust Integration

Detects the OS and applies the appropriate trust mechanism:

| OS | Detection | Install Command |
|----|-----------|-----------------|
| Debian/Ubuntu | `/usr/local/share/ca-certificates/` exists | `cp` + `update-ca-certificates` |
| Fedora/RHEL | `/etc/pki/ca-trust/source/anchors/` exists | `cp` + `update-ca-trust` |
| Arch/openSUSE | `/usr/share/pki/trust/anchors/` exists | `cp` + `trust extract-compat` |
| Alpine | `/etc/ca-certificates/trust-source/anchors/` exists | `cp` + `update-ca-certificates` |
| macOS | Always | `security add-trusted-cert` |
| Windows | Always | `certutil -addstore Root` |

The `is_ca_trusted()` function checks trust by looking for the CA's certificate hash file on Linux or searching the keychain on macOS.

### `util.rs` — PEM Decoding

A simple helper that strips PEM headers/footers and Base64-decodes the body. Used by `ca.rs::status()` and `cert.rs::show()` before passing DER bytes to `x509-parser`.

## Building

### Debug Build

```bash
cargo build
./target/debug/local-ssl --help
```

### Release Build

```bash
cargo build --release
# Binary at: ./target/release/local-ssl
sudo cp target/release/local-ssl /usr/local/bin/
```

### Cross-Compilation

For a different target (e.g., ARM for Raspberry Pi):

```bash
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

### Building for Offline Air-Gapped Systems

```bash
cargo vendor                  # Download all deps to vendor/
cargo build --release         # Build normally (uses vendor/ if configured)
# Copy the binary + vendor/ to the target machine
```

## Extending local-ssl

### Adding a New Command

1. Add variant to `Commands` enum with clap attributes
2. Add match arm in `execute()` function
3. Implement the handler function

Example — adding a `renew` command to regenerate an expiring cert:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    /// Regenerate a certificate before it expires
    Renew {
        /// Domain to renew
        domain: String,
    },
}

fn execute(cli: Cli) -> Result<String, String> {
    match cli.command {
        // ... existing arms ...
        Commands::Renew { domain } => cmd_renew(&store, &domain),
    }
}

fn cmd_renew(store: &ca::CaStore, domain: &str) -> Result<String, String> {
    // Backup existing cert, regenerate, replace
    let bundle = cert::generate(domain, store, &[])?;
    Ok(format!("Cert renewed for {domain}"))
}
```

### Supporting a New Trust Store

If a Linux distro uses a different trust mechanism (e.g., NixOS, Gentoo):

1. Add detection logic in `linux_install()` in `trust.rs`
2. Check for the distro-specific trust directory
3. Copy the CA cert to the right location
4. Run the distro's trust update command

Example for NixOS:

```rust
} else if Path::new("/etc/ssl/certs").is_dir()
    && Path::new("/nix").exists()
{
    // NixOS — copy to /etc/ssl/certs and run update-ca-certificates
}
```

Add corresponding check in `is_ca_trusted()` to verify the CA is present.

### Adding Certificate Profile Options

Extend `cert::generate()` to accept optional config:

```rust
pub fn generate(
    domain: &str,
    ca_store: &CaStore,
    sans: &[String],
    validity_days: Option<u64>,
    key_size: Option<KeySize>,
) -> Result<CertBundle, String>
```

Then expose these via clap arguments on the `Generate` variant.

### Adding EC Key Support

Currently `rcgen::KeyPair::generate()` creates RSA keys. To support ECDSA:

1. Add a `--key-type` flag to the `Generate` and `Init` commands
2. Use `rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)` for EC keys
3. The CA key type and server cert key type can differ

## Cross-Platform Notes

### Linux

- Requires root for writing to `/etc/local-ssl/` and system trust anchors
- Detects distro-specific trust directories automatically
- Uses `sudo -v` timestamp or direct `sudo` invocation

### macOS

- Requires `sudo` for `security add-trusted-cert` to System keychain
- Trust check uses `security find-certificate`
- All files stored at `/etc/local-ssl/` (Linux convention, works on macOS)

### Windows

- Stores CA at `%PROGRAMDATA%\local-ssl\` (typically `C:\ProgramData\local-ssl\`)
- Uses `certutil -addstore Root` for trust
- No automatic trust check (returns `false` for `is_ca_trusted()`)

### Known Limitations

- `x509-parser` has limited support for certain extension parsing on Windows (trust check falls back to `false`)
- CA regeneration requires manual cleanup of trust store entries from the previous CA
- Cross-compilation requires the `ring` native dependency (resolved by `rcgen`)

## Testing

### Unit Tests

Add tests with `#[cfg(test)]` modules:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_san_list_generation() {
        let result = cert::build_san_list("myapp.test", &["api.test".into()]);
        assert!(result.contains(&"myapp.test".to_string()));
        assert!(result.contains(&"*.myapp.test".to_string()));
        assert!(result.contains(&"api.test".to_string()));
    }

    #[test]
    fn test_ca_params_are_ca() {
        let params = CaStore::ca_params();
        assert!(matches!(params.is_ca, IsCa::Ca(_)));
    }

    #[test]
    fn test_pem_decode_valid() {
        let pem = "-----BEGIN CERT-----\nZm9v\n-----END CERT-----";
        assert!(util::pem_decode(pem).is_ok());
    }

    #[test]
    fn test_pem_decode_no_headers() {
        let pem = "plain text";
        assert!(util::pem_decode(pem).is_err());
    }
}
```

Run tests:

```bash
cargo test
cargo clippy --all-targets
```

### Manual Testing

```bash
# Full workflow test
sudo local-ssl init
sudo local-ssl status
sudo local-ssl generate myapp.test
sudo local-ssl generate api.test www.test admin.test
sudo local-ssl list
sudo local-ssl show myapp.test
sudo local-ssl show api.test

# Verify with openssl
openssl verify -CAfile /etc/local-ssl/ca-cert.pem /etc/local-ssl/certs/myapp.test/cert.pem
openssl x509 -in /etc/local-ssl/ca-cert.pem -text -noout | head -20
openssl x509 -in /etc/local-ssl/certs/myapp.test/cert.pem -text -noout | head -30

# Cleanup
sudo rm -rf /etc/local-ssl/
```

### Testing Trust on Linux

After `sudo local-ssl init`, verify the CA is trusted:

```bash
# Check trust anchor exists
ls -la /usr/local/share/ca-certificates/local-ssl.crt 2>/dev/null \
  || ls -la /etc/pki/ca-trust/source/anchors/local-ssl.pem 2>/dev/null

# Verify with OpenSSL
openssl verify /etc/local-ssl/ca-cert.pem  # should say OK (self-signed)
```

## Release Process

1. Update version in `Cargo.toml`
2. Run `cargo update` to refresh lockfile
3. Commit: `git commit -m "Release v0.1.1"`
4. Tag: `git tag v0.1.1`
5. Push: `git push origin main --tags`
6. Build release binaries: `cargo build --release`
7. Create GitHub release with binaries attached

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for full guidelines. Key points:

- Rust formatting: `cargo fmt`
- No warnings: `cargo build` must be clean
- Module philosophy: one responsibility per module
- CA operations require root — document this clearly
- Never expose CA private keys outside `/etc/local-ssl/`
- Certificate operations must validate all inputs
