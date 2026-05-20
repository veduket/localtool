# localtools

Website and telemetry API for the localtools family of open-source developer tools.

**Live at:** [https://localtool.vercel.app](https://localtool.vercel.app)

## What is localtools?

localtools is a collection of CLI tools for local development infrastructure. Currently:

- **[local-dns](https://github.com/veduket/local-dns)** — Profile-based DNS management for your local machine. Route any domain to any IP with wildcards, zones, groups, and cross-platform system detection.

- **[local-ssl](https://github.com/veduket/local-ssl)** — Generate locally-trusted HTTPS certificates for development. Pure Rust (rcgen), cross-platform trust, auto-wildcard SANs on every cert.

More tools coming.

## What this website does

| Route | Description |
|---|---|
| `/` | Marketing landing page |
| `/docs` | Documentation hub for all localtools |
| `/docs/local-dns` | local-dns CLI reference |
| `/docs/local-ssl` | local-ssl CLI reference |
| `/stats` | Live community leaderboard |
| `POST /api/telemetry` | Anonymous usage telemetry endpoint |
| `GET /api/stats` | Raw JSON stats for the leaderboard |

## Tech stack

- **Framework:** Next.js 14 (App Router)
- **Styling:** Tailwind CSS 3
- **Fonts:** Plus Jakarta Sans (UI) + JetBrains Mono (code)
- **Deployment:** Vercel
- **Storage:** File-based JSON (development), ready for database migration

## Development

```bash
npm install
npm run dev
```

## Telemetry

The CLI tools (`local-dns`, `local-ssl`) send anonymous usage data to `/api/telemetry` when enabled. No personal information is collected. Users can opt out at any time with:

```bash
local-dns telemetry disable
local-ssl telemetry disable
```

## License

MIT — see [LICENSE](LICENSE).
