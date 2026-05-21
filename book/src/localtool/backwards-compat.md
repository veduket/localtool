# Backwards Compatibility

The standalone `local-dns` and `local-ssl` binaries continue to work as before. They are now thin wrappers around the same library code that powers `localtool`.

## How it works

```
localtool dns add myapp.test 127.0.0.1
    ↓
localtool binary → local-dns library (lib.rs) → run(args)

local-dns add myapp.test 127.0.0.1
    ↓
local-dns binary (bin.rs) → local-dns library (lib.rs) → run(args)
```

Both call the same `local_dns::run()` or `local_ssl::run()` function. The standalone binaries are just entry points.

## Impact

- All existing scripts using `local-dns` or `local-ssl` continue to work
- No behavior changes
- Configuration and data files are shared (same paths: `/etc/local-dns/`, `/etc/local-ssl/`)
- Adding new features to both paths is automatic — they share the same library

## Migration

To migrate from standalone binaries to the unified CLI:

```bash
# Old way (still works)
local-dns add myapp.test 127.0.0.1
local-ssl generate myapp.test

# New way
localtool dns add myapp.test 127.0.0.1
localtool ssl generate myapp.test
```

The standalone binaries are **deprecated** and will receive only critical bug fixes. All new development is on `localtool`.
