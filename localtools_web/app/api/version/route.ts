import { NextResponse } from 'next/server'

export async function GET() {
  try {
    const res = await fetch(
      'https://api.github.com/repos/veduket/localtool/releases/latest',
      {
        headers: { 'Accept': 'application/vnd.github.v3+json', 'User-Agent': 'localtool' },
        next: { revalidate: 3600 },
      }
    )
    if (!res.ok) {
      return NextResponse.json({ version: 'v0.1.1', error: true })
    }
    const data = await res.json()
    return NextResponse.json({ version: data.tag_name })
  } catch {
    return NextResponse.json({ version: 'v0.1.1' })
  }
}
