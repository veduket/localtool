export default function LocaltoolDocsPage() {
  return (
    <div className="px-6 pt-32 pb-24">
      <div className="max-w-3xl mx-auto">
        <div className="flex items-center gap-3 mb-6">
          <a href="/docs" className="text-sm text-white/30 hover:text-white/50 transition-colors">Docs</a>
          <span className="text-white/20">/</span>
          <span className="text-sm text-white/60">localtool</span>
        </div>

        <h1 className="text-4xl sm:text-5xl font-bold tracking-tighter mb-4">
          <span className="text-[#a78bfa]">localtool</span>
        </h1>
        <p className="text-lg text-white/40 mb-12 max-w-[50ch]">
          Unified CLI for local development — DNS management and HTTPS certificates in one tool.
        </p>

        <Section title="Installation">
          <CodeBlock>{`cargo install localtool

# Or build from source:
# git clone https://github.com/veduket/localtool
# cd localtool && cargo build --release && sudo cp target/release/localtool /usr/local/bin/`}</CodeBlock>
        </Section>

        <Section title="Quick start">
          <Steps steps={[
            { label: 'Initialize DNS', code: 'localtool dns init' },
            { label: 'Add a domain', code: 'localtool dns add myapp.test 127.0.0.1' },
            { label: 'Apply DNS config', code: 'localtool dns apply' },
            { label: 'Verify DNS', code: 'ping myapp.test' },
            { label: 'Initialize SSL CA', code: 'localtool ssl init' },
            { label: 'Generate HTTPS cert', code: 'localtool ssl generate myapp.test' },
          ]} />
        </Section>

        <Section title="DNS Commands">
          <p className="text-sm text-white/50 leading-relaxed mb-4">
            All <code className="text-white/70">local-dns</code> commands are available under <code className="text-white/70">localtool dns</code>.
          </p>
          <div className="space-y-3">
            <CmdRow cmd="localtool dns add <domain> <ip>" desc="Add a DNS entry" opts="--zone, --group, --comment" />
            <CmdRow cmd="localtool dns remove <domain>" desc="Remove a DNS entry" />
            <CmdRow cmd="localtool dns move <domain>" desc="Move entry to another zone/group" opts="--zone, --group" />
            <CmdRow cmd="localtool dns copy <domain>" desc="Copy entry to another zone/group" opts="--zone, --group" />
            <CmdRow cmd="localtool dns edit <domain>" desc="Edit IP or comment" opts="--ip, --comment" />
            <CmdRow cmd="localtool dns list" desc="List all entries" />
            <CmdRow cmd="localtool dns init" desc="Initialize DNS configuration" />
            <CmdRow cmd="localtool dns reset" desc="Delete database and re-initialize" />
            <CmdRow cmd="localtool dns status" desc="Show DNS system status" />
            <CmdRow cmd="localtool dns apply" desc="Apply DNS configuration" />
            <CmdRow cmd="localtool dns detect" desc="Detect system DNS setup" />
            <CmdRow cmd="localtool dns logs" desc="View dnsmasq logs" opts="--follow, --errors, --lines" />
            <CmdRow cmd="localtool dns profile" desc="Manage profiles" opts="create, switch, list, delete" />
            <CmdRow cmd="localtool dns zone" desc="Manage zones" opts="create, list, delete, show" />
            <CmdRow cmd="localtool dns group" desc="Manage groups" opts="create, list, delete" />
          </div>
        </Section>

        <Section title="SSL Commands">
          <p className="text-sm text-white/50 leading-relaxed mb-4">
            All <code className="text-white/70">local-ssl</code> commands are available under <code className="text-white/70">localtool ssl</code>.
          </p>
          <div className="space-y-3">
            <CmdRow cmd="localtool ssl init" desc="Initialize CA and install system trust" />
            <CmdRow cmd="localtool ssl generate <domains>" desc="Generate HTTPS certificates" />
            <CmdRow cmd="localtool ssl list" desc="List all generated certificates" />
            <CmdRow cmd="localtool ssl show <domain>" desc="Show certificate details" />
            <CmdRow cmd="localtool ssl trust" desc="Reinstall CA system trust" />
            <CmdRow cmd="localtool ssl status" desc="Show CA and certificate status" />
            <CmdRow cmd="localtool ssl check <domain>" desc="Check certificate validity" />
          </div>
        </Section>

        <Section title="Backwards Compatibility">
          <p className="text-sm text-white/50 leading-relaxed mb-4">
            The standalone <code className="text-white/70">local-dns</code> and <code className="text-white/70">local-ssl</code> binaries continue to work
            as before. They are thin wrappers that delegate to the same underlying libraries.
          </p>
          <CodeBlock>{`# These still work:
local-dns add myapp.test 127.0.0.1
local-ssl generate myapp.test

# Equivalent unified commands:
localtool dns add myapp.test 127.0.0.1
localtool ssl generate myapp.test`}</CodeBlock>
        </Section>

        <Section title="GitHub">
          <p className="text-sm text-white/50 leading-relaxed">
            Source code, issues, and contributions:
          </p>
          <a
            href="https://github.com/veduket/localtool"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 mt-2 text-sm text-white/60 hover:text-white/80 transition-colors"
          >
            <span className="font-mono">github.com/veduket/localtool</span>
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
      <code className="font-mono text-sm text-white/60 leading-relaxed whitespace-pre">{children}</code>
    </pre>
  )
}

function Steps({ steps }: { steps: { label: string; code: string }[] }) {
  return (
    <div className="space-y-3">
      {steps.map((s, i) => (
        <div key={i} className="flex items-start gap-4">
          <div className="w-6 h-6 rounded-full bg-[#a78bfa]/10 border border-[#a78bfa]/20 flex items-center justify-center shrink-0 mt-0.5">
            <span className="text-xs font-mono text-[#a78bfa]">{i + 1}</span>
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
