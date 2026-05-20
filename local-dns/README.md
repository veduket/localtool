# local-dns — Local DNS Management

> Stop wrestling `/etc/hosts`. Manage local DNS with profiles, wildcards, zero-conflict ports, and hot-reload.

> **⚠ Deprecation notice:** `local-dns` is now a library crate. Use the unified CLI: `localtool dns <command>`.
> The standalone binary still works and is a thin wrapper around the library.
> See [github.com/veduket/localtool](https://github.com/veduket/localtool) for the unified tool.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![CI](https://img.shields.io/badge/CI-passing-brightgreen)]()
[![Version](https://img.shields.io/badge/version-0.1.0--semver-blue)]()

Every developer knows the pain. You're building a multi-service app — an API server on port 3000, a frontend on 5173, WebSockets on 8080, a database admin tool, and a mail catcher. To test authentication flows you need real-ish domains. So you reach for `/etc/hosts`.

And immediately hit the wall.

No wildcards (good luck adding `*.test` — impossible). No profiles (switching between "work" and "side-project" means commenting lines in and out). No organization — 20 entries deep, you've forgotten what half of them are for. No integration — your DNS resolver has no idea these entries exist, so split-brain issues haunt you. No hot-reload — every edit requires a network restart or manual cache flush.

`local-dns` bridges the gap between dev and production DNS. You define domains in logical zones and groups, point them at local IPs, and your entire system resolves them instantly — just like real DNS would in production. Add `myapp.test`, `api.staging.local`, `*.dev` — all with a single CLI command. Switch between project profiles without touching a config file. Hot-reload via SIGHUP means zero-downtime updates.

Built with Rust, backed by SQLite, powered by dnsmasq on port 5354 — so it never conflicts with your existing DNS stack. Auto-detects dnsdist, systemd-resolved, dnscrypt-proxy, and adapts. This is how local DNS should work.

## Features

- **Zones & Groups** — organize DNS entries hierarchically (`addis_ababa/bole`, `global/main`)
- **Profiles** — switch between "work", "personal", "project-x" DNS configs
- **Wildcards** — `*.test` → `127.0.0.1` in one line (impossible with `/etc/hosts`)
- **Smart detection** — auto-detects dnsdist, systemd-resolved, dnscrypt-proxy, bind9
- **Zero-conflict** — runs on port 5354, adapts to your existing DNS stack
- **Hot-reload** — SIGHUP reload, zero dropped queries
- **Query logging** — see every DNS request with `local-dns logs`
- **SQLite backend** — reliable, queryable, no JSON files

## Quick Start

```bash
# Install
cargo build --release
sudo cp target/release/local-dns /usr/local/bin/

# Setup — auto-detects your system
sudo local-dns init

# Start the service
sudo systemctl enable --now local-dnsmasq

# Add your first entry
sudo local-dns add myapp.test 127.0.0.1

# Test it
ping myapp.test
```

## Documentation

| Guide | Audience | Contents |
|-------|----------|---------|
| [README](README.md) | Everyone | Quick start, feature overview |
| [Admin Guide](ADMIN_GUIDE.md) | Sysadmins | Deployment, integration, troubleshooting |
| [Developer Guide](DEVELOPER_GUIDE.md) | Contributors | Architecture, building, extending |
| [Contributing](CONTRIBUTING.md) | Contributors | PR workflow, code style, testing |

## Usage

### Managing Entries

```bash
# Default zone/group (global/main)
sudo local-dns add myapp.test 127.0.0.1
sudo local-dns add api.dev 127.0.0.1 -c "local API server"
sudo local-dns add "*.test" 127.0.0.1

# Organize into zones and groups
sudo local-dns add web.bole.test 127.0.0.1 -z addis_ababa -g bole
sudo local-dns add api.kazanchis.test 127.0.0.1 -z addis_ababa -g kazanchis

sudo local-dns remove myapp.test
sudo local-dns list

# Move entries between zones/groups
sudo local-dns move myapp.test -z addis_ababa -g bole

# Copy entries to another zone/group
sudo local-dns copy myapp.test -z addis_ababa -g kazanchis

# Edit entry IP or comment
sudo local-dns edit myapp.test -i 127.0.0.2 -c "moved to new IP"
```

### Zones & Groups

```bash
# Create a zone
sudo local-dns zone create addis_ababa -d "Addis Ababa"

# Show zone details
sudo local-dns zone show addis_ababa

# Create groups within a zone
sudo local-dns group create bole -z addis_ababa
sudo local-dns group create piazza -z addis_ababa -d "Piazza"
sudo local-dns group create kazanchis -z addis_ababa

# List zones or groups
sudo local-dns zone list
sudo local-dns group list
sudo local-dns group list -z addis_ababa

# Delete
sudo local-dns zone delete addis_ababa
sudo local-dns group delete bole -z addis_ababa
```

### Profiles

```bash
sudo local-dns profile create work
sudo local-dns profile switch work
sudo local-dns profile list
sudo local-dns profile delete work
```

### System Detection

```bash
sudo local-dns detect
```

Sample output:

```
╔══════════════════════════════════════════╗
║     local-dns — System DNS Detection    ║
╚══════════════════════════════════════════╝

Port Usage:
  Port 53     dnsdist
  Port 5053   dnscrypt-proxy
  Port 5353   avahi-daemon
  Port 5354   (free) ✓

DNS Services:
  ✓ dnsdist
  ✓ dnscrypt-proxy
  ✓ avahi-daemon

Upstream: 127.0.0.1:5053
```

### Service Status

```bash
sudo local-dns status
```

### Query Logs

```bash
sudo local-dns logs       # Recent 50 queries
sudo local-dns logs -f    # Follow in real-time
sudo local-dns logs -e    # Errors only (NXDOMAIN, REFUSED)
```

## How It Works

```
App → upstream resolver ─┬─→ *.test, *.dev → local-dnsmasq:5354
                         └─→ everything else → internet DNS
```

local-dns uses **dnsmasq** as its DNS engine, configured with:
- Port **5354** (avoids conflict with system DNS on port 53)
- PID file at `/run/local-dns/dnsmasq.pid` for reliable SIGHUP reloads
- SQLite3 database for entries at `/etc/local-dns/local-dns.db`
- Automatic upstream detection (dnsdist, dnscrypt-proxy, systemd-resolved)

## Supported Setups

| Upstream Resolver | Integration |
|-------------------|-------------|
| **dnsdist** | Lua routing snippet (auto-generated by `init`) |
| **systemd-resolved** | `resolvectl` domain routing commands |
| **dnscrypt-proxy** | Auto-detected as upstream (works with or without dnsdist) |
| **bind9** | Configure forwarding zone in named.conf |
| **None / direct** | Point `/etc/resolv.conf` to `127.0.0.1` |

## Building from Source

```bash
git clone https://github.com/veduket/local-dns.git
cd local-dns
cargo build --release
sudo cp target/release/local-dns /usr/local/bin/
```

Requires Rust 1.75+.

## Versioning

This project follows [Semantic Versioning 2.0.0](https://semver.org/). Given a `MAJOR.MINOR.PATCH` version:
- **MAJOR** — breaking changes to CLI commands, database schema, or dnsmasq configuration format
- **MINOR** — new features, commands, or system integrations (backward-compatible)
- **PATCH** — bug fixes, performance improvements, or documentation updates

## License

MIT — see [LICENSE](LICENSE).

---

Made in Ethiopia with love by **Yared Getachew** and [OpenCode](https://opencode.ai) (Big Pickle).
