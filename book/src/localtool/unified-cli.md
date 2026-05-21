# localtool — Unified CLI

`localtool` is the single binary that replaces both `local-dns` and `local-ssl`.

## Usage

```bash
localtool <subcommand> <args...>
```

## Subcommands

### `dns`

All DNS management commands. See [DNS Commands](../dns/commands.md).

```bash
localtool dns init
localtool dns add myapp.test 127.0.0.1
localtool dns status
```

### `ssl`

All SSL certificate commands. See [SSL Commands](../ssl/commands.md).

```bash
localtool ssl init
localtool ssl generate myapp.test
localtool ssl status
```

### `help`

Print help information.

```bash
localtool help
localtool dns help
localtool ssl help
```

## Telemetry

Both tools send anonymous usage data (UUID, command name, tool name) via HTTP POST. No PII is collected. Telemetry is opt-in and can be disabled:

```bash
localtool dns telemetry disable
localtool ssl telemetry disable
```
