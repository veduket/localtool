export const dynamic = 'force-dynamic'

import { getStats } from '@/lib/telemetry-store'

export default async function StatsPage() {
  const stats = await getStats()

  const daysAgo = (ts: number) => {
    const diff = Date.now() / 1000 - ts
    const days = Math.floor(diff / 86400)
    if (days < 1) return 'today'
    if (days === 1) return '1 day ago'
    return `${days} days ago`
  }

  const formatDate = (ts: number) => {
    const d = new Date(ts * 1000)
    return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
  }

  return (
    <div className="px-6 pt-32 pb-24">
      <div className="max-w-6xl mx-auto">
        <div className="mb-16">
          <p className="text-xs font-medium text-white/30 tracking-[0.2em] uppercase mb-4">Telemetry</p>
          <h1 className="text-4xl sm:text-5xl font-bold tracking-tighter">Community stats</h1>
          <p className="mt-4 text-white/40 max-w-[50ch]">
            Anonymous usage statistics. No personal information is collected. All data is opt-in.
          </p>
        </div>

        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 mb-16">
          <StatCard label="Total users" value={stats.totalUsers.toLocaleString()} accent="#6366f1" />
          <StatCard label="Commands run" value={stats.totalCommands.toLocaleString()} accent="#22d3ee" />
          <StatCard label="Events received" value={stats.totalEvents.toLocaleString()} accent="#34d399" />
          <StatCard label="Tools tracked" value={Object.keys(stats.byTool).length.toString()} accent="#f59e0b" />
        </div>

        {Object.keys(stats.byTool).length > 0 && (
          <div className="mb-16">
            <h2 className="text-lg font-semibold mb-4">Per tool breakdown</h2>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              {Object.entries(stats.byTool).map(([tool, data]) => (
                <div key={tool} className="rounded-2xl border border-white/[0.04] bg-white/[0.02] p-6">
                  <p className="text-sm font-medium text-white/60 mb-3">{tool}</p>
                  <div className="flex items-baseline gap-6">
                    <div>
                      <p className="text-2xl font-bold tracking-tight">{data.users}</p>
                      <p className="text-xs text-white/30 mt-1">Users</p>
                    </div>
                    <div>
                      <p className="text-2xl font-bold tracking-tight">{data.commands.toLocaleString()}</p>
                      <p className="text-xs text-white/30 mt-1">Commands</p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        <div>
          <h2 className="text-lg font-semibold mb-4">Top users by tool age</h2>
          {stats.topUsers.length === 0 ? (
            <div className="rounded-2xl border border-white/[0.04] bg-white/[0.02] p-12 text-center">
              <p className="text-white/40">No telemetry data yet. Install the tools and use them!</p>
            </div>
          ) : (
            <div className="rounded-2xl border border-white/[0.04] overflow-hidden">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-white/[0.04] text-white/30 text-xs uppercase tracking-wider">
                    <th className="text-left px-5 py-4 font-medium">#</th>
                    <th className="text-left px-5 py-4 font-medium">Tool</th>
                    <th className="text-left px-5 py-4 font-medium">UUID</th>
                    <th className="text-left px-5 py-4 font-medium">Usage age</th>
                    <th className="text-left px-5 py-4 font-medium">Commands</th>
                    <th className="text-left px-5 py-4 font-medium">First seen</th>
                    <th className="text-left px-5 py-4 font-medium">Last seen</th>
                  </tr>
                </thead>
                <tbody>
                  {stats.topUsers.map((user, i) => (
                    <tr key={`${user.uuid}:${user.tool}`} className="border-b border-white/[0.02] hover:bg-white/[0.01] transition-colors">
                      <td className="px-5 py-4 text-white/40 font-mono text-xs">{i + 1}</td>
                      <td className="px-5 py-4">
                        <span className={`font-mono text-xs px-2 py-0.5 rounded-full ${
                          user.tool === 'local-dns' ? 'bg-[#22d3ee]/10 text-[#22d3ee]' : 'bg-[#34d399]/10 text-[#34d399]'
                        }`}>
                          {user.tool}
                        </span>
                      </td>
                      <td className="px-5 py-4 font-mono text-xs text-white/40">{user.uuid}</td>
                      <td className="px-5 py-4">
                        <span className="font-medium">{user.current_age_days}</span>
                        <span className="text-white/30 text-xs ml-1">days</span>
                      </td>
                      <td className="px-5 py-4 font-mono text-xs text-white/60">{user.total_commands}</td>
                      <td className="px-5 py-4 text-xs text-white/40">{formatDate(user.first_seen)}</td>
                      <td className="px-5 py-4 text-xs text-white/40">{daysAgo(user.last_seen)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

function StatCard({ label, value, accent }: { label: string; value: string; accent: string }) {
  return (
    <div className="rounded-2xl border border-white/[0.04] bg-white/[0.02] p-5">
      <p className="text-xs text-white/30 mb-1">{label}</p>
      <p className="text-2xl font-bold tracking-tight" style={{ color: accent }}>{value}</p>
    </div>
  )
}
