# DNS Overview

local-dns manages a private `dnsmasq` instance on port 5354, backed by SQLite. It gives you DNS resolution for any domain on your local machine — without touching `/etc/hosts`.

## Architecture

```
App → upstream resolver ─┬─→ *.test, *.dev → local-dnsmasq:5354
                         └─→ everything else → internet DNS
```

`localtool dns init` detects your system resolver and configures forwarding so that requests for your custom domains (`.test`, `.dev`, `.local`) are routed to dnsmasq while everything else continues working normally.

## Why not /etc/hosts?

| Problem | localtool dns solution |
|---|---|
| No wildcards | `localtool dns add "*.test" 127.0.0.1` |
| Hard to switch projects | Profiles: `localtool dns profile switch project-x` |
| No organization | Zones & Groups for hierarchy |
| No hot-reload | SIGHUP reloads config instantly |
| No integration | Auto-detects dnsdist, systemd-resolved, dnscrypt-proxy |

## Data storage

All entries are stored in SQLite at `/etc/local-dns/local-dns.db`. The schema supports:

- **Profiles** — isolated DNS configurations (work, personal, project-x)
- **Zones** — top-level organizational buckets (cities, teams, services)
- **Groups** — sub-divisions within zones (neighborhoods, microservices)

## Key ports

| Port | Service |
|---|---|
| 53 | System DNS (dnsdist, systemd-resolved, etc.) |
| 5053 | dnscrypt-proxy (if installed) |
| 5353 | avahi-daemon (mDNS) |
| 5354 | **local-dnsmasq** (free) |
