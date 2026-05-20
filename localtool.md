# OpenCode Prompt: localtool — Unification, DNS Repair, Sudo‑free SSL, and Future‑proof Architecture

> **Context:** This prompt captures everything we’ve designed in this session. It is intended for an AI‑powered coding agent (OpenCode) to implement the complete vision.

---

## 1. Background

You are given two existing Rust projects:

- **`local-dns`** — a DNS management tool that runs a `dnsmasq` instance on port `5354`, manages custom domains via a SQLite database, supports profiles, zones, wildcards, and is designed to integrate with system resolvers (`systemd-resolved`, `dnsdist`, `dnscrypt-proxy`, `bind9`, etc.).
- **`local-ssl`** — a pure‑Rust (using `rcgen`) SSL certificate generator that creates a local Certificate Authority, installs it into the system trust store, and issues certificates (with automatic wildcard SANs).

Together, they allow developers to use realistic domains like `myapp.test` with fully trusted HTTPS on their local machine. However, the current workflow has several pain points:

1. Users with custom DNS setups (e.g., `dnsdist` fronting `dnscrypt-proxy`) cannot resolve local development domains because the forwarding rules are missing.
2. Both tools require `sudo` for many operations (adding DNS entries that modify system config, installing CA system‑wide).
3. Two separate binaries mean more installation steps and mental overhead.
4. No easy way to get a “one‑shot” development environment.

This prompt describes how to evolve the project into a single `localtool` binary, fix the DNS forwarding problem automatically, make SSL trust work without `sudo`, and lay the groundwork for a future containerised development environment — all while preserving the Unix philosophy of “do one thing, do it well” and minimising destruction to the existing codebase.

---

## 2. Goals

1. **Single binary** (`localtool`) with subcommands `dns` and `ssl` that completely replace the separate `local-dns` and `local-ssl` binaries (while keeping them as deprecated wrappers for one release cycle).
2. **`localtool dns doctor --fix`** — a command that automatically detects and repairs the DNS forwarding chain (especially for `dnsdist` and `dnscrypt-proxy`) so that custom TLDs are resolved without manual editing.
3. **Sudo‑less SSL trust** — by default, the local CA is installed into the **user’s** personal trust store (no `sudo` required for daily use). System‑wide installation remains an option when running as root.
4. **Minimal code destruction** — convert `local-dns` and `local-ssl` into libraries; keep their internals intact.
5. **Backwards compatibility** — the old `local-dns` and `local-ssl` binaries (or symlinks) continue to work, delegating to `localtool`.
6. **Future‑proof structure** — the new crate layout and command structure allow easy addition of future subcommands like `localtool setup` (combine DNS + SSL) or `localtool env` (containerised development environment).

---

## 3. Project Structure (Cargo Workspace)

Create a workspace with three crates:
