# local-dns

Local DNS management for development machines. Profile-based DNS resolution with wildcards, zones, groups, and cross-platform system detection.

---

## Documentation

| Document | Description |
|---|---|
| [README](https://github.com/veduket/local-dns/blob/main/README.md) | Overview, installation, quick start |
| [INSTALL](https://github.com/veduket/local-dns/blob/main/INSTALL.md) | Detailed installation guide per platform |
| [COMMANDS](https://github.com/veduket/local-dns/blob/main/COMMANDS.md) | Full CLI command reference |
| [ADMIN_GUIDE](https://github.com/veduket/local-dns/blob/main/ADMIN_GUIDE.md) | Deployment, integration, troubleshooting |
| [DEVELOPER_GUIDE](https://github.com/veduket/local-dns/blob/main/DEVELOPER_GUIDE.md) | Architecture, building, extending |
| [CONTRIBUTING](https://github.com/veduket/local-dns/blob/main/CONTRIBUTING.md) | How to contribute |

## Quick reference

```
# Initialize configuration and services
local-dns init

# Add a DNS entry (zone and group are optional)
local-dns add myapp.test 127.0.0.1
local-dns add api.test 127.0.0.1 --zone services
local-dns add admin.test 127.0.0.1 --zone services --group internal

# List all entries
local-dns list

# Switch profile
local-dns profile create work
local-dns profile switch work

# Check system status
local-dns status
```

## Commands

| Command | Description |
|---|---|
| `init` | Initialize configuration, database, and services |
| `add <domain> <ip>` | Add a DNS entry |
| `remove <domain>` | Remove a DNS entry |
| `list` | List entries in active profile |
| `profile` | Manage profiles (create, switch, list, delete) |
| `zone` | Manage zones (create, list, delete, show) |
| `group` | Manage groups (create, list, delete) |
| `status` | Show system status |
| `apply` | Apply DNS configuration |
| `logs` | View dnsmasq logs |
| `detect` | Detect system DNS config |
| `telemetry` | Manage anonymous usage telemetry |

## Requirements

- **Linux**: dnsmasq, systemd or init.d
- **macOS**: dnsmasq via Homebrew
- **Windows**: dnsmasq via winget or manual install

## License

MIT &mdash; see [LICENSE](https://github.com/veduket/local-dns/blob/main/LICENSE).

---

*local-dns is now a library crate. Use the unified CLI: `localtool dns <command>`. See [github.com/veduket/localtool](https://github.com/veduket/localtool).*
