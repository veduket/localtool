# Contributing to local-dns

Thank you for considering contributing to local-dns! This document outlines the guidelines for contributions.

## Code of Conduct

By participating, you agree to maintain a respectful, inclusive environment for everyone.

## How to Contribute

### Reporting Bugs

1. Check existing issues to avoid duplicates
2. Use the bug report template (`.github/ISSUE_TEMPLATE/bug_report.md`)
3. Include:
   - Your OS and version (`cat /etc/os-release`)
   - local-dns version (`local-dns --version`)
   - Output of `sudo local-dns detect`
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
8. Submit the PR with a clear description

## Development Setup

```bash
# Clone your fork
git clone https://github.com/veduket/local-dns.git
cd local-dns

# Build
cargo build

# Test your changes
sudo ./target/debug/local-dns detect
sudo ./target/debug/local-dns add test.dev 127.0.0.1
```

## Code Style

- **Formatting**: Run `cargo fmt` before committing
- **Warnings**: The build must produce zero warnings
- **Single file**: Keep `main.rs` under 1200 lines. Split into modules if it exceeds this.
- **Error handling**: Use `Result<String, String>` for all commands. The `main()` function prints errors with `Error: ` prefix.
- **Naming**: Commands are verbs (`add`, `remove`, `list`). Options are lowercase with hyphens.
- **Comments**: Use section headers (`// ═══ ... ═══`) for major code sections. Avoid inline comments for obvious code.

## Database Migrations

If you change the SQLite schema:

1. Add a `PRAGMA user_version = N;` to track schema version
2. Write migration functions that check `user_version` and upgrade sequentially
3. Test with an existing database file

Example:

```rust
fn migrate_db(conn: &Connection) -> Result<(), String> {
    let version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap_or(0);
    if version < 1 {
        conn.execute_batch("CREATE TABLE IF NOT EXISTS ...").unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
    }
    if version < 2 {
        conn.execute_batch("ALTER TABLE ...").unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();
    }
    Ok(())
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dnsmasq_conf_generation() {
        // ...
    }
}
```

### Manual Testing Checklist

- [ ] `sudo local-dns detect` shows correct system state
- [ ] `sudo local-dns init` completes without errors
- [ ] `sudo local-dns add myapp.test 127.0.0.1` creates entry
- [ ] `sudo local-dns list` shows the entry
- [ ] `dig myapp.test @127.0.0.1 -p 5354` returns the IP
- [ ] `sudo local-dns remove myapp.test` removes entry
- [ ] `sudo local-dns profile create/swtich/list/delete` all work
- [ ] `sudo local-dns status` shows correct state
- [ ] `sudo local-dns logs` shows query log entries
- [ ] Profile switching reloads zones correctly

## Release Checklist

1. Update version in `Cargo.toml`
2. Update `ADMIN_GUIDE.md` if configuration changed
3. Run `cargo build --release` and verify
4. Commit and tag
5. Push tag to GitHub
6. Create GitHub release with binary attachments

## Getting Help

- Open a GitHub issue for questions
- Tag with `question` label

Thank you for contributing!
