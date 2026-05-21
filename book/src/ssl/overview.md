# SSL Overview

local-ssl generates a private Certificate Authority on your machine, installs it in your system trust store, and issues trusted HTTPS certificates for any local development domain.

## Problem

Modern browsers block features over HTTP:

- Service Workers require HTTPS
- OAuth providers demand HTTPS redirect URIs
- WebRTC, geolocation, and other APIs are HTTP-only
- Self-signed certificates cause scary browser warnings

## Solution

```
localtool ssl init
  ├── Generates 4096-bit RSA key pair for CA
  ├── Creates self-signed CA certificate (10-year validity)
  ├── Installs CA in system trust store
  └── Ready to sign development certificates

localtool ssl generate myapp.test
  ├── Generates server key pair
  ├── Creates cert signed by local CA
  │   ├── CN = myapp.test
  │   ├── SANs = myapp.test, *.myapp.test
  │   └── 1-year validity
  ├── Writes cert.pem + key.pem
  └── Ready to use with any HTTPS server
```

## Certificate locations

| Component | Path |
|---|---|
| CA key | `/etc/local-ssl/ca-key.pem` |
| CA cert | `/etc/local-ssl/ca-cert.pem` |
| Generated certs | `/etc/local-ssl/certs/<domain>/cert.pem` |
| Generated keys | `/etc/local-ssl/certs/<domain>/key.pem` |

## Platform support

| Platform | CA Trust Mechanism |
|---|---|
| Debian/Ubuntu | `update-ca-certificates` |
| Fedora/RHEL | `update-ca-trust` |
| Arch/openSUSE | `trust extract-compat` |
| macOS | `security add-trusted-cert` |
| Windows | `certutil -addstore Root` |
