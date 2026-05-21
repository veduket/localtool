# Installation

## Prerequisites

- Rust 1.75+
- Linux, macOS, or Windows
- `sudo` access (for system DNS configuration and CA trust installation)

## Install from source

```bash
# Clone the repo
git clone https://github.com/veduket/localtool.git
cd localtool

# Build release
cargo build --release

# Install system-wide
sudo cp target/release/localtool /usr/local/bin/
```

## Install via cargo

```bash
cargo install localtool
```

## Verify installation

```bash
localtool --help
```

You should see the help output listing `dns` and `ssl` subcommands.

## Dependencies

localtool requires these system tools depending on the subcommand:

| Subcommand | Dependency | Notes |
|---|---|---|
| `localtool dns` | `dnsmasq` | DNS server, started by `init` |
| `localtool ssl` | `openssl` | Used for trust detection only |

Install dnsmasq:

```bash
# Debian / Ubuntu / Zorin
sudo apt install dnsmasq

# macOS
brew install dnsmasq

# Fedora / RHEL
sudo dnf install dnsmasq

# Arch
sudo pacman -S dnsmasq
```
