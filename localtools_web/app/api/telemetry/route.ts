import { NextRequest, NextResponse } from 'next/server'
import { storeEvent, getStats } from '@/lib/telemetry-store'

export async function POST(req: NextRequest) {
  try {
    let data: Record<string, string>
    const ct = req.headers.get('content-type') || ''

    if (ct.includes('application/json')) {
      data = await req.json()
    } else {
      const text = await req.text()
      data = Object.fromEntries(new URLSearchParams(text))
    }

    const ip = req.headers.get('x-forwarded-for')?.split(',')[0]?.trim()
      || req.headers.get('x-real-ip')
      || ''

    const event = {
      uuid: data.uuid || 'unknown',
      tool: data.tool || 'unknown',
      version: data.version || '',
      os: data.os || '',
      event: data.event || 'command',
      command: data.command || '',
      age_days: parseInt(data.age_days || '0'),
      ts: parseInt(data.ts || '0'),
      zones: data.zones ? parseInt(data.zones) : undefined,
      groups: data.groups ? parseInt(data.groups) : undefined,
      entries: data.entries ? parseInt(data.entries) : undefined,
      certs: data.certs ? parseInt(data.certs) : undefined,
      ip,
    }

    await storeEvent(event)

    return NextResponse.json({ ok: true, message: 'Thank you for using localtools!' }, { status: 200 })
  } catch (err) {
    const msg = err instanceof Error ? err.message : 'unknown'
    console.error('telemetry error:', msg)
    return NextResponse.json({ ok: false, error: msg }, { status: 400 })
  }
}

export async function GET() {
  try {
    const stats = await getStats()
    return NextResponse.json(stats, { status: 200 })
  } catch (err) {
    console.error('stats error:', err)
    return NextResponse.json({ error: 'failed to load stats' }, { status: 500 })
  }
}
