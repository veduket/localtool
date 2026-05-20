---
name: Bug Report
about: Report a bug to help us improve local-dns
title: ''
labels: bug
assignees: ''
---

## Bug Description

A clear and concise description of the bug.

## System Information

- OS: [e.g. Ubuntu 24.04]
- local-dns version: [`local-dns --version`]
- dnsmasq version: [`dnsmasq --version`]

## Detection Output

```
# Run and paste: sudo local-dns detect
```

## Steps to Reproduce

1. Run `...`
2. Run `...`
3. See error

## Expected Behavior

What should have happened.

## Actual Behavior

What actually happened. Include error messages and logs.

```
sudo local-dns logs -e
sudo journalctl -u local-dnsmasq --no-pager -n 50
```

## Additional Context

- Does this happen consistently?
- Any recent changes to your DNS setup?
