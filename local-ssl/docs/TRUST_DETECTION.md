# Trust Detection Strategies

How `local-ssl status` and `local-ssl trust` determine whether the CA is installed in the system trust store.

---

## Option A — Search CA Bundle (current)

Search for the CA certificate's PEM content inside the system's CA bundle file.

**How it works:**
1. Read the CA cert PEM at `/etc/local-ssl/ca-cert.pem`
2. Try each known CA bundle path (`/etc/ssl/certs/ca-certificates.crt`, etc.)
3. Check if the CA cert's PEM block (or a unique substring) appears in the bundle

**Pros:**
- Works on any distro where `update-ca-certificates` appends new certs to a bundle
- No external commands needed; pure `std::fs`
- One `read_to_string` call — fast
- Covers all major families (Debian, RHEL, SUSE, Arch)

**Cons:**
- Requires knowing the bundle path for each distro family
- Cert content may not be byte-for-byte identical in the bundle (line ending differences, re-encoding)
- Doesn't detect certs installed via alternative mechanisms (p11-kit, manual symlink)

---

## Option B — Hash Symlink Check

Check for the OpenSSL hash symlink in the system's CA directory.

**How it works:**
1. `openssl x509 -in ca-cert.pem -hash -noout` → hash string
2. Look for `{hash}.0` or `{hash}.p11-kit` in:
   - `/etc/ssl/certs/`
   - `/usr/local/share/ca-certificates/`
   - `/etc/pki/ca-trust/source/anchors/`
   - `/etc/ca-certificates/trust-source/anchors/`

**Pros:**
- The definitive check on most Linux distros
- Standard OpenSSL mechanism — hash symlinks are the native trust indicator
- Works reliably when the cert was installed at the same path

**Cons:**
- Requires knowing where the OS places symlinks (varies by distro)
- May miss on systems that use p11-kit or trust-store in a non-standard location
- Requires `openssl` to be installed
- Previously missed `/etc/ssl/certs` on Debian/Ubuntu (false negative)

---

## Option C — `openssl verify` Chain Test

Generate a throwaway cert signed by the CA, then verify it against the system trust store.

**How it works:**
1. Create a temporary key and CSR
2. Sign with the CA to produce a leaf cert (or use `openssl req -CA ...`)
3. Run `openssl verify <leaf.pem>` — if OK, CA is trusted
4. Clean up temp files

**Pros:**
- Fully portable — no path guessing at all; openssl uses its compiled-in default CA path
- End-to-end test: actually proves a cert signed by our CA is trusted
- No distro-specific logic needed

**Cons:**
- Slow: requires generating keys and signing (multiple openssl invocations)
- Requires `openssl` on the system
- Temp file cleanup required
- openssl default CA path may not be configured on all systems (need `-CApath` fallback)
- More complex implementation

---

## Option D — `trust list` / p11-kit

Use the `p11-kit` trust module to list trusted certificates.

**How it works:**
1. `trust list --filter=ca-anchors` or `trust list | grep <subject>`
2. Check if our CA's subject or fingerprint appears in the output

**Pros:**
- Cross-distro (p11-kit is standard on modern Fedora, Arch, OpenSUSE)
- Doesn't depend on file paths
- Also works on some embedded Linux systems

**Cons:**
- Requires `p11-kit` / `trust` utility (not installed on minimal systems or older Debian)
- Output parsing is brittle (human-readable format)
- Not available on macOS or Windows
