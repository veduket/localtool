# Admin Guide — local-dns

Deployment, troubleshooting, and advanced configuration for system administrators.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Installation](#installation)
- [Integration with Existing DNS Stacks](#integration-with-existing-dns-stacks)
- [Systemd Service Management](#systemd-service-management)
- [Logging](#logging)
- [Troubleshooting](#troubleshooting)
- [Security Considerations](#security-considerations)
- [Backup & Restore](#backup--restore)

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                  local-dns (CLI)                      │
│  detect | init | add | remove | list | profile       │
│  status | apply | logs                                │
└──────────────────────┬──────────────────────────────┘
                       │ reads/writes
              ┌────────┴────────┐
              │  SQLite3 DB     │  /etc/local-dns/local-dns.db
              │  profiles +     │
              │  entries        │
              └────────┬────────┘
                       │ generates
              ┌────────┴────────┐
              │  dnsmasq config │  /etc/local-dns/run/zones.conf
              │  (dnsmasq       │
              │   format)       │
              └────────┬────────┘
                       │ loaded by
              ┌────────┴────────┐
              │  dnsmasq:5354   │  local-dnsmasq service
              └────────┬────────┘
                       │ routed by upstream resolver
           ┌───────────┴──────────────┐
           │                          │
     custom TLDs                everything else
  (.test, .dev, .local)        forwarded to upstream
```

### Ports

| Port | Service | Purpose |
|------|---------|---------|
| 5354 | local-dnsmasq | Serves custom local DNS zones |
| 53 | dnsdist (optional) | Existing DNS load balancer |
| 5053 | dnscrypt-proxy (optional) | Encrypted upstream DNS |

### Database

Entries are stored in SQLite3 at `/etc/local-dns/local-dns.db`:

```sql
-- Profiles for different environments
CREATE TABLE profiles (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT UNIQUE NOT NULL,
    is_active  INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- DNS entries per profile
CREATE TABLE entries (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id INTEGER NOT NULL,
    domain     TEXT NOT NULL,
    ip         TEXT NOT NULL,
    comment    TEXT,
    sort_key   TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);
```

Query the database directly:

```bash
sudo sqlite3 /etc/local-dns/local-dns.db "SELECT * FROM entries;"
sudo sqlite3 /etc/local-dns/local-dns.db "SELECT * FROM profiles;"
```

## Installation

### Prerequisites

```bash
sudo apt install dnsmasq
```

### Build from Source

```bash
git clone https://github.com/veduket/local-dns.git
cd local-dns
cargo build --release
sudo cp target/release/local-dns /usr/local/bin/
```

### Install via Cargo

```bash
cargo install local-dns
```

## Integration with Existing DNS Stacks

### With dnsdist (Recommended)

If you run dnsdist (as a load balancer in front of dnscrypt-proxy, for example):

1. Run `sudo local-dns init` — it detects dnsdist automatically
2. Add the provided Lua snippet to `/etc/dnsdist/dnsdist.conf`
3. Restart dnsdist: `sudo systemctl restart dnsdist`

The snippet forwards `.test`, `.dev`, `.local`, `.localhost` queries to `127.0.0.1:5354`.

### With systemd-resolved

1. Run `sudo local-dns init`
2. Configure resolved to forward custom domains:

```bash
sudo resolvectl dns lo 127.0.0.1
sudo resolvectl domain lo '~test' '~dev' '~local' '~localhost'
```

This tells resolved to query local-dnsmasq for those TLDs.

### With dnscrypt-proxy only (no dnsdist)

1. Run `sudo local-dns init`
2. The tool detects dnscrypt-proxy and configures dnsmasq to use it as upstream
3. Point your system's DNS to `127.0.0.1:5354`

If `/etc/resolv.conf` is managed by a resolver, add:

```
nameserver 127.0.0.1
```

to the top of the file, or configure your network manager to use `127.0.0.1` as DNS.

### Standalone (no upstream resolver)

local-dnsmasq always forwards unknown queries to an upstream. If you don't have one, it will fail to resolve internet domains. Make sure `server=` in `/etc/local-dns/dnsmasq.conf` points to a working DNS server.

## Systemd Service Management

```bash
# Start the service
sudo systemctl start local-dnsmasq

# Enable at boot
sudo systemctl enable local-dnsmasq

# Reload zones (zero-downtime, SIGHUP)
sudo systemctl reload local-dnsmasq

# Restart (drops cache)
sudo systemctl restart local-dnsmasq

# View logs
sudo journalctl -u local-dnsmasq -f
```

### Reload Mechanism

When you run `sudo local-dns apply` (automatic after add/remove/switch):

1. **SIGHUP** (systemd reload) — dnsmasq re-reads config files, zero dropped queries
2. **Direct kill -HUP** — if systemctl is unavailable but the PID file exists
3. **Full restart** — last resort, brief interruption

## Logging

All DNS queries are logged to `/var/log/local-dns.log` by default.

### Log Format

```
May 19 01:00:00 dnsmasq[12345]: query[A] myapp.test from 127.0.0.1
May 19 01:00:00 dnsmasq[12345]: forwarded myapp.test to 127.0.0.1#5053
May 19 01:00:00 dnsmasq[12345]: reply myapp.test is 127.0.0.1
May 19 01:00:00 dnsmasq[12345]: query[A] nonexistent.test from 127.0.0.1
May 19 01:00:00 dnsmasq[12345]: config nonexistent.test is NXDOMAIN
```

### CLI Log Viewing

```bash
# Recent queries
sudo local-dns logs

# Follow in real-time
sudo local-dns logs -f

# Errors only
sudo local-dns logs -e

# More lines
sudo local-dns logs -l 200
```

### Log Rotation

Add a logrotate config at `/etc/logrotate.d/local-dns`:

```
/var/log/local-dns.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    postrotate
        systemctl reload local-dnsmasq > /dev/null 2>&1 || true
    endscript
}
```

## Troubleshooting

### local-dnsmasq won't start

Check port conflicts:

```bash
sudo ss -tlnp | grep ':5354'
```

If another process is on port 5354, either stop it or change the port in `/etc/local-dns/dnsmasq.conf`.

Check config syntax:

```bash
dnsmasq --test --conf-file=/etc/local-dns/dnsmasq.conf
```

### DNS resolution fails for custom domains

Verify dnsmasq is running:

```bash
sudo local-dns status
```

Test directly:

```bash
dig myapp.test @127.0.0.1 -p 5354
```

If this works but `ping myapp.test` doesn't, the upstream resolver (dnsdist, systemd-resolved) isn't forwarding .test domains to port 5354.

### "Port 5354 is in use" during init

Something is already on port 5354. Either:

- Stop the conflicting service: `sudo systemctl stop <service>`
- Or change local-dns to a different port by editing `/etc/local-dns/dnsmasq.conf` and the upstream routing config

### No logs appear

Check that logging is enabled in the config:

```bash
grep 'log-queries\|log-facility' /etc/local-dns/dnsmasq.conf
```

The log file must be writable by the dnsmasq user:

```bash
sudo touch /var/log/local-dns.log
sudo chown dnsmasq:adm /var/log/local-dns.log
```

### Database corruption

Backup and reset:

```bash
sudo cp /etc/local-dns/local-dns.db /etc/local-dns/local-dns.db.bak
sudo rm /etc/local-dns/local-dns.db
sudo local-dns init
```

Then re-add entries from your backup.

## Security Considerations

- local-dns ports are bound to `127.0.0.1` only — not exposed to the network
- SQLite database permissions should be `640`, owned by root
- The systemd service runs dnsmasq as root (dnsmasq drops privileges automatically)
- Log files may contain query patterns; restrict access with `chown dnsmasq:adm`

## Backup & Restore

### Backup

```bash
# Database
sudo cp /etc/local-dns/local-dns.db /backup/local-dns-$(date +%F).db

# Config
sudo cp /etc/local-dns/dnsmasq.conf /backup/

# Full directory
sudo tar czf /backup/local-dns-$(date +%F).tar.gz /etc/local-dns/
```

### Restore

```bash
sudo systemctl stop local-dnsmasq
sudo cp /backup/local-dns.db /etc/local-dns/local-dns.db
sudo systemctl start local-dnsmasq
sudo local-dns apply
```

## Filesystem Layout

```
/etc/local-dns/
├── local-dns.db          # SQLite3 database (profiles + entries)
├── dnsmasq.conf           # Main dnsmasq configuration
├── run/
│   └── zones.conf         # Generated zones (auto-managed)
└── profiles/              # Legacy (not used with SQLite)
    └── active -> ...

/run/local-dns/
└── dnsmasq.pid            # PID file for SIGHUP reloads

/var/log/local-dns.log     # Query log
```
