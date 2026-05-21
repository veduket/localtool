# Quick Start

Get a development domain up with HTTPS in under a minute.

## 1. Initialize DNS

```bash
sudo localtool dns init
```

This detects your system DNS setup (dnsdist, systemd-resolved, etc.), creates the SQLite database with a default profile and `global/main` zone/group, and configures dnsmasq on port 5354.

## 2. Add a domain

```bash
sudo localtool dns add myapp.test 127.0.0.1
```

## 3. Apply DNS

```bash
sudo localtool dns apply
```

Your system now resolves `myapp.test` to `127.0.0.1`.

```bash
ping myapp.test
# → PING myapp.test (127.0.0.1) ...
```

## 4. Initialize SSL CA

```bash
sudo localtool ssl init
```

Generates a local Certificate Authority and installs it in your system trust store.

## 5. Generate a cert

```bash
sudo localtool ssl generate myapp.test
```

Certificate and key are written to `/etc/local-ssl/certs/myapp.test/`.

## 6. Verify

```bash
localtool ssl status
# → System trust: trusted ✓

curl --cacert /etc/local-ssl/ca-cert.pem https://myapp.test/
```

## That's it

Your local domain `myapp.test` now resolves locally **and** has a trusted HTTPS certificate. Use it with any web server, framework, or tool.
