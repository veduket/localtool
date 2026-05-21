# CA Management

## CA Lifecycle

### Creation

The CA is created by `localtool ssl init`. It generates:

- A 4096-bit RSA key pair
- A self-signed certificate with 10-year validity
- Subject: `CN = local-ssl Development CA, O = local-ssl`

### Renewal

The CA is valid for 10 years. If it expires, re-run `init`:

```bash
# This overwrites the existing CA
sudo localtool ssl init
```

**Warning:** This invalidates all certificates signed by the previous CA. You'll need to regenerate them.

### Certificate expiry

Generated certificates are valid for 1 year. Check expiry:

```bash
localtool ssl check myapp.test
localtool ssl show myapp.test    # Shows validity dates
```

Regenerate an expiring cert:

```bash
sudo localtool ssl generate myapp.test
```

## System trust

The CA cert is copied to a system-specific location and the trust store is updated.

| Distro | CA copy location | Update command |
|---|---|---|
| Debian/Ubuntu | `/usr/local/share/ca-certificates/local-ssl.crt` | `update-ca-certificates` |
| Fedora/RHEL | `/etc/pki/ca-trust/source/anchors/local-ssl.pem` | `update-ca-trust` |
| Arch/openSUSE | `/usr/share/pki/trust/anchors/local-ssl.pem` | `trust extract-compat` |

If trust is lost (e.g., after OS upgrade), re-run:

```bash
sudo localtool ssl trust
```

## Backup

The CA key and certificate are at `/etc/local-ssl/`. Back them up:

```bash
tar czf local-ssl-ca-backup.tar.gz /etc/local-ssl/ca-key.pem /etc/local-ssl/ca-cert.pem
```

Without the CA key, you cannot issue new certificates or renew existing ones without creating a new CA (which requires re-trusting).
