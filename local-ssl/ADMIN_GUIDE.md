# Admin Guide — local-ssl

Deployment, CA lifecycle management, troubleshooting, and integration for system administrators.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Installation](#installation)
- [CA Lifecycle](#ca-lifecycle)
- [Working with Certificates](#working-with-certificates)
- [Integration with local-dns](#integration-with-local-dns)
- [Troubleshooting](#troubleshooting)
- [Certificate Expiry](#certificate-expiry)
- [Security Considerations](#security-considerations)
- [Backup & Restore](#backup--restore)

## Architecture Overview

```
┌──────────────────────────────────────────────────────────┐
│                    local-ssl (CLI)                        │
│  init | generate | list | show | trust | status          │
└──────────────────────┬───────────────────────────────────┘
                       │ reads/writes
              ┌────────┴────────┐
              │  /etc/local-ssl/ │  PEM files on disk
              │  ├── ca-key.pem  │  CA private key (RSA 4096)
              │  ├── ca-cert.pem │  CA certificate (self-signed, 10yr)
              │  └── certs/      │  Generated server certificates
              │       ├── myapp.test/
              │       │   ├── cert.pem   │  Server cert (1yr)
              │       │   └── key.pem    │  Server private key
              │       └── api.test/
              └────────┬────────┘
                       │ installs trust
              ┌────────┴────────┐
              │  System Trust   │  update-ca-certificates / security / certutil
              │  Store          │
              └─────────────────┘
```

### Filesystem Layout

```
/etc/local-ssl/
├── ca-key.pem           # CA private key (KEEP SECURE — 0600 permissions)
├── ca-cert.pem          # CA certificate (distribute to team members)
├── version              # Schema version (future use)
└── certs/
    ├── <domain>/
    │   ├── cert.pem     # Server certificate
    │   └── key.pem      # Server private key
    └── ...
```

### Certificate Locations

| Component | Path | Permissions |
|-----------|------|-------------|
| CA key | `/etc/local-ssl/ca-key.pem` | `600` (root only) |
| CA cert | `/etc/local-ssl/ca-cert.pem` | `644` |
| Generated certs | `/etc/local-ssl/certs/<domain>/cert.pem` | `644` |
| Generated keys | `/etc/local-ssl/certs/<domain>/key.pem` | `644` |
| Trust anchor (Debian) | `/usr/local/share/ca-certificates/local-ssl.crt` | `644` |
| Trust anchor (Fedora) | `/etc/pki/ca-trust/source/anchors/local-ssl.pem` | `644` |

## Installation

### Prerequisites

- Rust 1.75+ (see [rustup.rs](https://rustup.rs))
- `sudo` access (for writing to `/etc/local-ssl/` and system trust anchors)
- No system dependencies — pure Rust, no OpenSSL, no libssl-dev

### Build from Source

```bash
git clone https://github.com/veduket/local-ssl.git
cd local-ssl
cargo build --release
sudo cp target/release/local-ssl /usr/local/bin/
```

### Install via Cargo (future)

```bash
cargo install local-ssl
```

### Verify Installation

```bash
local-ssl --version
local-ssl --help
```

## CA Lifecycle

### Initializing the CA (One-Time Setup)

```bash
sudo local-ssl init
```

This creates a 4096-bit RSA CA key pair and self-signed certificate at `/etc/local-ssl/`, then installs the CA into the system trust store.

**What to expect:**

```
Generating local Certificate Authority...
✓ CA key:  /etc/local-ssl/ca-key.pem
✓ CA cert: /etc/local-ssl/ca-cert.pem

Installing CA into system trust store...
✓ CA trusted system-wide

Ready. Generate certs for local development with:
  local-ssl generate myapp.test
  local-ssl generate api.test www.test
```

### Checking CA Status

```bash
sudo local-ssl status
```

Output includes:
- Subject, issuer, serial number
- Validity period (not before / not after)
- System trust status (trusted ✓ or not trusted)
- Count of generated certificates

### Regenerating the CA

If the CA key is compromised, expired (after 10 years), or you need a fresh start:

```bash
# 1. Remove old CA data
sudo rm -rf /etc/local-ssl/

# 2. Remove old CA from system trust (manual)
# Debian/Ubuntu:
sudo rm -f /usr/local/share/ca-certificates/local-ssl.crt
sudo update-ca-certificates --fresh

# Fedora/RHEL:
sudo rm -f /etc/pki/ca-trust/source/anchors/local-ssl.pem
sudo update-ca-trust force

# Arch/openSUSE:
sudo rm -f /usr/share/pki/trust/anchors/local-ssl.pem
sudo trust extract-compat

# macOS:
sudo security delete-certificate -c "local-ssl Development CA" /Library/Keychains/System.keychain

# Windows:
certutil -delstore Root "local-ssl Development CA"

# 3. Reinitialize
sudo local-ssl init
```

**Warning**: Regenerating the CA invalidates all previously issued server certificates. You must regenerate every cert after CA renewal.

### Reinstalling Trust

If system trust was removed or corrupted:

```bash
sudo local-ssl trust
```

This re-copies the CA cert to the system trust anchors and runs the update command without regenerating the CA.

## Working with Certificates

### Generating Certificates

```bash
# Single domain (includes *.domain as SAN)
sudo local-ssl generate myapp.test

# Multiple SANs
sudo local-ssl generate api.test www.test admin.test
```

Generated files:
- `/etc/local-ssl/certs/myapp.test/cert.pem`
- `/etc/local-ssl/certs/myapp.test/key.pem`

### Listing All Certificates

```bash
sudo local-ssl list
```

### Viewing Certificate Details

```bash
sudo local-ssl show myapp.test
```

Example output:

```
Domain:        myapp.test
Issuer:        local-ssl Development CA
Valid from:    2026-05-19 12:00:00 +00:00:00
Valid until:   2027-05-19 12:00:00 +00:00:00
SANs:          myapp.test, *.myapp.test
Cert:          /etc/local-ssl/certs/myapp.test/cert.pem
Key:           /etc/local-ssl/certs/myapp.test/key.pem
```

### Using Certificates in Applications

**Node.js/Express:**

```javascript
const https = require('https');
const fs = require('fs');

https.createServer({
  key: fs.readFileSync('/etc/local-ssl/certs/myapp.test/key.pem'),
  cert: fs.readFileSync('/etc/local-ssl/certs/myapp.test/cert.pem'),
}, app).listen(443);
```

**Python/Flask:**

```python
import ssl
context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.load_cert_chain(
    '/etc/local-ssl/certs/myapp.test/cert.pem',
    '/etc/local-ssl/certs/myapp.test/key.pem'
)
```

**Caddy:**

```
myapp.test {
    tls /etc/local-ssl/certs/myapp.test/cert.pem /etc/local-ssl/certs/myapp.test/key.pem
    reverse_proxy localhost:3000
}
```

**nginx:**

```nginx
server {
    listen 443 ssl;
    server_name myapp.test *.myapp.test;
    ssl_certificate /etc/local-ssl/certs/myapp.test/cert.pem;
    ssl_certificate_key /etc/local-ssl/certs/myapp.test/key.pem;
}
```

### Testing with curl

```bash
# Verify TLS connection
curl --cacert /etc/local-ssl/ca-cert.pem https://myapp.test/

# Or if the CA is in the system trust store
curl https://myapp.test/
```

## Integration with local-dns

local-ssl pairs with [local-dns](https://github.com/veduket/local-dns) to provide the complete local development experience: name resolution + TLS trust.

### Setup

```bash
# 1. Install local-dns
git clone https://github.com/veduket/local-dns.git
cd local-dns
cargo build --release
sudo cp target/release/local-dns /usr/local/bin/

# 2. Initialize both tools
sudo local-dns init
sudo local-ssl init

# 3. Point a domain to localhost and get a trusted cert
sudo local-dns add myapp.test 127.0.0.1
sudo local-ssl generate myapp.test

# 4. Start your HTTPS server
# Cert: /etc/local-ssl/certs/myapp.test/cert.pem
# Key:  /etc/local-ssl/certs/myapp.test/key.pem

# 5. Access in browser
# https://myapp.test/  — resolves to localhost, trusted TLS
```

### Typical Workflow

```bash
# Start a new project
sudo local-dns add project.test 127.0.0.1
sudo local-ssl generate project.test

# Start your dev server with the generated certs
# Access at https://project.test/
```

## Troubleshooting

### Permission Denied

**Problem**: `local-ssl` commands fail with "Permission denied".

**Solution**: All `local-ssl` commands that modify files require root access:

```bash
# Correct
sudo local-ssl init
sudo local-ssl generate myapp.test
sudo local-ssl trust

# Read-only commands may work without sudo
local-ssl list
```

### Trust Not Working (Linux)

**Problem**: `sudo local-ssl status` shows "not trusted" after `init`.

**Steps**:

1. Check the trust anchor exists:

```bash
ls -la /usr/local/share/ca-certificates/local-ssl.crt
```

2. If missing, reinstall trust:

```bash
sudo local-ssl trust
```

3. If still failing, manually verify:

```bash
# Debian/Ubuntu
sudo cp /etc/local-ssl/ca-cert.pem /usr/local/share/ca-certificates/local-ssl.crt
sudo update-ca-certificates

# Fedora/RHEL
sudo cp /etc/local-ssl/ca-cert.pem /etc/pki/ca-trust/source/anchors/local-ssl.pem
sudo update-ca-trust

# Arch
sudo cp /etc/local-ssl/ca-cert.pem /usr/share/pki/trust/anchors/local-ssl.pem
sudo trust extract-compat
```

4. Restart your browser (browsers cache trust anchors at startup).

### Trust Not Working (macOS)

**Problem**: CA not trusted on macOS.

**Solution**:

```bash
# Verify the cert is in the System keychain
security find-certificate -c "local-ssl Development CA" /Library/Keychains/System.keychain

# If missing, install manually
sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain /etc/local-ssl/ca-cert.pem

# Restart Safari/Chrome
```

### Cert Not Found

**Problem**: `sudo local-ssl show <domain>` returns "No certificate for '<domain>'".

**Causes**:
- Domain never generated (check with `sudo local-ssl list`)
- Typo in domain name
- `/etc/local-ssl/certs/` was deleted or moved

**Solution**: Regenerate the certificate:

```bash
sudo local-ssl generate myapp.test
```

### OpenSSL Verification Fails

**Problem**: `openssl verify -CAfile /etc/local-ssl/ca-cert.pem ...` returns verification error.

**Check**:

1. Verify the CA is self-signed:

```bash
openssl x509 -in /etc/local-ssl/ca-cert.pem -text -noout | grep "CA:TRUE"
```

2. Verify the server cert was signed by this CA:

```bash
openssl x509 -in /etc/local-ssl/certs/myapp.test/cert.pem -text -noout | grep "Issuer"
```

The issuer should be `CN = local-ssl Development CA, O = local-ssl`.

3. Verify the chain:

```bash
openssl verify -CAfile /etc/local-ssl/ca-cert.pem /etc/local-ssl/certs/myapp.test/cert.pem
```

Should return `cert.pem: OK`.

### Browser Shows "Not Secure" Despite Trust

**Causes and solutions**:

1. **Browser started before CA was trusted** — restart the browser completely
2. **Using HTTP, not HTTPS** — ensure you're accessing `https://` (not `http://`)
3. **Wrong domain** — the domain in the URL must match one of the cert's SANs
4. **Chrome uses its own trust store** — on Linux, Chrome uses the NSS shared DB. Try:

```bash
# Add CA to Chrome's trust store
certutil -d sql:$HOME/.pki/nssdb -A -t "C,," -n "local-ssl Dev CA" -i /etc/local-ssl/ca-cert.pem
```

5. **Firefox uses its own trust store** — go to Preferences → Privacy & Security → Certificates → Import → select `/etc/local-ssl/ca-cert.pem` and check "Trust this CA to identify websites"

### CA Already Exists

Running `sudo local-ssl init` when a CA already exists shows:

```
CA already exists. Run `local-ssl trust` to reinstall trust.
```

To force regeneration, delete the CA first:

```bash
sudo rm -rf /etc/local-ssl/
sudo local-ssl init
```

## Certificate Expiry

| Certificate | Validity | Expires | Action Needed |
|-------------|----------|---------|---------------|
| Root CA | 10 years | ~2036 | Regenerate CA, re-issue all certs, re-trust on all machines |
| Server certs | 1 year | +1 year from generation | Regenerate with `sudo local-ssl generate <domain>` |

### Checking Expiry

```bash
# CA expiry
sudo local-ssl status

# Server cert expiry
sudo local-ssl show myapp.test
```

### Automated Renewal (cron)

Add a monthly cron job to regenerate certs that expire within 30 days:

```bash
# /etc/cron.monthly/local-ssl-renew
#!/bin/bash
for certdir in /etc/local-ssl/certs/*/; do
    domain=$(basename "$certdir")
    expiry=$(openssl x509 -in "$certdir/cert.pem" -noout -enddate | cut -d= -f2)
    expiry_epoch=$(date -d "$expiry" +%s)
    now_epoch=$(date +%s)
    days_left=$(( (expiry_epoch - now_epoch) / 86400 ))
    if [ "$days_left" -lt 30 ]; then
        sudo local-ssl generate "$domain"
    fi
done
```

## Security Considerations

- **CA private key** (`ca-key.pem`) is the root of trust for all development certificates. Protect it with `chmod 600`. Do not share it outside your team.
- **CA certificate** (`ca-cert.pem`) is safe to share. Team members can install it to trust certificates you sign.
- **Generated keys** are per-domain. Keep them private but note they're for local development only.
- **Never use these certificates in production.** The CA is self-signed and explicitly marked for development use only.
- **Root required** — local-ssl needs `sudo` to write to `/etc/local-ssl/` and modify system trust anchors. The binary drops privileges where possible.
- **No network exposure** — local-ssl operates entirely on local filesystem. Certificates are for local HTTPS servers only.

## Backup & Restore

### Backup

```bash
# Full backup
sudo tar czf /backup/local-ssl-$(date +%F).tar.gz /etc/local-ssl/

# Verify backup contents
tar tzf /backup/local-ssl-$(date +%F).tar.gz
```

### Restore

```bash
# Full restore
sudo systemctl stop your-https-server  # if using generated certs
sudo tar xzf /backup/local-ssl-2026-05-19.tar.gz -C /
sudo local-ssl trust                    # reinstall trust anchors
sudo systemctl start your-https-server
```

### Team Distribution of CA Cert

Share the CA certificate (not the key) so teammates can trust certs you generate:

```bash
# Export CA cert
cp /etc/local-ssl/ca-cert.pem ./team-ca-cert.pem

# Each teammate installs it
sudo local-ssl trust  # if they have local-ssl installed
# OR manually:
sudo cp team-ca-cert.pem /usr/local/share/ca-certificates/local-ssl.crt
sudo update-ca-certificates
```
