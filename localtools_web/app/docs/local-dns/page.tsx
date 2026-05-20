export default function LocalDnsDocsPage() {
  return (
    <div className="px-6 pt-32 pb-24">
      <div className="max-w-3xl mx-auto">
        <div className="flex items-center gap-3 mb-6">
          <a href="/docs" className="text-sm text-white/30 hover:text-white/50 transition-colors">Docs</a>
          <span className="text-white/20">/</span>
          <span className="text-sm text-white/60">local-dns</span>
        </div>

        <h1 className="text-4xl sm:text-5xl font-bold tracking-tighter mb-4">
          <span className="text-[#22d3ee]">local-dns</span>
        </h1>
        <p className="text-lg text-white/40 mb-12 max-w-[50ch]">
          DNS management for local development. Profile-based, wildcard-ready, cross-platform.
        </p>

        <Section title="Installation">
          <CodeBlock>`# macOS (Homebrew)
brew install dnsmasq
cargo install local-dns

# Linux
sudo apt install dnsmasq
cargo install local-dns

# Windows (winget)
winget install dnsmasq
cargo install local-dns`</CodeBlock>
        </Section>

        <Section title="Quick start">
          <Steps steps={[
            { label: 'Initialize', code: 'local-dns init' },
            { label: 'Add a domain', code: 'local-dns add myapp.test 127.0.0.1' },
            { label: 'Verify', code: 'ping myapp.test' },
            { label: 'Add with zone', code: 'local-dns add api.test 127.0.0.1 --zone projects' },
            { label: 'List entries', code: 'local-dns list' },
          ]} />
        </Section>

        <Section title="Commands">
          <div className="space-y-3">
            <CmdRow cmd="local-dns add <domain> <ip>" desc="Add a DNS entry" opts="--zone, --group, --comment" />
            <CmdRow cmd="local-dns remove <domain>" desc="Remove a DNS entry" />
            <CmdRow cmd="local-dns move <domain>" desc="Move entry to another zone/group" opts="--zone, --group" />
            <CmdRow cmd="local-dns copy <domain>" desc="Copy entry to another zone/group" opts="--zone, --group" />
            <CmdRow cmd="local-dns edit <domain>" desc="Edit IP or comment" opts="--ip, --comment" />
            <CmdRow cmd="local-dns list" desc="List all entries in active profile" />
            <CmdRow cmd="local-dns init" desc="Initialize configuration and services" />
            <CmdRow cmd="local-dns reset" desc="Delete database and re-initialize from scratch" />
            <CmdRow cmd="local-dns status" desc="Show system status" />
            <CmdRow cmd="local-dns apply" desc="Apply DNS configuration" />
            <CmdRow cmd="local-dns detect" desc="Detect system DNS configuration" />
            <CmdRow cmd="local-dns check <domain>" desc="Check entry status (created date, loaded state)" />
            <CmdRow cmd="local-dns logs" desc="View dnsmasq logs" opts="--follow, --errors, --lines" />
            <CmdRow cmd="local-dns telemetry status" desc="View telemetry settings" />
          </div>
        </Section>

        <Section title="Profiles">
          <p className="text-sm text-white/50 leading-relaxed mb-4">
            Profiles let you switch between different DNS configurations. Perfect for separating
            work projects from personal projects.
          </p>
          <CodeBlock>`# Create a profile
local-dns profile create work

# Switch to a profile
local-dns profile switch work

# List profiles
local-dns profile list

# Show active profile
local-dns profile show`</CodeBlock>
        </Section>

        <Section title="Zones and Groups">
          <p className="text-sm text-white/50 leading-relaxed mb-4">
            Zones group related DNS entries. Groups within zones add another level of organization.
          </p>
          <CodeBlock>`# Create a zone
local-dns zone create services

# Create a group in a zone
local-dns group create api --zone services

# Add entry to group
local-dns add myapp.test 127.0.0.1 --zone services --group api`</CodeBlock>
        </Section>

        <Section title="GitHub">
          <p className="text-sm text-white/50 leading-relaxed">
            Source code, issues, and contributions:
          </p>
          <a
            href="https://github.com/veduket/local-dns"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 mt-2 text-sm text-white/60 hover:text-white/80 transition-colors"
          >
            <span className="font-mono">github.com/veduket/local-dns</span>
            <span className="text-xs text-white/20">↗</span>
          </a>
        </Section>
      </div>
    </div>
  )
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="mb-12">
      <h2 className="text-xl font-semibold tracking-tight mb-5">{title}</h2>
      {children}
    </div>
  )
}

function CodeBlock({ children }: { children: string }) {
  return (
    <pre className="rounded-2xl border border-white/[0.04] bg-white/[0.02] p-5 overflow-x-auto">
      <code className="font-mono text-sm text-white/60 leading-relaxed">{children}</code>
    </pre>
  )
}

function Steps({ steps }: { steps: { label: string; code: string }[] }) {
  return (
    <div className="space-y-3">
      {steps.map((s, i) => (
        <div key={i} className="flex items-start gap-4">
          <div className="w-6 h-6 rounded-full bg-[#22d3ee]/10 border border-[#22d3ee]/20 flex items-center justify-center shrink-0 mt-0.5">
            <span className="text-xs font-mono text-[#22d3ee]">{i + 1}</span>
          </div>
          <div className="flex-1">
            <p className="text-sm font-medium text-white/70 mb-1">{s.label}</p>
            <pre className="rounded-xl border border-white/[0.04] bg-white/[0.02] px-4 py-2.5 overflow-x-auto">
              <code className="font-mono text-sm text-white/50"><span className="text-white/20">$ </span>{s.code}</code>
            </pre>
          </div>
        </div>
      ))}
    </div>
  )
}

function CmdRow({ cmd, desc, opts }: { cmd: string; desc: string; opts?: string }) {
  return (
    <div className="flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-4 py-2">
      <code className="font-mono text-sm text-white/70 sm:w-[32rem] shrink-0">{cmd}</code>
      <span className="text-sm text-white/40">{desc}</span>
      {opts && <span className="text-xs text-white/20 font-mono">{opts}</span>}
    </div>
  )
}
