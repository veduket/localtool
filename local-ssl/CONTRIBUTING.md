# Contributing to local-ssl

Thank you for considering contributing to local-ssl! This document outlines the guidelines for contributions.

## Code of Conduct

By participating, you agree to maintain a respectful, inclusive environment for everyone.

## How to Contribute

### Reporting Bugs

1. Check existing issues to avoid duplicates
2. Use the bug report template (`.github/ISSUE_TEMPLATE/bug_report.md`)
3. Include:
   - Your OS and version (`cat /etc/os-release`)
   - local-ssl version (`local-ssl --version`)
   - Output of `sudo local-ssl status`
   - Steps to reproduce
   - Expected vs actual behavior

### Suggesting Features

1. Use the feature request template (`.github/ISSUE_TEMPLATE/feature_request.md`)
2. Describe the problem you're solving, not just the solution you want
3. Include examples of how the feature would work

### Pull Requests

1. Fork the repository
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Follow code style guidelines
4. Write or update tests
5. Ensure the build is clean: `cargo build` (no warnings)
6. Run tests: `cargo test`
7. Format code: `cargo fmt`
8. Run clippy: `cargo clippy --all-targets`
9. Submit the PR with a clear description

## Development Setup

```bash
# Clone your fork
git clone https://github.com/veduket/local-ssl.git
cd local-ssl

# Build
cargo build

# Test your changes
sudo ./target/debug/local-ssl init
sudo ./target/debug/local-ssl generate myapp.test
sudo ./target/debug/local-ssl list
sudo ./target/debug/local-ssl show myapp.test
sudo ./target/debug/local-ssl status

# Clean up test CA
sudo rm -rf /etc/local-ssl/
```

## Code Style

- **Formatting**: Run `cargo fmt` before committing
- **Warnings**: The build must produce zero warnings
- **Clippy**: Run `cargo clippy --all-targets` and address all suggestions
- **Error handling**: Use `Result<String, String>` for all commands. The `main()` function prints errors with `Error: ` prefix in red.
- **Naming**: Commands are verbs (`init`, `generate`, `list`, `show`, `trust`, `status`). Options are lowercase with hyphens.
- **Modules**: Keep modules focused on one responsibility (ca.rs for CA lifecycle, cert.rs for certificate generation/inspection, trust.rs for system trust integration, util.rs for helpers).
- **Comments**: Avoid inline comments for obvious code.

## Certificate Expiry Policy

| Certificate | Validity | Rationale |
|-------------|----------|-----------|
| Root CA | 10 years | Rarely changes — reissuing requires re-trusting on all machines |
| Server certs | 1 year | Easy to regenerate — shorter expiry encourages automation |

## Migration Policy

local-ssl stores its state in `/etc/local-ssl/` (or `%PROGRAMDATA%\local-ssl\` on Windows). If the data format changes:

1. Never break backward compatibility for existing CA stores
2. Add a version file (`/etc/local-ssl/version`) if schema changes are needed
3. Provide a migration path with clear documentation

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cert_san_generation() {
        let domains = vec!["api.test".to_string()];
        // ... assert SANs include "api.test" and "*.api.test"
    }
}
```

### Manual Testing Checklist

- [ ] `sudo local-ssl init` creates CA without errors
- [ ] `sudo local-ssl status` shows CA details and "trusted" status
- [ ] `sudo local-ssl generate myapp.test` creates cert/key files
- [ ] `sudo local-ssl list` shows the generated domain
- [ ] `sudo local-ssl show myapp.test` displays parsed certificate fields
- [ ] `sudo local-ssl generate api.test www.test` generates cert with both SANs
- [ ] `openssl verify -CAfile /etc/local-ssl/ca-cert.pem /etc/local-ssl/certs/myapp.test/cert.pem` returns OK
- [ ] `sudo local-ssl trust` reinstalls trust without errors
- [ ] `curl --cacert /etc/local-ssl/ca-cert.pem https://myapp.test/` works with local HTTPS server
- [ ] Second `sudo local-ssl init` shows "CA already exists" message
- [ ] `sudo local-ssl generate` without CA returns helpful error

## Release Checklist

1. Update version in `Cargo.toml`
2. Update `ADMIN_GUIDE.md` if paths or commands changed
3. Run `cargo build --release` and verify
4. Run `cargo test` and `cargo clippy`
5. Commit with `cargo update` if dependencies changed
6. Tag: `git tag v0.1.1`
7. Push tags: `git push origin main --tags`
8. Create GitHub release with binary attachments

## Getting Help

- Open a GitHub issue for questions
- Tag with `question` label

---

Thank you for contributing! Made in Ethiopia with love by **Yared Getachew** and **OpenCode (Big Pickle)**.
