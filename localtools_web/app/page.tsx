'use client'

import { useEffect, useRef, useState } from 'react'

export default function Home() {
  return (
    <div className="overflow-hidden">
      <HeroSection />
      <VersionBadge />
      <ToolsOverview />
      <GitHubStatsSection />
      <StatsBanner />
      <PricingSection />
    </div>
  )
}

function HeroSection() {
  return (
    <section className="relative min-h-[100dvh] flex items-center px-6 pt-24">
      <div className="absolute inset-0 overflow-hidden pointer-events-none">
        <div className="absolute -top-1/2 -right-1/4 w-[800px] h-[800px] rounded-full bg-[#22d3ee]/5 blur-[120px]" />
        <div className="absolute -bottom-1/2 -left-1/4 w-[600px] h-[600px] rounded-full bg-[#6366f1]/5 blur-[120px]" />
      </div>

      <div className="max-w-6xl mx-auto w-full grid grid-cols-1 lg:grid-cols-2 gap-12 items-center">
        <div className="animate-[fade-in_0.8s_ease-out]">
          <div className="inline-flex items-center gap-2 rounded-full px-4 py-1.5 border border-white/[0.06] bg-white/[0.02] text-xs font-medium text-white/40 mb-8 tracking-wide uppercase">
            Open-source developer tools
          </div>
          <h1 className="text-4xl sm:text-5xl lg:text-7xl font-bold tracking-tighter leading-[0.95] text-balance">
            Local infrastructure<br />
            <span className="text-transparent bg-clip-text bg-gradient-to-r from-[#22d3ee] via-[#6366f1] to-[#34d399]">for developers</span>
          </h1>
          <p className="mt-6 text-lg text-white/40 leading-relaxed max-w-[45ch]">
            <strong className="text-white/70">localtool</strong> gives you production-grade DNS resolution and HTTPS certificates
            for local development. One CLI, two subcommands: <code className="text-white/60">dns</code> and <code className="text-white/60">ssl</code>.
            No cloud dependencies. No configuration headaches.
          </p>
          <div className="flex flex-wrap gap-4 mt-10">
            <a href="/docs" className="group relative inline-flex items-center gap-2 rounded-full px-6 py-3 bg-white text-[#050508] font-medium text-sm transition-all duration-500 ease-[cubic-bezier(0.32,0.72,0,1)] hover:scale-[1.02] active:scale-[0.98]">
              Get started
              <span className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-black/10 transition-transform duration-500 ease-[cubic-bezier(0.32,0.72,0,1)] group-hover:translate-x-0.5 group-hover:-translate-y-0.5">
                <ArrowRightIcon />
              </span>
            </a>
            <a href="https://github.com/veduket/localtool" target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-2 rounded-full px-6 py-3 border border-white/[0.08] text-sm text-white/60 hover:text-white/80 hover:border-white/[0.15] transition-all duration-500 ease-[cubic-bezier(0.32,0.72,0,1)]">
               <GithubIcon />
               View on GitHub
             </a>
          </div>

          <div className="flex flex-wrap gap-3 mt-8">
            <img src="https://img.shields.io/github/v/release/veduket/localtool?style=flat&label=Release&color=6366f1" alt="GitHub release" />
            <img src="https://img.shields.io/crates/v/localtool?style=flat&label=localtool&color=22d3ee" alt="localtool crate" />
            <img src="https://img.shields.io/crates/v/local-dns?style=flat&label=local-dns&color=34d399" alt="local-dns crate" />
            <img src="https://img.shields.io/crates/v/local-ssl?style=flat&label=local-ssl&color=f59e0b" alt="local-ssl crate" />
          </div>
        </div>

        <div className="hidden lg:block animate-[fade-in_1s_ease-out_0.3s_both]">
          <TerminalMock />
        </div>
      </div>
    </section>
  )
}

function TerminalMock() {
  return (
    <div className="rounded-3xl border border-white/[0.06] bg-[#0a0a14]/80 backdrop-blur-xl overflow-hidden shadow-[0_0_0_1px_rgba(99,102,241,0.06)]">
      <div className="flex items-center gap-1.5 px-5 py-3.5 border-b border-white/[0.04]">
        <div className="w-3 h-3 rounded-full bg-white/10" />
        <div className="w-3 h-3 rounded-full bg-white/10" />
        <div className="w-3 h-3 rounded-full bg-white/10" />
      </div>
      <div className="p-5 font-mono text-[13px] leading-relaxed">
        <p className="text-white/30">$ <span className="text-white/70">localtool dns init</span></p>
        <p className="text-[#22d3ee] mt-1">✓ Initialized</p>
        <p className="text-white/30 mt-3">$ <span className="text-white/70">localtool dns add myapp.test 127.0.0.1</span></p>
        <p className="text-[#22d3ee] mt-1">✓ Added myapp.test → 127.0.0.1</p>
        <p className="text-white/30 mt-3">$ <span className="text-white/70">localtool ssl generate myapp.test</span></p>
        <p className="text-[#34d399] mt-1">✓ Certificate for myapp.test</p>
        <p className="text-white/30 mt-3">$ <span className="text-white/70">curl --cacert /etc/local-ssl/ca-cert.pem https://myapp.test/</span></p>
        <p className="text-white/50 mt-1">&lt;!DOCTYPE html&gt;</p>
        <p className="text-white/50">&lt;html lang=&quot;en&quot;&gt;</p>
        <p className="text-white/50">&lt;head&gt;...&lt;/head&gt;</p>
      </div>
    </div>
  )
}

function VersionBadge() {
  const [version, setVersion] = useState('v0.1.1')

  useEffect(() => {
    fetch('/api/version')
      .then(r => r.json())
      .then(d => setVersion(d.version))
      .catch(() => {})
  }, [])

  return (
    <div className="flex justify-center">
      <a
        href="https://github.com/veduket/localtool/releases"
        target="_blank"
        className="inline-flex items-center gap-2 px-4 py-2 rounded-full border border-white/10 bg-white/[0.03] text-sm text-white/60 hover:text-white/90 hover:border-white/20 transition-colors"
      >
        <span className="w-2 h-2 rounded-full bg-emerald-400/80" />
        <span>Latest release: <span className="font-mono text-white/80">{version}</span></span>
      </a>
    </div>
  )
}

function ToolsOverview() {
  return (
    <section className="px-6 py-32">
      <div className="max-w-6xl mx-auto">
        <div className="text-center mb-20">
          <p className="text-xs font-medium text-white/30 tracking-[0.2em] uppercase mb-4">Tools</p>
          <h2 className="text-3xl sm:text-4xl lg:text-5xl font-bold tracking-tighter">Everything local dev needs</h2>
          <p className="mt-4 text-white/40 max-w-[50ch] mx-auto">
            <code className="text-white/60">localtool dns</code> and <code className="text-white/60">localtool ssl</code> —
            two subcommands that solve the most frustrating parts of local web development.
          </p>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <ToolCard
            name="dns"
            tagline="DNS for your local machine"
            accent="#22d3ee"
            features={[
              'Profile-based DNS with named zones and groups',
              'Wildcard support for subdomain routing',
              'Cross-platform: Linux, macOS, Windows',
              'Automatic dnsmasq configuration',
              'System detection and port conflict warnings',
            ]}
            command="localtool dns add myapp.test 127.0.0.1"
          />
          <ToolCard
            name="ssl"
            tagline="HTTPS certs for local development"
            accent="#34d399"
            features={[
              'CA generation and system trust installation',
              'Wildcard SANs on every certificate',
              'Cross-platform trust: Linux, macOS, Windows',
              'Pure Rust — no OpenSSL system dependency',
              'Pairs seamlessly with localtool dns',
            ]}
            command="localtool ssl generate myapp.test"
          />
        </div>
      </div>
    </section>
  )
}

function ToolCard({ name, tagline, accent, features, command }: {
  name: string
  tagline: string
  accent: string
  features: string[]
  command: string
}) {
  const ref = useRef<HTMLDivElement>(null)
  const [visible, setVisible] = useState(false)

  useEffect(() => {
    const el = ref.current
    if (!el) return
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) { setVisible(true); observer.disconnect() }
    }, { threshold: 0.1 })
    observer.observe(el)
    return () => observer.disconnect()
  }, [])

  return (
    <div
      ref={ref}
      className={`p-1.5 rounded-[2rem] transition-all duration-800 ease-[cubic-bezier(0.32,0.72,0,1)] ${
        visible ? 'opacity-100 translate-y-0 blur-0' : 'opacity-0 translate-y-12 blur-sm'
      }`}
      style={{ background: `linear-gradient(135deg, ${accent}15, transparent 50%)` }}
    >
      <div className="rounded-[calc(2rem-0.375rem)] bg-[#0a0a14] p-8 sm:p-10 shadow-[inset_0_1px_1px_rgba(255,255,255,0.04)]">
        <div className="flex items-center gap-3 mb-4">
          <div className="w-8 h-8 rounded-lg flex items-center justify-center" style={{ background: `${accent}15` }}>
            <div className="w-4 h-4 rounded-sm" style={{ background: accent }} />
          </div>
          <div>
            <h3 className="text-xl font-bold tracking-tight">{name}</h3>
            <p className="text-sm text-white/40">{tagline}</p>
          </div>
        </div>

        <ul className="space-y-2.5 mb-8">
          {features.map((f, i) => (
            <li key={i} className="flex items-start gap-3 text-sm text-white/60">
              <svg className="w-4 h-4 mt-0.5 shrink-0" viewBox="0 0 16 16" fill="none" style={{ color: accent }}>
                <path d="M4 8l3 3 5-6" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
              {f}
            </li>
          ))}
        </ul>

        <div className="rounded-xl bg-white/[0.02] border border-white/[0.04] p-4 font-mono text-sm">
          <span className="text-white/30">$ </span>
          <span className="text-white/70">{command}</span>
        </div>
      </div>
    </div>
  )
}

function StatsBanner() {
  const ref = useRef<HTMLDivElement>(null)
  const [visible, setVisible] = useState(false)

  useEffect(() => {
    const el = ref.current
    if (!el) return
    const observer = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting) { setVisible(true); observer.disconnect() }
    }, { threshold: 0.1 })
    observer.observe(el)
    return () => observer.disconnect()
  }, [])

  return (
    <section
      ref={ref}
      className={`px-6 py-32 transition-all duration-800 ease-[cubic-bezier(0.32,0.72,0,1)] ${
        visible ? 'opacity-100 translate-y-0 blur-0' : 'opacity-0 translate-y-12 blur-sm'
      }`}
    >
      <div className="max-w-6xl mx-auto rounded-[2rem] border border-white/[0.04] bg-gradient-to-b from-white/[0.02] to-transparent p-8 sm:p-16 text-center">
        <p className="text-xs font-medium text-white/30 tracking-[0.2em] uppercase mb-4">Telemetry</p>
        <h2 className="text-3xl sm:text-4xl font-bold tracking-tighter mb-4">Built by the community</h2>
        <p className="text-white/40 max-w-[50ch] mx-auto mb-8">
          Anonymous usage data helps us improve. Check the live leaderboard to see who has been using the
          tools the longest.
        </p>
        <a href="/stats" className="group relative inline-flex items-center gap-2 rounded-full px-6 py-3 bg-white/5 border border-white/[0.08] text-sm font-medium text-white/70 transition-all duration-500 ease-[cubic-bezier(0.32,0.72,0,1)] hover:bg-white/10 hover:text-white active:scale-[0.98]">
          View leaderboard
          <span className="inline-flex items-center justify-center w-5 h-5 rounded-full bg-white/5 transition-transform duration-500 ease-[cubic-bezier(0.32,0.72,0,1)] group-hover:translate-x-0.5">
            <ArrowRightIcon />
          </span>
        </a>
      </div>
    </section>
  )
}

function PricingSection() {
  return (
    <section className="px-6 py-32">
      <div className="max-w-6xl mx-auto text-center">
        <p className="text-xs font-medium text-white/30 tracking-[0.2em] uppercase mb-4">Pricing</p>
        <h2 className="text-3xl sm:text-4xl font-bold tracking-tighter mb-4">Free. Always.</h2>
        <p className="text-white/40 max-w-[50ch] mx-auto mb-8">
          Both tools are open-source under the MIT license. No paid tiers, no feature gates,
          no cloud subscriptions. Forever.
        </p>
        <a href="https://github.com/veduket/localtool" target="_blank" rel="noopener noreferrer" className="group relative inline-flex items-center gap-2 rounded-full px-6 py-3 bg-white text-[#050508] font-medium text-sm transition-all duration-500 ease-[cubic-bezier(0.32,0.72,0,1)] hover:scale-[1.02] active:scale-[0.98]">
          <GithubIcon />
          Star on GitHub
          <span className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-black/10 transition-transform duration-500 ease-[cubic-bezier(0.32,0.72,0,1)] group-hover:translate-x-0.5 group-hover:-translate-y-0.5">
            <ArrowRightIcon />
          </span>
        </a>
      </div>
    </section>
  )
}

type RepoData = { name: string; stars: number; forks: number; url: string }

function GitHubStatsSection() {
  const [repo, setRepo] = useState<RepoData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(false)

  useEffect(() => {
    fetch('/api/github-stats')
      .then(r => r.json())
      .then(data => {
        if (data.error) { setError(true); setLoading(false); return }
        setRepo(data)
        setLoading(false)
      })
      .catch(() => { setError(true); setLoading(false) })
  }, [])

  if (loading) return null
  if (error || !repo) return null

  return (
    <section className="px-6 py-32">
      <div className="max-w-6xl mx-auto text-center">
        <p className="text-xs font-medium text-white/30 tracking-[0.2em] uppercase mb-4">GitHub</p>
        <h2 className="text-3xl sm:text-4xl font-bold tracking-tighter mb-4">Community stats</h2>
        <p className="text-white/60 max-w-[50ch] mx-auto mb-12">
          Stars and forks for the <strong className="text-white/80">localtool</strong> repository on GitHub.
        </p>

        <div className="max-w-md mx-auto">
          <a
            href={repo.url}
            target="_blank"
            rel="noopener noreferrer"
            className="group flex items-center justify-between rounded-2xl border border-white/[0.04] bg-white/[0.02] p-6 hover:bg-white/[0.04] transition-all"
          >
            <p className="font-mono text-sm text-white/80">localtool</p>
            <div className="flex items-center gap-6">
              <div className="flex items-center gap-2">
                <StarIcon />
                <span className="text-lg font-bold tracking-tight text-white">
                  {repo.stars.toLocaleString()}
                </span>
                <span className="text-xs text-white/50">stars</span>
              </div>
              <div className="flex items-center gap-2">
                <ForkIcon />
                <span className="text-lg font-bold tracking-tight text-white">
                  {repo.forks.toLocaleString()}
                </span>
                <span className="text-xs text-white/50">forks</span>
              </div>
            </div>
          </a>
        </div>
      </div>
    </section>
  )
}

function ArrowRightIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 7h8" /><path d="M8 3l4 4-4 4" />
    </svg>
  )
}

function GithubIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
      <path fillRule="evenodd" clipRule="evenodd" d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z" />
    </svg>
  )
}

function StarIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="text-white/40">
      <polygon points="8,1 10.5,5.5 15.5,6.5 12,10 13,15.5 8,12.5 3,15.5 4,10 0.5,6.5 5.5,5.5" fill="none" />
    </svg>
  )
}

function ForkIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className="text-white/40">
      <circle cx="4" cy="4" r="2" /><circle cx="12" cy="4" r="2" /><circle cx="8" cy="12" r="2" />
      <line x1="4" y1="6" x2="4" y2="10" /><line x1="12" y1="6" x2="12" y2="10" />
      <line x1="4" y1="10" x2="8" y2="12" /><line x1="12" y1="10" x2="8" y2="12" />
    </svg>
  )
}
