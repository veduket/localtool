# Trust Detection

`localtool ssl status` and `localtool ssl trust` determine whether the CA is trusted by the system.

## Detection strategy

local-ssl uses a multi-strategy approach to detect trust, falling back through methods until one succeeds.

### Strategy B — Hash Symlink Check (current default)

Uses `openssl x509 -hash` to compute the CA cert's hash, then checks for the corresponding hash symlink in known CA directories.

**How it works:**
1. `openssl x509 -hash -noout -in /etc/local-ssl/ca-cert.pem` → hash string
2. Look for `{hash}.0` or `{hash}.p11-kit` in CA directories

**Works on:** Debian, Ubuntu, Fedora, RHEL, Arch, openSUSE

### Strategy A — CA Bundle Search

Searches for the CA cert PEM content inside the system's CA bundle file.

**How it works:**
1. Read the CA cert PEM
2. Try known CA bundle paths
3. Check if the PEM content appears in the bundle

### Strategy C — openssl verify Chain Test

Generates a throwaway cert signed by the CA and verifies it against the system trust store.

### Strategy D — p11-kit trust list

Uses the `p11-kit` trust module to list trusted certificates.

## Why multiple strategies?

Different Linux distributions store trusted CA certificates differently. Using a single detection method would fail on some systems. The fallback chain ensures maximum compatibility across distros.

## Troubleshooting

If `status` shows "not trusted" but you've run `trust`:

1. **Verify the CA is installed:**
   ```bash
   ls /usr/local/share/ca-certificates/local-ssl.crt
   ```

2. **Manually update trust:**
   ```bash
   sudo update-ca-certificates --fresh
   ```

3. **Check the hash symlink:**
   ```bash
   openssl x509 -hash -noout -in /etc/local-ssl/ca-cert.pem
   ls /etc/ssl/certs/ | grep <hash>
   ```

4. **Re-run trust:**
   ```bash
   sudo localtool ssl trust
   ```
