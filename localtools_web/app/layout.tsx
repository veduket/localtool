import type { Metadata } from 'next'
import './globals.css'

export const metadata: Metadata = {
  title: 'localtools — Local development infrastructure',
  description: 'local-dns: DNS management for local dev. local-ssl: HTTPS certificates for local development. Open-source developer tools.',
  metadataBase: new URL('https://localtool.vercel.app'),
  openGraph: {
    title: 'localtools — Local development infrastructure',
    description: 'local-dns: DNS management. local-ssl: HTTPS certificates. Open-source developer tools.',
    url: 'https://localtool.vercel.app',
    siteName: 'localtools',
  },
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className="antialiased">
        <div className="grain-overlay" />
        <Header />
        <main>{children}</main>
        <Footer />
      </body>
    </html>
  )
}

function Header() {
  return (
    <header className="fixed top-0 left-0 right-0 z-40 flex justify-center pt-4">
      <nav className="flex items-center gap-6 px-6 py-3 bg-[#0a0a14]/90 backdrop-blur-2xl border border-white/[0.06] rounded-full shadow-[0_0_0_1px_rgba(99,102,241,0.08)]">
        <a href="/" className="font-semibold tracking-tight text-sm text-white/90 hover:text-white transition-colors">localtools</a>
        <div className="w-px h-4 bg-white/[0.08]" />
        <a href="/docs" className="text-sm text-white/50 hover:text-white/80 transition-colors">Docs</a>
        <a href="/stats" className="text-sm text-white/50 hover:text-white/80 transition-colors">Stats</a>
        <a href="https://github.com/veduket/localtools_web" target="_blank" rel="noopener noreferrer" className="text-sm text-white/50 hover:text-white/80 transition-colors">GitHub</a>
      </nav>
    </header>
  )
}

function Footer() {
  return (
    <footer className="border-t border-white/[0.04] mt-32">
      <div className="max-w-6xl mx-auto px-6 py-12 flex flex-col md:flex-row items-center justify-between gap-4">
        <p className="text-sm text-white/30">localtools — MIT License — Yared Getachew</p>
        <div className="flex items-center gap-6">
          <a href="https://github.com/veduket/localtools_web" target="_blank" rel="noopener noreferrer" className="text-sm text-white/30 hover:text-white/60 transition-colors">GitHub</a>
          <a href="/docs" className="text-sm text-white/30 hover:text-white/60 transition-colors">Documentation</a>
          <a href="/stats" className="text-sm text-white/30 hover:text-white/60 transition-colors">Stats</a>
        </div>
      </div>
    </footer>
  )
}
