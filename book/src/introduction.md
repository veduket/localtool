# Introduction

**localtool** is a unified CLI for local development infrastructure. It combines two complementary tools under one command:

- **`localtool dns`** — manage local DNS resolution with profiles, wildcards, zones, and groups
- **`localtool ssl`** — generate locally-trusted HTTPS certificates for any domain

Together they let you use realistic domains like `myapp.test` with fully trusted HTTPS on your local machine — no cloud dependencies, no configuration headaches.

## Why localtool?

Modern web development requires:

- **Realistic domains** — OAuth providers demand HTTPS redirect URIs, but `localhost` breaks OAuth flows
- **Wildcard routing** — microservices with 10+ subdomains need `*.test` → `127.0.0.1`, impossible with `/etc/hosts`
- **Trusted HTTPS** — Service Workers, WebRTC, and geolocation require HTTPS even in development
- **Zero conflicts** — your existing DNS setup (dnsdist, dnscrypt-proxy, systemd-resolved) should keep working

localtool solves all of this with a single CLI.

## How it works

```
localtool dns init    → starts dnsmasq on port 5354
localtool dns add     → registers a domain in SQLite-backed DNS
localtool ssl init    → generates a local Certificate Authority
localtool ssl generate → issues a signed cert for any domain
```

## Project structure

localtool is a Cargo workspace with three crates:

| Crate | Purpose |
|---|---|
| `local-dns` | DNS management library + standalone binary (deprecated) |
| `local-ssl` | SSL certificate library + standalone binary (deprecated) |
| `localtool` | Unified CLI wrapping both libraries |

The standalone `local-dns` and `local-ssl` binaries still work as thin wrappers.
