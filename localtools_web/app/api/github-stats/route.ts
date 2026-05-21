import { NextResponse } from 'next/server'

export async function GET() {
  try {
    const res = await fetch('https://api.github.com/repos/veduket/localtool', {
      headers: { 'Accept': 'application/vnd.github.v3+json' },
      next: { revalidate: 3600 },
    })
    if (!res.ok) {
      return NextResponse.json({ error: 'GitHub API error', status: res.status }, { status: 502 })
    }
    const data = await res.json()
    return NextResponse.json({
      name: data.name,
      stars: data.stargazers_count,
      forks: data.forks_count,
      url: data.html_url,
    })
  } catch {
    return NextResponse.json({ error: 'Failed to fetch' }, { status: 502 })
  }
}
