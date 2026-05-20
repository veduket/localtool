import { NextResponse } from 'next/server'

const REPOS = ['local-dns', 'local-ssl', 'localtools_web']

export async function GET() {
  try {
    const res = await fetch('https://api.github.com/users/veduket/repos?per_page=100&sort=updated', {
      headers: { 'Accept': 'application/vnd.github.v3+json' },
      next: { revalidate: 3600 },
    })
    if (!res.ok) {
      return NextResponse.json({ error: 'GitHub API error', status: res.status }, { status: 502 })
    }
    const data = await res.json()
    if (!Array.isArray(data)) {
      return NextResponse.json({ error: 'Unexpected response' }, { status: 502 })
    }
    const repos = data
      .filter((r: { name: string }) => REPOS.includes(r.name))
      .map((r: { name: string; stargazers_count: number; forks_count: number; html_url: string }) => ({
        name: r.name,
        stars: r.stargazers_count,
        forks: r.forks_count,
        url: r.html_url,
      }))
    const totals = { stars: repos.reduce((s: number, r: { stars: number }) => s + r.stars, 0), forks: repos.reduce((s: number, r: { forks: number }) => s + r.forks, 0) }
    return NextResponse.json({ repos, totals })
  } catch {
    return NextResponse.json({ error: 'Failed to fetch' }, { status: 502 })
  }
}
