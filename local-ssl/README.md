# local-ssl — Local HTTPS for Development

> Trust your local certs. Ship with confidence.

> **⚠ Deprecation notice:** `local-ssl` is now a library crate. Use the unified CLI: `localtool ssl <command>`.
> The standalone binary still works and is a thin wrapper around the library.
> See [github.com/veduket/localtool](https://github.com/veduket/localtool) for the unified tool.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/version-0.1.0--semver-blue)]()

Modern browsers require HTTPS for Service Workers, WebRTC, geolocation, and more. OAuth providers demand HTTPS redirect URIs — even for `localhost`. Self-signed certificates trigger scary warnings that break your flow.

`local-ssl` generates a private Certificate Authority on your machine, installs it in your system trust store, and issues trusted HTTPS certificates for any local development domain. One command to create a CA. One command to generate certs. Your browser stops complaining and your OAuth flows work.

This tool is built for **local development only**. Never use these certificates in production.

Pair it with [`local-dns`](https://github.com/veduket/local-dns) for the complete local dev experience — name resolution + TLS trust.

## Features

- **Local CA generation** — creates a private root CA and installs it in your system trust store (Linux, macOS, Windows)
- **Wildcard certs** — every generated cert includes `*.domain` as a Subject Alternative Name
- **Multi-domain SANs** — `local-ssl generate api.test www.test app.test` for one cert covering all three
- **System trust integration** — auto-installs CA via `security` (macOS), `update-ca-certificates` (Debian), `update-ca-trust` (Fedora), `certutil` (Windows)
- **Pure Rust** — no OpenSSL dependency, no Node.js, no Python
- **Compatible with local-dns** — use together: `local-dns add myapp.test 127.0.0.1 && local-ssl generate myapp.test`

## Quick Start

```bash
# Install
cargo build --release
sudo cp target/release/local-ssl /usr/local/bin/

# Generate and trust a local CA (one-time)
sudo local-ssl init

# Generate a cert for your development domain
sudo local-ssl generate myapp.test

# Use in your app (example with curl)
curl --cacert /etc/local-ssl/ca-cert.pem https://myapp.test/
```

## Why This Exists

Every developer hits the HTTPS wall eventually:

| Problem | Solution |
|---------|----------|
| Browsers block features over HTTP | Trusted local CA silences security warnings |
| OAuth providers require HTTPS redirect URIs | Valid certs for `myapp.test` make OAuth flows work locally |
| Service Workers require HTTPS | Register workers during development without workarounds |
| Self-signed certs cause confusing errors | System-trusted CA means zero browser friction |
| `/etc/hosts` entries need HTTPS too | Pair with `local-dns` for DNS + TLS in one workflow |

## Usage

### Initialize CA

```bash
sudo local-ssl init
```

This generates a 10-year CA key and certificate at `/etc/local-ssl/` and installs trust system-wide.

### Generate Certificates

```bash
# Single domain (auto-includes *.domain as SAN)
sudo local-ssl generate myapp.test

# Multiple SANs
sudo local-ssl generate api.test www.test admin.test

# List generated certs
sudo local-ssl list

# Show certificate details
sudo local-ssl show myapp.test

# Check certificate validity or remote TLS
sudo local-ssl check myapp.test
sudo local-ssl check api.test:443   # Remote TLS inspection
```

### Manage Trust

```bash
# Reinstall CA into system trust store
sudo local-ssl trust

# Check CA and system trust status
sudo local-ssl status

# Manage telemetry
sudo local-ssl telemetry status
sudo local-ssl telemetry enable
sudo local-ssl telemetry disable
```

### Integration with local-dns

```bash
# Install both tools
sudo local-dns init
sudo local-ssl init

# Point domain to localhost and get a trusted cert
sudo local-dns add myapp.test 127.0.0.1
sudo local-ssl generate myapp.test

# Start your HTTPS server with the generated cert/key
# Cert: /etc/local-ssl/certs/myapp.test/cert.pem
# Key:  /etc/local-ssl/certs/myapp.test/key.pem
```

## How It Works

```
local-ssl init
  ├── Generates 4096-bit RSA key pair for CA
  ├── Creates self-signed CA certificate (10-year validity)
  ├── Installs CA in system trust store
  └── Ready to sign development certificates

local-ssl generate myapp.test
  ├── Generates server key pair
  ├── Creates cert signed by local CA
  │   ├── CN = myapp.test
  │   ├── SANs = myapp.test, *.myapp.test
  │   └── 1-year validity
  ├── Writes cert.pem + key.pem
  └── Ready to use with any HTTPS server
```

## Certificate Locations

| Component | Path |
|-----------|------|
| CA key | `/etc/local-ssl/ca-key.pem` |
| CA cert | `/etc/local-ssl/ca-cert.pem` |
| Generated certs | `/etc/local-ssl/certs/<domain>/cert.pem` |
| Generated keys | `/etc/local-ssl/certs/<domain>/key.pem` |

## Supported Platforms

| Platform | CA Trust Mechanism |
|----------|-------------------|
| Debian/Ubuntu | `update-ca-certificates` via `/usr/local/share/ca-certificates/` |
| Fedora/RHEL | `update-ca-trust` via `/etc/pki/ca-trust/source/anchors/` |
| Arch/openSUSE | `trust extract-compat` via `/usr/share/pki/trust/anchors/` |
| macOS | `security add-trusted-cert` to System keychain |
| Windows | `certutil -addstore Root` |

## Building from Source

```bash
git clone https://github.com/veduket/local-ssl.git
cd local-ssl
cargo build --release
sudo cp target/release/local-ssl /usr/local/bin/
```

Requires Rust 1.75+.

## Documentation

| Guide | Audience | Contents |
|-------|----------|---------|
| [README](README.md) | Everyone | Quick start, feature overview |
| [Admin Guide](ADMIN_GUIDE.md) | Sysadmins | Deployment, integration, troubleshooting |
| [Developer Guide](DEVELOPER_GUIDE.md) | Contributors | Architecture, building, extending |
| [Contributing](CONTRIBUTING.md) | Contributors | PR workflow, code style, testing |

## Versioning

This project follows [Semantic Versioning 2.0.0](https://semver.org/). Given a `MAJOR.MINOR.PATCH` version:
- **MAJOR** — breaking changes to CLI commands, certificate storage format, or CA generation
- **MINOR** — new features, commands, or platform integrations (backward-compatible)
- **PATCH** — bug fixes, performance improvements, or documentation updates

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for our guidelines.

Before submitting a PR, please:
1. Ensure all tests pass: `cargo test`
2. Verify the code compiles with no warnings: `cargo build`
3. Run `cargo clippy` for linting
4. Update documentation if adding or changing commands

## License

MIT — see [LICENSE](LICENSE).

---

Made in Ethiopia with love by **Yared Getachew** and [OpenCode](https://opencode.ai) (Big Pickle).
