# SSL Commands

All SSL commands are available via `localtool ssl <command>` or the standalone `local-ssl <command>`.

## Core commands

### `init`

Generate a local Certificate Authority and install it in the system trust store.

```bash
sudo localtool ssl init
```

Creates `/etc/local-ssl/ca-key.pem` and `/etc/local-ssl/ca-cert.pem` (10-year validity).

### `generate <domains...>`

Generate HTTPS certificates signed by the local CA.

```bash
# Single domain (auto-includes *.domain as SAN)
sudo localtool ssl generate myapp.test

# Multiple SANs
sudo localtool ssl generate api.test www.test admin.test
```

Certificates are valid for 1 year and include:
- `CN = <domain>`
- `SANs = <domain>, *.<domain>`
- Extended Key Usage: Server Authentication, Client Authentication

## Management

### `list`

List all generated certificates.

```bash
localtool ssl list
```

### `show <domain>`

Show certificate details including subject, issuer, validity, and fingerprints.

```bash
localtool ssl show myapp.test
```

### `check <domain>`

Check certificate validity.

```bash
localtool ssl check myapp.test          # Local cert check
localtool ssl check api.test:443        # Remote TLS inspection
```

## Trust management

### `status`

Show CA and system trust status.

```bash
localtool ssl status
```

Output includes CA subject, validity dates, file locations, and whether the CA is trusted by the system.

### `trust`

Reinstall the CA into the system trust store. Useful after a system update or if trust was removed.

```bash
sudo localtool ssl trust
```

## Telemetry

```bash
localtool ssl telemetry status
localtool ssl telemetry enable
localtool ssl telemetry disable
```
