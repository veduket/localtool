import { NextResponse } from 'next/server'
import { getStats } from '@/lib/telemetry-store'

export async function GET() {
  try {
    const stats = await getStats()
    return NextResponse.json(stats, { status: 200 })
  } catch (err) {
    console.error('stats error:', err)
    return NextResponse.json({ error: 'failed to load stats' }, { status: 500 })
  }
}
