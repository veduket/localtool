# localtool — Local Development Toolkit

> One tool for local DNS management and HTTPS certificates.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![Version](https://img.shields.io/badge/version-0.1.0--semver-blue)]()

`localtool` unifies [`local-dns`](https://github.com/veduket/local-dns) and [`local-ssl`](https://github.com/veduket/local-ssl) into a single CLI.
Manage DNS entries and generate locally-trusted HTTPS certificates — no more `/etc/hosts` editing or self-signed cert warnings.

```bash
# DNS management
localtool dns add myapp.test 127.0.0.1
localtool dns list
localtool dns init

# SSL certificates
localtool ssl init
localtool ssl generate myapp.test
localtool ssl status
```

## Features

- **DNS management** — profiles, zones, groups, wildcards, hot-reload via SIGHUP
- **HTTPS certificates** — local CA generation, system trust integration, wildcard SANs
- **Smart detection** — auto-detects dnsdist, systemd-resolved, dnscrypt-proxy, bind9
- **Cross-platform** — Linux, macOS, Windows
- **Backwards compatible** — standalone `local-dns` and `local-ssl` binaries still work

## Install

```bash
cargo install localtool

# Build from source
git clone https://github.com/veduket/localtool
cd localtool
cargo build --release
sudo cp target/release/localtool /usr/local/bin/
```

## Quick Start

```bash
# 1. Initialize DNS
sudo localtool dns init
sudo systemctl enable --now local-dnsmasq

# 2. Add a domain
sudo localtool dns add myapp.test 127.0.0.1
sudo localtool dns apply

# 3. Set up HTTPS
sudo localtool ssl init
sudo localtool ssl generate myapp.test

# 4. Verify
ping myapp.test
curl --cacert /etc/local-ssl/ca-cert.pem https://myapp.test/
```

## Commands

### DNS

| Command | Description |
|---------|-------------|
| `localtool dns add <domain> <ip>` | Add a DNS entry |
| `localtool dns remove <domain>` | Remove a DNS entry |
| `localtool dns list` | List all entries |
| `localtool dns init` | Initialize DNS configuration |
| `localtool dns reset` | Delete database and re-initialize |
| `localtool dns status` | Show DNS system status |
| `localtool dns apply` | Apply DNS configuration |
| `localtool dns detect` | Detect system DNS setup |
| `localtool dns logs` | View dnsmasq logs |
| `localtool dns profile` | Manage profiles |
| `localtool dns zone` | Manage zones |
| `localtool dns group` | Manage groups |

### SSL

| Command | Description |
|---------|-------------|
| `localtool ssl init` | Initialize CA and install system trust |
| `localtool ssl generate <domains>` | Generate HTTPS certificates |
| `localtool ssl list` | List all generated certificates |
| `localtool ssl show <domain>` | Show certificate details |
| `localtool ssl trust` | Reinstall CA system trust |
| `localtool ssl status` | Show CA and certificate status |
| `localtool ssl check <domain>` | Check certificate validity |

## Backwards Compatibility

The standalone binaries continue to work:

```bash
local-dns add myapp.test 127.0.0.1     # still works
local-ssl generate myapp.test           # still works
```

They are thin wrappers that delegate to the same underlying libraries.

## Architecture

```
localtool
├── local-dns/    — DNS management library (SQLite + dnsmasq)
├── local-ssl/    — SSL certificate library (rcgen)
└── localtool/    — Unified CLI binary (clap dispatch)
```

## License

MIT — see [LICENSE](LICENSE).

---

Made in Ethiopia with love by **Yared Getachew** and [OpenCode](https://opencode.ai).
