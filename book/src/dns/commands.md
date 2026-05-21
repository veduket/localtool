# DNS Commands

All DNS commands are available via `localtool dns <command>` or the standalone `local-dns <command>`.

## Core commands

### `init`

Initialize the database, configuration, and dnsmasq service.

```bash
sudo localtool dns init
```

Creates default profile, global zone, main group, and generates dnsmasq configuration.

### `add <domain> <ip>`

Add a DNS entry.

```bash
sudo localtool dns add myapp.test 127.0.0.1
sudo localtool dns add api.test 127.0.0.1 --zone services --group internal -c "API server"
sudo localtool dns add "*.test" 127.0.0.1   # wildcard
```

Options: `--zone`, `--group`, `--comment`, `--profile`

### `remove <domain>`

Remove a DNS entry.

```bash
sudo localtool dns remove myapp.test
```

### `list`

List all entries in the active profile.

```bash
localtool dns list
```

### `move <domain>`

Move an entry to a different zone or group.

```bash
sudo localtool dns move myapp.test --zone addis_ababa --group bole
```

### `copy <domain>`

Copy an entry to another zone/group.

```bash
sudo localtool dns copy myapp.test --zone addis_ababa --group kazanchis
```

### `edit <domain>`

Change an entry's IP or comment.

```bash
sudo localtool dns edit myapp.test --ip 127.0.0.2 --comment "moved to new IP"
```

## Profile commands

See [Profiles](profiles.md) for details.

```bash
localtool dns profile create work
localtool dns profile switch work
localtool dns profile list
localtool dns profile delete work
```

## Zone & Group commands

See [Zones & Groups](zones-groups.md) for details.

```bash
localtool dns zone create addis_ababa -d "Addis Ababa"
localtool dns zone list
localtool dns zone show addis_ababa
localtool dns zone delete addis_ababa

localtool dns group create bole --zone addis_ababa
localtool dns group list
localtool dns group delete bole --zone addis_ababa
```

## System commands

### `detect`

Detect DNS services running on the system.

```bash
sudo localtool dns detect
```

Shows port usage and discovered DNS services.

### `status`

Show dnsmasq service status and system detection summary.

```bash
sudo localtool dns status
```

### `apply`

Regenerate dnsmasq configuration from the database and send SIGHUP to reload.

```bash
sudo localtool dns apply
```

### `reset`

Delete the database and re-run init.

```bash
sudo localtool dns reset
```

### `logs`

View dnsmasq logs.

```bash
sudo localtool dns logs        # Recent 50 queries
sudo localtool dns logs -f     # Follow in real-time
sudo localtool dns logs -e     # Errors only (NXDOMAIN, REFUSED)
```
