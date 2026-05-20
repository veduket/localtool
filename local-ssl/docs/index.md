# local-ssl

Locally-trusted HTTPS certificates for development. Generate a Certificate Authority and issue certificates for any domain — all in pure Rust.

---

## Documentation

| Document | Description |
|---|---|---|
| [README](https://github.com/veduket/local-ssl/blob/main/README.md) | Overview, installation, quick start |
| [ADMIN_GUIDE](https://github.com/veduket/local-ssl/blob/main/ADMIN_GUIDE.md) | CA lifecycle, expiry management, troubleshooting |
| [DEVELOPER_GUIDE](https://github.com/veduket/local-ssl/blob/main/DEVELOPER_GUIDE.md) | Architecture, extending, cross-platform notes |
| [CONTRIBUTING](https://github.com/veduket/local-ssl/blob/main/CONTRIBUTING.md) | How to contribute |
| [TRUST_DETECTION](https://github.com/veduket/local-ssl/blob/main/docs/TRUST_DETECTION.md) | System trust detection strategies (A-D) with trade-offs |

## Quick reference

```
# Initialize CA and install system trust
sudo local-ssl init

# Generate certificate for a domain (includes wildcard SAN)
sudo local-ssl generate myapp.test

# Generate with multiple Subject Alternative Names
sudo local-ssl generate api.test www.test

# List all generated certificates
local-ssl list

# Show certificate details
local-ssl show myapp.test

# Check CA status
local-ssl status

# Reinstall system trust
sudo local-ssl trust
```

## Commands

| Command | Description |
|---|---|
| `init` | Initialize CA and install system trust |
| `generate <domains>` | Generate HTTPS certificates |
| `list` | List all generated certificates |
| `show <domain>` | Show certificate details |
| `trust` | Reinstall CA system trust |
| `status` | Show CA and certificate status |
| `telemetry` | Manage anonymous usage telemetry |

## File locations

```
/etc/local-ssl/
  ca-key.pem         CA private key
  ca-cert.pem        CA certificate
  certs/
    <domain>/
      cert.pem       Domain certificate
      key.pem        Domain private key
```

## License

MIT &mdash; see [LICENSE](https://github.com/veduket/local-ssl/blob/main/LICENSE).

---

*local-ssl is now a library crate. Use the unified CLI: `localtool ssl <command>`. See [github.com/veduket/localtool](https://github.com/veduket/localtool).*
