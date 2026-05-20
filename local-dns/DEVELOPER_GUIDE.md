# Developer Guide — local-dns

Architecture, building, extending, and contributing to local-dns.

## Table of Contents

- [Project Structure](#project-structure)
- [Architecture](#architecture)
- [Building](#building)
- [Code Overview](#code-overview)
- [Extending local-dns](#extending-local-dns)
- [Testing](#testing)
- [Release Process](#release-process)
- [Contributing](#contributing)

## Project Structure

```
local-dns/
├── Cargo.toml              # Rust dependencies and metadata
├── src/
│   └── main.rs             # Single-binary CLI application (~820 lines)
├── README.md               # Project landing page
├── ADMIN_GUIDE.md           # Deployment and operations guide
├── DEVELOPER_GUIDE.md       # This file
├── CONTRIBUTING.md          # Contribution guidelines
├── LICENSE                  # MIT license
├── .gitignore
└── .github/
    └── ISSUE_TEMPLATE/
        ├── bug_report.md
        └── feature_request.md
```

### Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` (4.x, derive) | CLI argument parsing |
| `rusqlite` (0.31.x, bundled) | SQLite3 database with bundled libsqlite3 |

The `bundled` feature of rusqlite compiles SQLite from source — no system SQLite library required.

## Architecture

### Data Flow

```
CLI Input
    │
    ▼
CLI Parser (clap derive)
    │
    ├──► detect   → scan_ports() + find_services() → system state report
    │
    ├──► init     → detect_system() → write configs → init_db()
    │
    ├──► add      → open_db() → INSERT → apply()
    ├──► remove   → open_db() → DELETE → apply()
    ├──► list     → open_db() → SELECT
    │
    ├──► profile  → open_db() → UPDATE/SELECT profiles
    │
    ├──► apply    → open_db() → generate_zones_conf() → reload_dnsmasq()
    │
    ├──► status   → detect_system() + open_db() + service checks
    │
    ├──► logs     → tail/grep system commands
    │
    └──► detect   → detect_system() → pretty print
```

### Module Organization (single file)

The code is organized in sections within `main.rs`:

| Section | Lines | Purpose |
|---------|-------|---------|
| CLI definitions | 1-75 | Structs, enums, CLI argument parsing |
| Database | 75-145 | SQLite connection, schema, query helpers |
| System detection | 145-380 | Port scanning, service detection, upstream detection |
| Init | 380-460 | Guided first-time setup |
| Entry commands | 460-540 | add, remove, list |
| Profile commands | 540-620 | profile CRUD |
| Apply | 620-680 | Generate zones config, reload dnsmasq |
| Status | 680-780 | Service status, database summary |
| Logs | 780-850 | Query log viewing |
| Helpers | 850-920 | Config writers, service reload |

### Detection System

The `detect_system()` function builds a `SystemState` by:

1. **Port scanning** — runs `ss -tlnp` and matches DNS-related ports
2. **Service detection** — checks `systemctl is-active` for known DNS services
3. **Upstream detection** — uses heuristics to find the active DNS resolver
4. **Config generation** — produces dnsdist or systemd-resolved snippets as needed

This runs on every `detect`, `init`, and `status` command to always reflect the live system state.

### Database Schema

```sql
-- Profiles: named sets of DNS entries
CREATE TABLE profiles (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT UNIQUE NOT NULL,      -- "default", "work", "project-x"
    is_active  INTEGER NOT NULL DEFAULT 0, -- exactly one profile is active
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Entries: individual DNS records per profile
CREATE TABLE entries (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id INTEGER NOT NULL,          -- FK to profiles.id
    domain     TEXT NOT NULL,              -- "myapp.test" or "*.test"
    ip         TEXT NOT NULL,              -- "127.0.0.1" or "::1"
    comment    TEXT,                       -- optional description
    sort_key   TEXT NOT NULL,              -- domain without wildcard prefix, for sorting
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE,
    UNIQUE(profile_id, domain)            -- no duplicate domains per profile
);
```

## Building

### Debug Build

```bash
cargo build
./target/debug/local-dns --help
```

### Release Build

```bash
cargo build --release
# Binary at: ./target/release/local-dns
```

### Cross-Compilation

For a different target (e.g., ARM for Raspberry Pi):

```bash
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

## Extending local-dns

### Adding a New Command

1. Add variant to `Commands` enum with clap attributes
2. Add match arm in `execute()` function
3. Implement the handler function

Example — adding a `stats` command:

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing commands ...
    /// Show DNS statistics
    Stats,
}

fn execute(cli: Cli) -> Result<String, String> {
    match cli.command {
        // ... existing arms ...
        Commands::Stats => cmd_stats(),
    }
}

fn cmd_stats() -> Result<String, String> {
    let conn = open_db()?;
    let profile = active_profile(&conn)?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entries WHERE profile_id = ?1",
        params![profile.id],
        |row| row.get(0),
    ).unwrap_or(0);
    Ok(format!("Profile '{}' has {count} entries", profile.name))
}
```

### Adding a New DNS Backend

The tool currently uses dnsmasq. To support another DNS server (e.g., CoreDNS, Unbound):

1. Add a new config generator function (like `generate_zones_conf`)
2. Add a new config writer function (like `write_dnsmasq_conf`)
3. Add a `--backend` flag to `init`

### Enhancing Detection

Add new services to the `checks` list in `find_services()`:

```rust
let checks: Vec<(&str, DnsService)> = vec![
    // ... existing ...
    ("knot-resolver", DnsService::Other("knot-resolver".into())),
];
```

## Testing

### Unit Tests

The codebase is a single binary. Tests can be added with `#[cfg(test)]`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_zones_conf_basic() {
        let entries = vec![
            Entry { domain: "myapp.test".into(), ip: "127.0.0.1".into(), comment: None },
        ];
        let conf = generate_zones_conf(&entries);
        assert!(conf.contains("address=/myapp.test/127.0.0.1"));
    }

    #[test]
    fn test_generate_zones_conf_wildcard() {
        let entries = vec![
            Entry { domain: "*.test".into(), ip: "127.0.0.1".into(), comment: None },
        ];
        let conf = generate_zones_conf(&entries);
        assert!(conf.contains("address=/.test/127.0.0.1"));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(2048), "2.0 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
    }
}
```

Run tests:

```bash
cargo test
```

### Manual Testing

```bash
# Run in a sandboxed environment
sudo local-dns detect
sudo local-dns init
sudo local-dns add test.local 127.0.0.1
sudo local-dns list
sudo local-dns status
sudo local-dns logs
dig test.local @127.0.0.1 -p 5354
sudo local-dns remove test.local
```

## Release Process

1. Update version in `Cargo.toml`
2. Commit: `git commit -m "Release v0.1.1"`
3. Tag: `git tag v0.1.1`
4. Push: `git push origin main --tags`
5. Build release binaries: `cargo build --release`
6. Create GitHub release with binaries attached

## Reload Mechanism

The `reload_dnsmasq()` function tries three methods in order:

1. **systemctl reload** → sends SIGHUP → zero-downtime config reload
2. **kill -HUP <pid>** → reads PID from `/run/local-dns/dnsmasq.pid` or systemctl
3. **systemctl restart** → last resort, brief query interruption

### Adding a reload method

```rust
// Try supervisor reload (e.g., s6, runit)
if Command::new("sv").args(["hup", DNSMASQ_SERVICE]).status().ok()
    .map_or(false, |s| s.success())
{
    return Ok(());
}
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for full guidelines. Key points:

- Rust formatting: `cargo fmt`
- No warnings: `cargo build` must be clean
- Single-file philosophy: keep `main.rs` under 1200 lines
- System detection must never require root
- Database migrations must be backward-compatible
