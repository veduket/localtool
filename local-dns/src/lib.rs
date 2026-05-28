use clap::{Parser, Subcommand};
use colored::Colorize;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub mod telemetry;

// ─── Platform-aware paths ──────────────────────────────────

fn data_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".into()))
            .join("local-dns")
    } else {
        PathBuf::from("/etc/local-dns")
    }
}

fn run_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        data_dir().join("run")
    } else {
        PathBuf::from("/run/local-dns")
    }
}

fn log_path() -> PathBuf {
    if cfg!(target_os = "windows") {
        data_dir().join("local-dns.log")
    } else {
        PathBuf::from("/var/log/local-dns.log")
    }
}

fn ensure_run_dir() -> Result<PathBuf, String> {
    let r = run_dir();
    if r.exists() && !r.is_dir() {
        eprintln!(
            "{} Removing stale file at {}",
            "⚠".yellow(),
            r.display().to_string().cyan()
        );
        fs::remove_file(&r).map_err(|e| format!("Cannot remove {}: {e}", r.display()))?;
    }
    fs::create_dir_all(&r).map_err(|e| format!("Cannot create {}: {e}", r.display()))?;
    Ok(r)
}

fn collect_tool_stats() -> Vec<(&'static str, String)> {
    let mut stats: Vec<(&'static str, String)> = Vec::new();
    if let Ok(conn) = open_db_readonly() {
        if let Ok(n) = conn.query_row("SELECT COUNT(*) FROM zones", [], |r| r.get::<_, i64>(0)) {
            stats.push(("zones", n.to_string()));
        }
        if let Ok(n) = conn.query_row("SELECT COUNT(*) FROM groups", [], |r| r.get::<_, i64>(0)) {
            stats.push(("groups", n.to_string()));
        }
        if let Ok(n) = conn.query_row("SELECT COUNT(*) FROM entries", [], |r| r.get::<_, i64>(0)) {
            stats.push(("entries", n.to_string()));
        }
    }
    stats
}

fn pid_path() -> PathBuf {
    run_dir().join("dnsmasq.pid")
}

fn dnsmasq_binary() -> &'static str {
    if cfg!(target_os = "macos") {
        "/opt/homebrew/sbin/dnsmasq"
    } else if cfg!(target_os = "windows") {
        "dnsmasq.exe" // WSL or native port
    } else {
        "/usr/sbin/dnsmasq"
    }
}

fn default_upstream() -> (String, u16) {
    if cfg!(target_os = "windows") {
        ("8.8.8.8".to_string(), 53)
    } else {
        ("127.0.0.1".to_string(), 53)
    }
}

// ─── Detecting available system tools ──────────────────────

fn has_tool(name: &str) -> bool {
    if cfg!(target_os = "windows") {
        Command::new("where")
            .arg(name)
            .output()
            .ok()
            .is_some_and(|o| o.status.success())
    } else {
        Command::new("which")
            .arg(name)
            .output()
            .ok()
            .is_some_and(|o| o.status.success())
    }
}

fn supports_systemd() -> bool {
    has_tool("systemctl")
}

fn supports_launchctl() -> bool {
    cfg!(target_os = "macos") && has_tool("launchctl")
}

fn supports_dnsmasq() -> bool {
    has_tool("dnsmasq") || Path::new(dnsmasq_binary()).exists()
}

// ─── Platform-specific network/dns detection ───────────────

fn detect_dns_tools() -> Vec<(u16, String)> {
    let mut results = Vec::new();

    if cfg!(target_os = "windows") {
        // Windows: use netstat
        if let Ok(out) = Command::new("netstat").args(["-an"]).output() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                for &port in &[53u16, 5353, 5354, 5053] {
                    if line.contains(&format!(":{port}")) && line.contains("LISTENING") {
                        results.push((port, "process".into()));
                    }
                }
            }
        }
    } else {
        // Unix: use ss, fallback to lsof, fallback to netstat
        let out = Command::new("ss")
            .args(["-tlnp"])
            .output()
            .or_else(|_| Command::new("lsof").args(["-i", "-P", "-n"]).output())
            .or_else(|_| Command::new("netstat").args(["-tlnp"]).output());

        if let Ok(output) = out {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                for &port in &[53u16, 5053, 5353, 5354, 5300, 853, 443] {
                    if !line.contains(&format!(":{port}")) {
                        continue;
                    }
                    let proc = if let Some(pidx) = line.find("users:") {
                        line[pidx..]
                            .split('"')
                            .nth(1)
                            .unwrap_or("unknown")
                            .to_string()
                    } else if let Some(pidx) = line.find("->") {
                        line[pidx..]
                            .split_whitespace()
                            .next()
                            .unwrap_or("unknown")
                            .to_string()
                    } else {
                        "unknown".to_string()
                    };
                    results.push((port, proc));
                }
            }
        }
    }
    results
}

fn detect_services() -> Vec<String> {
    let mut services = Vec::new();
    let checks = if cfg!(target_os = "macos") {
        vec!["dnsmasq", "dnscrypt-proxy", "unbound", "avahi-daemon"]
    } else {
        vec![
            "dnsmasq",
            "dnsdist",
            "dnscrypt-proxy",
            "systemd-resolved",
            "named",
            "unbound",
            "avahi-daemon",
        ]
    };

    for name in &checks {
        let active = if supports_systemd() {
            Command::new("systemctl")
                .args(["is-active", name, "--quiet"])
                .status()
                .ok()
                .is_some_and(|s| s.success())
        } else if supports_launchctl() {
            Command::new("launchctl")
                .args(["print", &format!("system/{name}")])
                .status()
                .ok()
                .is_some_and(|s| s.success())
        } else {
            false
        };
        if active {
            services.push(name.to_string());
        }
    }

    // Additional pgrep check on Unix
    #[cfg(unix)]
    if let Ok(out) = Command::new("pgrep").args(["-a", "-f", "dns"]).output() {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            for name in &["pdnsd", "knot", "maradns", "stubby", "coredns"] {
                if line.contains(name) && !services.iter().any(|s| s.contains(name)) {
                    services.push(name.to_string());
                }
            }
        }
    }

    services
}

// ═══════════════════════════════════════════════════════════
//  CONSTANTS (computed at runtime for cross-platform paths)
// ═══════════════════════════════════════════════════════════

const DNSMASQ_SERVICE: &str = "local-dnsmasq";

// ═══════════════════════════════════════════════════════════
//  CLI
// ═══════════════════════════════════════════════════════════

#[derive(Parser)]
#[command(name = "local-dns", about = "Local DNS management", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Add {
        domain: String,
        ip: String,
        #[arg(short, long)]
        zone: Option<String>,
        #[arg(short = 'g', long)]
        group: Option<String>,
        #[arg(short, long)]
        comment: Option<String>,
    },
    Remove {
        domain: String,
    },
    Move {
        domain: String,
        #[arg(short, long)]
        zone: Option<String>,
        #[arg(short = 'g', long)]
        group: Option<String>,
    },
    Copy {
        domain: String,
        #[arg(short, long)]
        zone: Option<String>,
        #[arg(short = 'g', long)]
        group: Option<String>,
    },
    Edit {
        domain: String,
        #[arg(short, long)]
        ip: Option<String>,
        #[arg(short, long)]
        comment: Option<String>,
    },
    List,
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },
    Zone {
        #[command(subcommand)]
        action: ZoneAction,
    },
    Group {
        #[command(subcommand)]
        action: GroupAction,
    },
    Init,
    Reset,
    Status,
    Apply,
    Logs {
        #[arg(short, long)]
        follow: bool,
        #[arg(short, long)]
        errors: bool,
        #[arg(short, long, default_value = "50")]
        lines: usize,
    },
    Detect,
    /// Diagnose and fix DNS routing issues
    Doctor {
        /// Attempt to automatically fix detected issues
        #[arg(short, long)]
        fix: bool,
    },
    Check {
        domain: String,
    },
    Telemetry {
        #[command(subcommand)]
        action: TelemetryAction,
    },
}

#[derive(Subcommand)]
pub enum TelemetryAction {
    Enable,
    Disable,
    Status,
}

#[derive(Subcommand)]
pub enum ProfileAction {
    Show,
    Switch { name: String },
    Create { name: String },
    List,
    Delete { name: String },
}
#[derive(Subcommand)]
pub enum ZoneAction {
    Create {
        name: String,
        #[arg(short, long)]
        display: Option<String>,
    },
    List,
    Delete {
        name: String,
    },
    Show {
        name: String,
    },
}
#[derive(Subcommand)]
pub enum GroupAction {
    Create {
        name: String,
        zone: String,
        #[arg(short, long)]
        display: Option<String>,
    },
    List {
        #[arg(short, long)]
        zone: Option<String>,
    },
    Delete {
        name: String,
        zone: String,
    },
}

struct Entry {
    domain: String,
    ip: String,
    comment: Option<String>,
    zone_name: String,
    group_name: String,
    created_at: String,
}
struct Profile {
    id: i64,
    name: String,
    #[allow(dead_code)]
    is_active: bool,
}

#[derive(Debug)]
struct SystemState {
    port_listeners: HashMap<u16, String>,
    services: Vec<String>,
    #[allow(dead_code)]
    resolv_conf: String,
    upstream_dns: String,
    upstream_port: u16,
    dnsdist_snippet: Option<String>,
    resolved_conf: Option<String>,
    warnings: Vec<String>,
}

// ═══════════════════════════════════════════════════════════
//  MAIN
// ═══════════════════════════════════════════════════════════

pub fn run(args: &[String]) -> Result<String, String> {
    let tel = telemetry::Telemetry::load(&data_dir());
    let cli = Cli::parse_from(args);
    let is_telemetry_cmd = matches!(&cli.command, Commands::Telemetry { .. });
    let result = execute(cli, &tel);
    if !is_telemetry_cmd && tel.maybe_heartbeat() {
        tel.show_heartbeat_notice();
    }
    result
}

fn execute(cli: Cli, tel: &telemetry::Telemetry) -> Result<String, String> {
    let cmd_name = match &cli.command {
        Commands::Telemetry { .. } => "telemetry",
        Commands::Add { .. } => "add",
        Commands::Remove { .. } => "remove",
        Commands::Move { .. } => "move",
        Commands::Copy { .. } => "copy",
        Commands::Edit { .. } => "edit",
        Commands::List => "list",
        Commands::Profile { .. } => "profile",
        Commands::Zone { .. } => "zone",
        Commands::Group { .. } => "group",
        Commands::Init => "init",
        Commands::Reset => "reset",
        Commands::Status => "status",
        Commands::Apply => "apply",
        Commands::Logs { .. } => "logs",
        Commands::Detect => "detect",
        Commands::Doctor { .. } => "doctor",
        Commands::Check { .. } => "check",
    };
    let stats = if matches!(cmd_name, "init" | "reset" | "status" | "detect") {
        vec![]
    } else {
        collect_tool_stats()
    };
    tel.send_command_event(
        cmd_name,
        &stats
            .iter()
            .map(|(k, v)| (*k, v.as_str()))
            .collect::<Vec<_>>(),
    );

    match cli.command {
        Commands::Add {
            domain,
            ip,
            zone,
            group,
            comment,
        } => add_entry(
            &domain,
            &ip,
            zone.as_deref(),
            group.as_deref(),
            comment.as_deref(),
        ),
        Commands::Remove { domain } => remove_entry(&domain),
        Commands::Move {
            domain,
            zone,
            group,
        } => move_entry(&domain, zone.as_deref(), group.as_deref()),
        Commands::Copy {
            domain,
            zone,
            group,
        } => copy_entry(&domain, zone.as_deref(), group.as_deref()),
        Commands::Edit {
            domain,
            ip,
            comment,
        } => edit_entry(&domain, ip.as_deref(), comment.as_deref()),
        Commands::List => list_entries(),
        Commands::Profile { action } => handle_profile(action),
        Commands::Zone { action } => handle_zone(action),
        Commands::Group { action } => handle_group(action),
        Commands::Init => cmd_init(),
        Commands::Reset => cmd_reset(),
        Commands::Status => cmd_status(),
        Commands::Apply => cmd_apply(),
        Commands::Logs {
            follow,
            errors,
            lines,
        } => cmd_logs(follow, errors, lines),
        Commands::Detect => cmd_detect(),
        Commands::Doctor { fix } => cmd_doctor(fix),
        Commands::Check { domain } => cmd_check(&domain),
        Commands::Telemetry { action } => handle_telemetry(action, tel),
    }
}

// ═══════════════════════════════════════════════════════════
//  DATABASE
// ═══════════════════════════════════════════════════════════

fn db_path() -> PathBuf {
    data_dir().join("local-dns.db")
}

fn open_db_readonly() -> Result<Connection, String> {
    Connection::open_with_flags(db_path(), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("Cannot open database: {e}"))
}

fn open_db() -> Result<Connection, String> {
    let conn = Connection::open(db_path()).map_err(|e| format!("Cannot open database: {e}"))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| format!("{e}"))?;
    Ok(conn)
}

fn init_db() -> Result<Connection, String> {
    let conn = open_db()?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS profiles (
            id    INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT UNIQUE NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS zones (
            id INTEGER PRIMARY KEY AUTOINCREMENT, profile_id INTEGER NOT NULL,
            name TEXT NOT NULL, display_name TEXT, sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE,
            UNIQUE(profile_id, name)
        );
        CREATE TABLE IF NOT EXISTS groups (
            id INTEGER PRIMARY KEY AUTOINCREMENT, zone_id INTEGER NOT NULL,
            name TEXT NOT NULL, display_name TEXT, sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (zone_id) REFERENCES zones(id) ON DELETE CASCADE,
            UNIQUE(zone_id, name)
        );
        CREATE TABLE IF NOT EXISTS entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT, group_id INTEGER NOT NULL,
            domain TEXT NOT NULL, ip TEXT NOT NULL, comment TEXT, sort_key TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE,
            UNIQUE(group_id, domain)
        );
        CREATE INDEX IF NOT EXISTS idx_zones_profile ON zones(profile_id);
        CREATE INDEX IF NOT EXISTS idx_groups_zone ON groups(zone_id);
        CREATE INDEX IF NOT EXISTS idx_entries_group ON entries(group_id);",
    )
    .map_err(|e| format!("Cannot create schema: {e}"))?;
    Ok(conn)
}

fn active_profile(conn: &Connection) -> Result<Profile, String> {
    conn.query_row(
        "SELECT id, name, is_active FROM profiles WHERE is_active = 1 LIMIT 1",
        [],
        |row| {
            Ok(Profile {
                id: row.get(0)?,
                name: row.get(1)?,
                is_active: row.get::<_, i32>(2)? != 0,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            "Not initialized. Run `local-dns init` first.".into()
        }
        _ => format!("Database error: {e}"),
    })
}

fn ensure_defaults(conn: &Connection) -> Result<(i64, i64), String> {
    let p = active_profile(conn)?;
    conn.execute("INSERT OR IGNORE INTO zones (profile_id, name, display_name) VALUES (?1, 'global', 'Global')", params![p.id]).ok();
    let zid: i64 = conn
        .query_row(
            "SELECT id FROM zones WHERE profile_id = ?1 AND name = 'global'",
            params![p.id],
            |row| row.get(0),
        )
        .unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO groups (zone_id, name, display_name) VALUES (?1, 'main', 'Main')",
        params![zid],
    )
    .ok();
    let gid: i64 = conn
        .query_row(
            "SELECT id FROM groups WHERE zone_id = ?1 AND name = 'main'",
            params![zid],
            |row| row.get(0),
        )
        .unwrap();
    Ok((zid, gid))
}

fn resolve_zone(conn: &Connection, pid: i64, name: &str) -> Result<i64, String> {
    conn.query_row(
        "SELECT id FROM zones WHERE profile_id = ?1 AND name = ?2",
        params![pid, name],
        |row| row.get(0),
    )
    .map_err(|_| format!("Zone '{name}' not found"))
}

fn resolve_group(conn: &Connection, zid: i64, name: &str) -> Result<i64, String> {
    conn.query_row(
        "SELECT id FROM groups WHERE zone_id = ?1 AND name = ?2",
        params![zid, name],
        |row| row.get(0),
    )
    .map_err(|_| format!("Group '{name}' not found"))
}

fn profile_names(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT name FROM profiles ORDER BY name")
        .map_err(|e| format!("{e}"))?;
    let r: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| format!("{e}"))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{e}"))?;
    Ok(r)
}

fn zone_list(conn: &Connection, pid: i64) -> Result<Vec<(String, Option<String>)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT name, display_name FROM zones WHERE profile_id = ?1 ORDER BY sort_order, name",
        )
        .map_err(|e| format!("{e}"))?;
    let r = stmt
        .query_map(params![pid], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|e| format!("{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{e}"))?;
    Ok(r)
}

fn group_list(conn: &Connection, zid: i64) -> Result<Vec<(String, Option<String>)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT name, display_name FROM groups WHERE zone_id = ?1 ORDER BY sort_order, name",
        )
        .map_err(|e| format!("{e}"))?;
    let r = stmt
        .query_map(params![zid], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|e| format!("{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{e}"))?;
    Ok(r)
}

fn all_entries(conn: &Connection, pid: i64) -> Result<Vec<Entry>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT e.domain, e.ip, e.comment, z.name, g.name, e.created_at FROM entries e
         JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id
         WHERE z.profile_id = ?1 ORDER BY z.sort_order, z.name, g.sort_order, g.name, e.sort_key",
        )
        .map_err(|e| format!("{e}"))?;
    let r = stmt
        .query_map(params![pid], |row| {
            Ok(Entry {
                domain: row.get(0)?,
                ip: row.get(1)?,
                comment: row.get(2)?,
                zone_name: row.get(3)?,
                group_name: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| format!("{e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{e}"))?;
    Ok(r)
}

// ═══════════════════════════════════════════════════════════
//  SYSTEM DETECTION
// ═══════════════════════════════════════════════════════════

fn cmd_detect() -> Result<String, String> {
    let state = detect_system()?;
    Ok(format_detect(&state))
}

fn detect_system() -> Result<SystemState, String> {
    let raw_ports = detect_dns_tools();
    let port_listeners: HashMap<u16, String> = raw_ports.into_iter().collect();
    let services = detect_services();
    let resolv_conf = if cfg!(unix) {
        fs::read_to_string("/etc/resolv.conf").unwrap_or_default()
    } else {
        String::new()
    };
    let upstream = detect_upstream_candidates(&services, &port_listeners);

    let mut state = SystemState {
        port_listeners,
        services,
        resolv_conf,
        upstream_dns: upstream.0,
        upstream_port: upstream.1,
        dnsdist_snippet: None,
        resolved_conf: None,
        warnings: Vec::new(),
    };

    if state.port_listeners.contains_key(&5354) {
        state.warnings.push(format!(
            "Port 5354 is in use by '{}'.",
            state.port_listeners.get(&5354).unwrap()
        ));
    }
    if state.services.iter().any(|s| s == "dnsdist") {
        state.dnsdist_snippet = Some(dnsdist_routing_snippet());
    }
    if state.services.iter().any(|s| s == "systemd-resolved") {
        state.resolved_conf = Some(resolved_routing_snippet());
    }

    Ok(state)
}

fn detect_upstream_candidates(services: &[String], ports: &HashMap<u16, String>) -> (String, u16) {
    if services.iter().any(|s| s.contains("dnscrypt")) && ports.contains_key(&5053) {
        return ("127.0.0.1".into(), 5053);
    }
    if services.iter().any(|s| s.contains("dnsdist")) && ports.contains_key(&53) {
        return ("127.0.0.1".into(), 53);
    }
    default_upstream()
}

fn dnsdist_routing_snippet() -> String {
    r#"-- === local-dns: forward custom zones to local dnsmasq ===
local localZones = newSuffixMatchNode()
localZones:add("test"); localZones:add("dev"); localZones:add("local"); localZones:add("localhost")
addAction(SuffixMatchNodeRule(localZones), PoolAction("local"))
newServer({address="127.0.0.1:5354", pool="local"})
-- ===================================================="#
        .into()
}

fn resolved_routing_snippet() -> String {
    "# Make queries for custom TLDs reach local dnsmasq:\n# resolvectl dns lo 127.0.0.1\n# resolvectl domain lo '~test' '~dev' '~local' '~localhost'".into()
}

fn configure_resolved_routing() -> Result<(), String> {
    let conf_dir = Path::new("/etc/systemd/resolved.conf.d");
    fs::create_dir_all(conf_dir).map_err(|e| format!("Cannot create {conf_dir:?}: {e}"))?;
    let conf = r#"# Managed by local-dns — local development DNS routing
[Resolve]
DNS=127.0.0.1:5354
Domains=~test ~dev ~local ~localhost ~invalid
"#;
    let path = conf_dir.join("local-dns.conf");
    fs::write(&path, conf).map_err(|e| format!("Cannot write {path:?}: {e}"))?;
    println!(
        "  {} Created {}",
        "✓".green(),
        path.display().to_string().cyan()
    );

    let restart = Command::new("systemctl")
        .args(["try-reload-or-restart", "systemd-resolved"])
        .status()
        .map_err(|e| format!("Cannot restart systemd-resolved: {e}"))?;
    if restart.success() {
        println!("  {} Restarted systemd-resolved", "✓".green());
    } else {
        eprintln!("  {} Failed to restart systemd-resolved", "⚠".yellow());
    }

    // Also apply immediately via resolvectl so it works without reboot
    let dns_set = Command::new("resolvectl")
        .args(["dns", "lo", "127.0.0.1:5354"])
        .status();
    let domain_set = Command::new("resolvectl")
        .args([
            "domain",
            "lo",
            "~test",
            "~dev",
            "~local",
            "~localhost",
            "~invalid",
        ])
        .status();
    if dns_set.ok().is_some_and(|s| s.success()) && domain_set.ok().is_some_and(|s| s.success())
    {
        println!("  {} Applied resolvectl routing", "✓".green());
    }

    Ok(())
}

fn format_detect(state: &SystemState) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} {}\n\n",
        "Platform:".cyan().bold(),
        std::env::consts::OS
    ));
    out.push_str(&format!("{}\n", "Port Usage:".bold()));
    if state.port_listeners.is_empty() {
        out.push_str(&format!("  {}\n", "(none detected)".yellow()));
    } else {
        let mut s: Vec<_> = state.port_listeners.iter().collect();
        s.sort_by_key(|(p, _)| **p);
        for (p, n) in &s {
            let marker = if **p == 5354 {
                " ← local-dns".green().to_string()
            } else {
                String::new()
            };
            out.push_str(&format!(
                "  Port {:<5} {}{}\n",
                p.to_string().yellow(),
                n,
                marker
            ));
        }
    }
    out.push_str(&format!("\n{}\n", "DNS Services:".bold()));
    if state.services.is_empty() {
        out.push_str(&format!("  {}\n", "(none)".yellow()));
    } else {
        for s in &state.services {
            out.push_str(&format!("  {}\n", format!("✓ {s}").green()));
        }
    }
    out.push_str(&format!(
        "\n{} {}:{}\n",
        "Upstream:".bold(),
        state.upstream_dns.cyan(),
        state.upstream_port.to_string().cyan()
    ));
    let dnsmasq_status = if supports_dnsmasq() {
        "✓ available".green().to_string()
    } else {
        "✗ not found".red().to_string()
    };
    out.push_str(&format!("dnsmasq: {dnsmasq_status}\n"));
    if !state.warnings.is_empty() {
        out.push_str(&format!("\n{}\n", "Warnings:".yellow().bold()));
        for w in &state.warnings {
            out.push_str(&format!("  {}\n", format!("⚠ {w}").yellow()));
        }
    }
    out
}

// ═══════════════════════════════════════════════════════════
//  DOCTOR
// ═══════════════════════════════════════════════════════════

fn cmd_doctor(fix: bool) -> Result<String, String> {
    let mut out = String::new();
    let state = detect_system()?;
    out.push_str(&format!("{} {}\n", "Platform:".cyan().bold(), std::env::consts::OS));
    out.push_str(&format_detect(&state));

    let dnsmasq_running = is_service_running();
    if dnsmasq_running {
        out.push_str(&format!("\n{} local-dnsmasq is running", "✓".green()));
    } else {
        out.push_str(&format!("\n{} local-dnsmasq is NOT running", "✗".red().bold()));
    }

    let mut issues = Vec::new();

    let resolved_routing = check_resolved_routing();
    if resolved_routing {
        out.push_str(&format!("\n{} systemd-resolved routing is configured", "✓".green()));
    } else {
        issues.push("systemd-resolved routing not configured for .test/.dev domains");
        out.push_str(&format!("\n{} systemd-resolved routing not configured", "✗".yellow()));
    }

    if dnsmasq_running {
        let port_free = !state.port_listeners.contains_key(&5354)
            || state.port_listeners.get(&5354).map(|p| p.contains("dnsmasq")).unwrap_or(false);
        if port_free {
            out.push_str(&format!("\n{} Port 5354 is ready for dnsmasq", "✓".green()));
        } else {
            issues.push("Port 5354 is in use by another process");
            out.push_str(&format!("\n{} Port 5354 in use by '{}'", "✗".yellow(), state.port_listeners[&5354]));
        }
    }

    if issues.is_empty() {
        out.push_str(&format!("\n\n{} No issues detected.", "✓".green().bold()));
    } else {
        out.push_str(&format!("\n\n{} Issues found:", "⚠".yellow().bold()));
        for i in &issues {
            out.push_str(&format!("\n  • {i}"));
        }
        if fix {
            out.push_str(&format!("\n\n{} Attempting fixes...", "🔧".cyan().bold()));
            if !dnsmasq_running {
                if supports_systemd() {
                    let start = Command::new("systemctl")
                        .args(["start", DNSMASQ_SERVICE])
                        .status();
                    if start.ok().is_some_and(|s| s.success()) {
                        out.push_str(&format!("\n{} Started local-dnsmasq", "✓".green()));
                    } else {
                        out.push_str(&format!("\n{} Could not start local-dnsmasq", "✗".red()));
                    }
                }
            }
            if !resolved_routing {
                match configure_resolved_routing() {
                    Ok(()) => out.push_str(&format!("\n{} Configured DNS routing via systemd-resolved", "✓".green())),
                    Err(e) => out.push_str(&format!("\n{} {e}", "✗".red())),
                }
            }
            out.push_str(&format!("\n{} Doctor complete. Try pinging your local domain now.", "✓".green().bold()));
        } else {
            out.push_str(&format!("\n\n{} Run with --fix to auto-correct", "💡".cyan()));
        }
    }

    Ok(out)
}

fn check_resolved_routing() -> bool {
    if !has_tool("resolvectl") {
        return false;
    }
    if let Ok(out) = Command::new("resolvectl").args(["domain", "lo"]).output() {
        let s = String::from_utf8_lossy(&out.stdout);
        s.contains("~test") || s.contains("~dev")
    } else {
        false
    }
}

// ═══════════════════════════════════════════════════════════
//  INIT
// ═══════════════════════════════════════════════════════════

fn cmd_init() -> Result<String, String> {
    println!("{}\n", "🔍 Detecting system...".cyan().bold());
    let state = detect_system()?;
    println!("{}", format_detect(&state));

    if !supports_dnsmasq() {
        let hint = if cfg!(target_os = "macos") {
            "Install: brew install dnsmasq"
        } else if cfg!(target_os = "windows") {
            "Install WSL or use: winget install dnsmasq"
        } else {
            "Install: sudo apt install dnsmasq / sudo dnf install dnsmasq"
        };
        return Err(format!("dnsmasq not found. {hint}"));
    }

    let d = data_dir();
    fs::create_dir_all(&d).map_err(|e| format!("Cannot create {d:?}: {e}"))?;
    ensure_run_dir()?;

    let conn = init_db()?;
    conn.execute(
        "INSERT OR IGNORE INTO profiles (name, is_active) VALUES ('default', 1)",
        [],
    )
    .ok();
    ensure_defaults(&conn)?;

    write_dnsmasq_conf(&state)?;
    install_service()?;

    println!(
        "\n{}",
        "╔══════════════════════════════════════════╗".green()
    );
    println!(
        "{}",
        "║     local-dns — Setup Complete           ║"
            .green()
            .bold()
    );
    println!(
        "{}\n",
        "╚══════════════════════════════════════════╝".green()
    );
    println!("  {} {}/", "Data:".bold(), d.display().to_string().cyan());
    println!(
        "  {} {}",
        "Database:".bold(),
        db_path().display().to_string().cyan()
    );
    println!(
        "  {} {}/dnsmasq.conf",
        "Config:".bold(),
        d.display().to_string().cyan()
    );
    println!(
        "  {} {}",
        "Logs:".bold(),
        log_path().display().to_string().cyan()
    );
    println!(
        "  {} {}:{}\n",
        "Upstream:".bold(),
        state.upstream_dns.cyan(),
        state.upstream_port.to_string().cyan()
    );

    if let Some(ref s) = state.dnsdist_snippet {
        println!("{}", "dnsdist detected!".yellow().bold());
        println!(
            "{}\n\n{s}\n\n{}\n",
            "Add to /etc/dnsdist/dnsdist.conf:".yellow(),
            "Then: sudo systemctl restart dnsdist".yellow()
        );
    }
    if has_tool("resolvectl") {
        if check_resolved_routing() {
            println!("  {} DNS routing already configured", "✓".green());
        } else {
            println!(
                "{}",
                "Configuring DNS routing via systemd-resolved..."
                    .yellow()
                    .bold()
            );
            if let Err(e) = configure_resolved_routing() {
                eprintln!("  {} {e}", "⚠".yellow());
                if let Some(ref c) = state.resolved_conf {
                    println!("\n  To configure manually:\n  {c}\n");
                }
            }
        }
    } else {
        println!(
            "  {} To route .test/.dev domains to dnsmasq, set your system DNS to 127.0.0.1:5354",
            "ℹ".yellow()
        );
    }

    println!(
        "{}",
        "────────────────────────────────────────────".dimmed()
    );
    println!("{}", "Next:".bold());
    println!(
        "{}",
        "────────────────────────────────────────────".dimmed()
    );
    println!("  {}  {}", "1.".green().bold(), start_service_hint());
    println!("  {}  local-dns add myapp.test 127.0.0.1", "2.".green().bold());
    println!("  {}  ping myapp.test\n", "3.".green().bold());
    Ok(String::new())
}

fn cmd_reset() -> Result<String, String> {
    let db = db_path();
    if db.exists() {
        fs::remove_file(&db).map_err(|e| format!("Cannot remove database: {e}"))?;
        println!("{} Removed existing database.", "✓".green());
    }
    cmd_init()
}

fn start_service_hint() -> String {
    if supports_systemd() {
        format!("sudo systemctl enable --now {DNSMASQ_SERVICE}")
    } else if supports_launchctl() {
        "brew services start dnsmasq && launchctl load ...".into()
    } else {
        format!(
            "Start dnsmasq manually: {} -k --conf-file={}/dnsmasq.conf",
            dnsmasq_binary(),
            data_dir().display()
        )
    }
}

// ═══════════════════════════════════════════════════════════
//  ZONE & GROUP COMMANDS
// ═══════════════════════════════════════════════════════════

fn handle_zone(action: ZoneAction) -> Result<String, String> {
    match action {
        ZoneAction::Create { name, display } => {
            let conn = open_db()?;
            let p = active_profile(&conn)?;
            conn.execute(
                "INSERT INTO zones (profile_id, name, display_name) VALUES (?1, ?2, ?3)",
                params![p.id, name, display],
            )
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    format!("Zone '{name}' exists")
                } else {
                    format!("{e}")
                }
            })?;
            Ok(format!("{} Zone '{name}' created", "✓".green()))
        }
        ZoneAction::List => {
            let conn = open_db_readonly()?;
            let p = active_profile(&conn)?;
            let z = zone_list(&conn, p.id)?;
            if z.is_empty() {
                return Ok(format!("{}", "No zones.".yellow()));
            }
            Ok(format!(
                "{}:\n{}",
                "Zones".bold(),
                z.iter()
                    .map(|(n, d)| format!(
                        "  {}{}",
                        n,
                        d.as_deref()
                            .map(|s| format!(" ({})", s.cyan()))
                            .unwrap_or_default()
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            ))
        }
        ZoneAction::Show { name } => {
            let conn = open_db_readonly()?;
            let p = active_profile(&conn)?;
            let zid = resolve_zone(&conn, p.id, &name)?;
            let dsp: Option<String> = conn
                .query_row(
                    "SELECT display_name FROM zones WHERE id = ?1",
                    params![zid],
                    |row| row.get(0),
                )
                .ok()
                .flatten();
            let grps = group_list(&conn, zid)?;
            let mut out = format!(
                "{} {}{}\n{}:\n",
                "Zone:".yellow().bold(),
                name,
                dsp.as_deref()
                    .map(|s| format!(" ({})", s.cyan()))
                    .unwrap_or_default(),
                "Groups".bold()
            );
            if grps.is_empty() {
                out.push_str(&format!("  {}\n", "(empty)".yellow()));
            } else {
                for (g, d) in &grps {
                    out.push_str(&format!(
                        "  {}{}\n",
                        g.green(),
                        d.as_deref()
                            .map(|s| format!(" ({})", s.cyan()))
                            .unwrap_or_default()
                    ));
                }
            }
            Ok(out.trim().into())
        }
        ZoneAction::Delete { name } => {
            let conn = open_db()?;
            let p = active_profile(&conn)?;
            let zid = resolve_zone(&conn, p.id, &name)?;
            conn.execute("DELETE FROM zones WHERE id = ?1", params![zid])
                .ok();
            Ok(format!("{} Zone '{name}' deleted", "✓".green()))
        }
    }
}

fn handle_group(action: GroupAction) -> Result<String, String> {
    match action {
        GroupAction::Create {
            name,
            zone,
            display,
        } => {
            let conn = open_db()?;
            let p = active_profile(&conn)?;
            let zid = resolve_zone(&conn, p.id, &zone)?;
            conn.execute(
                "INSERT INTO groups (zone_id, name, display_name) VALUES (?1, ?2, ?3)",
                params![zid, name, display],
            )
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    format!("Group '{name}' exists in '{zone}'")
                } else {
                    format!("{e}")
                }
            })?;
            Ok(format!(
                "{} Group '{name}' created in '{zone}'",
                "✓".green()
            ))
        }
        GroupAction::List { zone } => {
            let conn = open_db_readonly()?;
            let p = active_profile(&conn)?;
            if let Some(z) = zone {
                let zid = resolve_zone(&conn, p.id, &z)?;
                let grps = group_list(&conn, zid)?;
                if grps.is_empty() {
                    return Ok(format!("{} No groups in '{z}'.", "⚠".yellow()));
                }
                Ok(format!(
                    "{} Groups in '{z}':\n{}",
                    "Groups".bold(),
                    grps.iter()
                        .map(|(n, d)| format!(
                            "  {}{}",
                            n.green(),
                            d.as_deref()
                                .map(|s| format!(" ({})", s.cyan()))
                                .unwrap_or_default()
                        ))
                        .collect::<Vec<_>>()
                        .join("\n")
                ))
            } else {
                let zs = zone_list(&conn, p.id)?;
                let mut out = format!("{}:\n", "Groups".bold());
                for (zn, _) in &zs {
                    if let Ok(zid) = resolve_zone(&conn, p.id, zn) {
                        if let Ok(gs) = group_list(&conn, zid) {
                            for (gn, _) in &gs {
                                out.push_str(&format!("  {}/{}\n", zn.yellow(), gn.green()));
                            }
                        }
                    }
                }
                Ok(out.trim().into())
            }
        }
        GroupAction::Delete { name, zone } => {
            let conn = open_db()?;
            let p = active_profile(&conn)?;
            let zid = resolve_zone(&conn, p.id, &zone)?;
            conn.execute(
                "DELETE FROM groups WHERE zone_id = ?1 AND name = ?2",
                params![zid, name],
            )
            .ok();
            Ok(format!(
                "{} Group '{name}' deleted from '{zone}'",
                "✓".green()
            ))
        }
    }
}

// ═══════════════════════════════════════════════════════════
//  ENTRY COMMANDS
// ═══════════════════════════════════════════════════════════

fn add_entry(
    domain: &str,
    ip: &str,
    zone: Option<&str>,
    group: Option<&str>,
    comment: Option<&str>,
) -> Result<String, String> {
    let conn = open_db()?;
    let p = active_profile(&conn)?;
    let (_, def_gid) = ensure_defaults(&conn)?;
    let zn = zone.unwrap_or("global");
    let gn = group.unwrap_or("main");
    let zid = if zn == "global" {
        conn.query_row(
            "SELECT id FROM zones WHERE profile_id = ?1 AND name = 'global'",
            params![p.id],
            |row| row.get(0),
        )
        .unwrap()
    } else {
        resolve_zone(&conn, p.id, zn)?
    };
    let gid = if gn == "main" && zn == "global" {
        def_gid
    } else {
        resolve_group(&conn, zid, gn).unwrap_or_else(|_| {
            conn.execute(
                "INSERT INTO groups (zone_id, name) VALUES (?1, ?2)",
                params![zid, gn],
            )
            .map_err(|e| format!("Cannot create group '{gn}' in '{zn}': {e}"))
            .ok();
            conn.last_insert_rowid()
        })
    };
    conn.execute(
        "INSERT INTO entries (group_id, domain, ip, comment, sort_key) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![gid, domain, ip, comment, domain.trim_start_matches("*.")],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            format!("Entry '{domain}' exists")
        } else {
            format!("{e}")
        }
    })?;
    cmd_apply()?;
    let loc = if zn != "global" || gn != "main" {
        format!(" ({zn}/{gn})")
    } else {
        String::new()
    };
    Ok(format!("{} Added {domain} → {ip}{loc}", "✓".green()))
}

fn remove_entry(domain: &str) -> Result<String, String> {
    let conn = open_db()?;
    let p = active_profile(&conn)?;
    let d = conn.execute("DELETE FROM entries WHERE id IN (SELECT e.id FROM entries e JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id WHERE z.profile_id = ?1 AND e.domain = ?2)", params![p.id, domain])
        .map_err(|e| format!("{e}"))?;
    if d == 0 {
        return Err(format!("Entry '{domain}' not found"));
    }
    cmd_apply()?;
    Ok(format!("{} Removed {domain}", "✓".green()))
}

fn move_entry(domain: &str, zone: Option<&str>, group: Option<&str>) -> Result<String, String> {
    let conn = open_db()?;
    let p = active_profile(&conn)?;
    let (_, def_gid) = ensure_defaults(&conn)?;
    let zn = zone.unwrap_or("global");
    let gn = group.unwrap_or("main");
    let zid = if zn == "global" {
        conn.query_row(
            "SELECT id FROM zones WHERE profile_id = ?1 AND name = 'global'",
            params![p.id],
            |row| row.get(0),
        )
        .unwrap()
    } else {
        resolve_zone(&conn, p.id, zn)?
    };
    let gid = if gn == "main" && zn == "global" {
        def_gid
    } else {
        resolve_group(&conn, zid, gn).unwrap_or_else(|_| {
            conn.execute(
                "INSERT INTO groups (zone_id, name) VALUES (?1, ?2)",
                params![zid, gn],
            )
            .map_err(|e| format!("Cannot create group '{gn}' in '{zn}': {e}"))
            .ok();
            conn.last_insert_rowid()
        })
    };

    let entry = conn
        .query_row(
            "SELECT e.id, e.domain, e.ip, e.comment, e.sort_key, e.group_id FROM entries e
         JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id
         WHERE z.profile_id = ?1 AND e.domain = ?2",
            params![p.id, domain],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .map_err(|_| format!("Entry '{domain}' not found"))?;

    let (eid, domain_name, ip, comment, sort_key, old_gid) = entry;
    if old_gid == gid {
        return Err(format!("Entry '{domain}' is already in '{zn}/{gn}'"));
    }

    conn.execute(
        "INSERT INTO entries (group_id, domain, ip, comment, sort_key) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![gid, domain_name, ip, comment, sort_key],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            format!("Entry '{domain}' already exists in '{zn}/{gn}'")
        } else {
            format!("{e}")
        }
    })?;
    conn.execute("DELETE FROM entries WHERE id = ?1", params![eid])
        .ok();
    cmd_apply()?;
    Ok(format!("{} Moved {domain} → {zn}/{gn}", "✓".green()))
}

fn copy_entry(domain: &str, zone: Option<&str>, group: Option<&str>) -> Result<String, String> {
    let conn = open_db()?;
    let p = active_profile(&conn)?;
    let (_, def_gid) = ensure_defaults(&conn)?;
    let zn = zone.unwrap_or("global");
    let gn = group.unwrap_or("main");
    let zid = if zn == "global" {
        conn.query_row(
            "SELECT id FROM zones WHERE profile_id = ?1 AND name = 'global'",
            params![p.id],
            |row| row.get(0),
        )
        .unwrap()
    } else {
        resolve_zone(&conn, p.id, zn)?
    };
    let gid = if gn == "main" && zn == "global" {
        def_gid
    } else {
        resolve_group(&conn, zid, gn).unwrap_or_else(|_| {
            conn.execute(
                "INSERT INTO groups (zone_id, name) VALUES (?1, ?2)",
                params![zid, gn],
            )
            .map_err(|e| format!("Cannot create group '{gn}' in '{zn}': {e}"))
            .ok();
            conn.last_insert_rowid()
        })
    };

    let entry = conn
        .query_row(
            "SELECT e.domain, e.ip, e.comment, e.sort_key FROM entries e
         JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id
         WHERE z.profile_id = ?1 AND e.domain = ?2",
            params![p.id, domain],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .map_err(|_| format!("Entry '{domain}' not found"))?;

    let (domain_name, ip, comment, sort_key) = entry;
    conn.execute(
        "INSERT INTO entries (group_id, domain, ip, comment, sort_key) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![gid, domain_name, ip, comment, sort_key],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            format!("Entry '{domain}' already exists in '{zn}/{gn}'")
        } else {
            format!("{e}")
        }
    })?;
    cmd_apply()?;
    Ok(format!("{} Copied {domain} → {zn}/{gn}", "✓".green()))
}

fn edit_entry(domain: &str, ip: Option<&str>, comment: Option<&str>) -> Result<String, String> {
    let conn = open_db()?;
    let p = active_profile(&conn)?;

    let exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM entries e JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id WHERE z.profile_id = ?1 AND e.domain = ?2",
        params![p.id, domain], |row| row.get::<_, i64>(0),
    ).unwrap_or(0) > 0;
    if !exists {
        return Err(format!("Entry '{domain}' not found"));
    }

    let mut changes = Vec::new();
    if let Some(new_ip) = ip {
        conn.execute("UPDATE entries SET ip = ?1 WHERE id IN (SELECT e.id FROM entries e JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id WHERE z.profile_id = ?2 AND e.domain = ?3)",
            params![new_ip, p.id, domain]).map_err(|e| format!("{e}"))?;
        changes.push(format!("ip → {new_ip}"));
    }
    if let Some(new_comment) = comment {
        conn.execute("UPDATE entries SET comment = ?1 WHERE id IN (SELECT e.id FROM entries e JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id WHERE z.profile_id = ?2 AND e.domain = ?3)",
            params![new_comment, p.id, domain]).map_err(|e| format!("{e}"))?;
        changes.push(format!("comment → '{new_comment}'"));
    }

    if changes.is_empty() {
        return Err("Nothing to edit. Use --ip or --comment.".into());
    }
    cmd_apply()?;
    Ok(format!(
        "{} Edited {domain}: {}",
        "✓".green(),
        changes.join(", ")
    ))
}

fn list_entries() -> Result<String, String> {
    let conn = open_db_readonly()?;
    let p = active_profile(&conn)?;
    let entries = all_entries(&conn, p.id)?;
    if entries.is_empty() {
        let zs = zone_list(&conn, p.id)?;
        if zs.is_empty() {
            return Ok(format!(
                "{} Use `local-dns add myapp.test 127.0.0.1`",
                "No entries.".yellow()
            ));
        }
        return Ok(format!("{}", "No entries.".yellow()));
    }
    let mut out = format!("{} {}\n", "Profile:".cyan().bold(), p.name);
    let mut cz = String::new();
    let mut cg = String::new();
    for e in &entries {
        if e.zone_name != cz {
            cz = e.zone_name.clone();
            let d: Option<String> = conn
                .query_row(
                    "SELECT display_name FROM zones WHERE profile_id = ?1 AND name = ?2",
                    params![p.id, cz],
                    |row| row.get(0),
                )
                .ok()
                .flatten();
            out.push_str(&format!(
                "\n  {} {}{}\n",
                "Zone:".yellow().bold(),
                cz,
                d.as_deref()
                    .map(|s| format!(" ({})", s.cyan()))
                    .unwrap_or_default()
            ));
            cg.clear();
        }
        if e.group_name != cg {
            cg = e.group_name.clone();
            let d: Option<String> = conn.query_row("SELECT gr.display_name FROM groups gr JOIN zones z ON gr.zone_id = z.id WHERE z.profile_id = ?1 AND z.name = ?2 AND gr.name = ?3", params![p.id, cz, cg], |row| row.get(0)).ok().flatten();
            out.push_str(&format!(
                "    {} {}{}\n",
                "Group:".green().bold(),
                cg,
                d.as_deref()
                    .map(|s| format!(" ({})", s.cyan()))
                    .unwrap_or_default()
            ));
        }
        out.push_str(&format!(
            "      {:<30} {:<15}  {}\n",
            e.domain.green(),
            e.ip.cyan(),
            e.comment.as_deref().unwrap_or("").dimmed()
        ));
    }
    Ok(out)
}

fn cmd_check(domain: &str) -> Result<String, String> {
    let conn = open_db_readonly()?;
    let p = active_profile(&conn)?;

    if domain == "all" {
        let entries = all_entries(&conn, p.id)?;
        if entries.is_empty() {
            return Ok(format!("{}", "No entries found.".yellow()));
        }
        let mut out = String::new();
        for e in &entries {
            let loaded = is_entry_loaded(&e.domain);
            let loaded_str = if loaded {
                "loaded ✓".green().to_string()
            } else {
                "needs apply".yellow().to_string()
            };
            out.push_str(&format!(
                "{:<30} created: {:<20} {}\n",
                e.domain.green(),
                e.created_at.cyan(),
                loaded_str
            ));
        }
        return Ok(out);
    }

    // Specific domain
    let entry: Option<Entry> = conn
        .query_row(
            "SELECT e.domain, e.ip, e.comment, z.name, g.name, e.created_at FROM entries e
         JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id
         WHERE z.profile_id = ?1 AND e.domain = ?2",
            params![p.id, domain],
            |row| {
                Ok(Entry {
                    domain: row.get(0)?,
                    ip: row.get(1)?,
                    comment: row.get(2)?,
                    zone_name: row.get(3)?,
                    group_name: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )
        .ok();

    match entry {
        Some(e) => {
            let loaded = is_entry_loaded(&e.domain);
            let loaded_str = if loaded {
                "loaded ✓".green().to_string()
            } else {
                "needs apply".yellow().to_string()
            };
            let mut out = format!("{} {}\n", "Domain:".cyan().bold(), e.domain.green());
            out.push_str(&format!("{} {}\n", "IP:".bold(), e.ip.cyan()));
            out.push_str(&format!("{} {}\n", "Zone:".bold(), e.zone_name));
            out.push_str(&format!("{} {}\n", "Group:".bold(), e.group_name));
            out.push_str(&format!("{} {}\n", "Created:".bold(), e.created_at.cyan()));
            out.push_str(&format!(
                "{} {}\n",
                "Comment:".bold(),
                e.comment.as_deref().unwrap_or("(none)")
            ));
            out.push_str(&format!("{} {}\n", "Status:".bold(), loaded_str));
            Ok(out)
        }
        None => Err(format!("Entry '{domain}' not found")),
    }
}

fn is_entry_loaded(domain: &str) -> bool {
    let hosts_path = run_dir().join("hosts");
    if !hosts_path.exists() {
        return false;
    }
    fs::read_to_string(hosts_path)
        .ok()
        .is_some_and(|content| {
            let clean = domain.trim_start_matches("*.");
            content
                .lines()
                .any(|line| line.split_whitespace().nth(1) == Some(clean))
        })
}

// ═══════════════════════════════════════════════════════════
//  PROFILE COMMANDS
// ═══════════════════════════════════════════════════════════

fn handle_profile(action: ProfileAction) -> Result<String, String> {
    match action {
        ProfileAction::Show => {
            let conn = open_db_readonly()?;
            Ok(format!(
                "{} {}",
                "Active profile:".cyan().bold(),
                active_profile(&conn)?.name.green().bold()
            ))
        }
        ProfileAction::Switch { name } => {
            let conn = open_db()?;
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM profiles WHERE name = ?1",
                    params![name],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0)
                > 0;
            if !exists {
                return Err(format!("Profile '{name}' not found"));
            }
            conn.execute("UPDATE profiles SET is_active = 0", []).ok();
            conn.execute(
                "UPDATE profiles SET is_active = 1 WHERE name = ?1",
                params![name],
            )
            .ok();
            cmd_apply()?;
            Ok(format!("{} Switched to '{name}'", "➜".green().bold()))
        }
        ProfileAction::Create { name } => {
            let conn = open_db()?;
            conn.execute("INSERT INTO profiles (name) VALUES (?1)", params![name])
                .map_err(|e| {
                    if e.to_string().contains("UNIQUE") {
                        format!("Profile '{name}' exists")
                    } else {
                        format!("{e}")
                    }
                })?;
            Ok(format!("{} Profile '{name}' created", "✓".green()))
        }
        ProfileAction::List => {
            let conn = open_db_readonly()?;
            let n = profile_names(&conn)?;
            Ok(if n.is_empty() {
                "No profiles.".yellow().to_string()
            } else {
                format!(
                    "{}:\n{}",
                    "Profiles".bold(),
                    n.iter()
                        .map(|x| format!("  {x}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })
        }
        ProfileAction::Delete { name } => {
            let conn = open_db()?;
            if name == active_profile(&conn)?.name {
                return Err("Cannot delete active profile".into());
            }
            conn.execute("DELETE FROM profiles WHERE name = ?1", params![name])
                .ok();
            Ok(format!("{} Profile '{name}' deleted", "✓".green()))
        }
    }
}

// ═══════════════════════════════════════════════════════════
//  TELEMETRY
// ═══════════════════════════════════════════════════════════

fn handle_telemetry(action: TelemetryAction, tel: &telemetry::Telemetry) -> Result<String, String> {
    match action {
        TelemetryAction::Enable => {
            let mut t = telemetry::Telemetry::load(&data_dir());
            t.enable()?;
            Ok(format!(
                "{} Anonymous telemetry enabled\n{}",
                "✓".green(),
                t.status()
            ))
        }
        TelemetryAction::Disable => {
            let mut t = telemetry::Telemetry::load(&data_dir());
            t.disable()?;
            Ok(format!(
                "{} Anonymous telemetry disabled\n{}",
                "✓".yellow(),
                t.status()
            ))
        }
        TelemetryAction::Status => Ok(tel.status()),
    }
}

// ═══════════════════════════════════════════════════════════
//  APPLY
// ═══════════════════════════════════════════════════════════

fn cmd_apply() -> Result<String, String> {
    let conn = open_db()?;
    let p = active_profile(&conn)?;
    let entries = all_entries(&conn, p.id)?;
    ensure_run_dir()?;
    let mut lines: Vec<String> = entries
        .iter()
        .map(|e| {
            let domain = e.domain.trim_start_matches("*.");
            let _tag = format!("[{}]", e.group_name);
            match &e.comment {
                Some(c) => format!(
                    "{}\t{domain}\t# {c} [{}/{}]",
                    e.ip, e.zone_name, e.group_name
                ),
                None => format!("{}\t{domain}\t# [{}/{}]", e.ip, e.zone_name, e.group_name),
            }
        })
        .collect();
    lines.sort_by(|a, b| {
        a.split_whitespace()
            .nth(1)
            .unwrap_or("")
            .cmp(b.split_whitespace().nth(1).unwrap_or(""))
    });
    lines.push(String::new());
    let hosts_path = run_dir().join("hosts");
    fs::write(hosts_path, lines.join("\n")).map_err(|e| format!("{e}"))?;
    reload_dnsmasq()?;
    Ok(format!(
        "{} Applied '{}' ({} entries)",
        "✓".green(),
        p.name,
        entries.len().to_string().cyan()
    ))
}

// ═══════════════════════════════════════════════════════════
//  STATUS
// ═══════════════════════════════════════════════════════════

fn cmd_status() -> Result<String, String> {
    let mut out = String::new();
    let state = detect_system().ok();
    let running = is_service_running();
    out.push_str(&format!(
        "{} {}\n",
        "Platform:".cyan().bold(),
        std::env::consts::OS
    ));
    let status_color = if running {
        "running ✓".green()
    } else {
        "stopped ✗".red().bold()
    };
    out.push_str(&format!("local-dnsmasq: {status_color}\n"));
    if let Some(ref s) = state {
        for svc in &s.services {
            out.push_str(&format!("  {}: {}\n", svc.cyan(), "running".green()));
        }
    }
    if let Ok(conn) = open_db_readonly() {
        if let Ok(p) = active_profile(&conn) {
            out.push_str(&format!(
                "\n{} {}\n",
                "Profile:".cyan().bold(),
                p.name.bold()
            ));
            if let Ok(zs) = zone_list(&conn, p.id) {
                for (zn, zd) in &zs {
                    if let Ok(zid) = resolve_zone(&conn, p.id, zn) {
                        if let Ok(gs) = group_list(&conn, zid) {
                            for (gn, gd) in &gs {
                                let c: i64 = conn.query_row("SELECT COUNT(*) FROM entries e JOIN groups g ON e.group_id = g.id WHERE g.zone_id = ?1 AND g.name = ?2", params![zid, gn], |row| row.get(0)).unwrap_or(0);
                                let ecount = if c > 0 {
                                    c.to_string().green()
                                } else {
                                    c.to_string().yellow()
                                };
                                out.push_str(&format!(
                                    "  {}{}/{}{}: {ecount} entries\n",
                                    zn.yellow(),
                                    zd.as_deref()
                                        .map(|s| format!(" ({})", s.cyan()))
                                        .unwrap_or_default(),
                                    gn.green(),
                                    gd.as_deref()
                                        .map(|s| format!(" ({})", s.cyan()))
                                        .unwrap_or_default()
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    let lp = log_path();
    if let Ok(m) = fs::metadata(&lp) {
        if m.len() > 0 {
            out.push_str(&format!(
                "\n{} {}\n",
                "Log:".cyan().bold(),
                lp.display().to_string().cyan()
            ));
        }
    }
    out.push_str(&format!(
        "\n{} {}\n",
        "dnsmasq:".bold(),
        dnsmasq_binary().cyan()
    ));
    out.push_str(&format!(
        "{} {}\n",
        "Data:".bold(),
        data_dir().display().to_string().cyan()
    ));
    Ok(out)
}

// ═══════════════════════════════════════════════════════════
//  LOGS
// ═══════════════════════════════════════════════════════════

fn is_service_running() -> bool {
    if supports_systemd() {
        Command::new("systemctl")
            .args(["is-active", "--quiet", DNSMASQ_SERVICE])
            .status()
            .ok()
            .is_some_and(|s| s.success())
    } else if cfg!(target_os = "macos") {
        Command::new("launchctl")
            .args(["print", &format!("system/{DNSMASQ_SERVICE}")])
            .status()
            .ok()
            .is_some_and(|s| s.success())
    } else {
        // Check PID file
        fs::read_to_string(pid_path())
            .ok()
            .and_then(|p| p.trim().parse::<i32>().ok())
            .is_some_and(|pid| {
                Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .status()
                    .ok()
                    .is_some_and(|s| s.success())
            })
    }
}

fn cmd_logs(follow: bool, errors: bool, lines: usize) -> Result<String, String> {
    let lp = log_path();
    if !lp.exists() {
        return Err(format!("Log not found: {}", lp.display()));
    }
    if follow {
        if cfg!(target_os = "windows") {
            Command::new("powershell")
                .args(["Get-Content", "-Wait", &lp.to_string_lossy()])
                .status()
                .ok();
        } else {
            Command::new("tail")
                .args(["-f", &lp.to_string_lossy()])
                .status()
                .ok();
        }
        return Ok(String::new());
    }
    if errors {
        let r = if has_tool("rg") {
            Command::new("rg")
                .args([
                    "-i",
                    "(NXDOMAIN|REFUSED|SERVFAIL|TIMEOUT|error)",
                    &lp.to_string_lossy(),
                ])
                .output()
        } else if has_tool("grep") {
            Command::new("grep")
                .args([
                    "-i",
                    "-E",
                    "(NXDOMAIN|REFUSED|SERVFAIL|TIMEOUT|error)",
                    &lp.to_string_lossy(),
                ])
                .output()
        } else {
            return Err("No search tool found (rg/grep)".into());
        }
        .map_err(|e| format!("{e}"))?;
        let s = String::from_utf8_lossy(&r.stdout).to_string();
        return Ok(if s.is_empty() {
            "No errors.".green().to_string()
        } else {
            s.trim().into()
        });
    }
    let r = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args([
                "Get-Content",
                "-Tail",
                &lines.to_string(),
                &lp.to_string_lossy(),
            ])
            .output()
    } else {
        Command::new("tail")
            .args(["-n", &lines.to_string(), &lp.to_string_lossy()])
            .output()
    }
    .map_err(|e| format!("{e}"))?;
    let s = String::from_utf8_lossy(&r.stdout).to_string();
    Ok(if s.is_empty() {
        "Log is empty.".yellow().to_string()
    } else {
        s.trim().into()
    })
}

// ═══════════════════════════════════════════════════════════
//  SERVICE MANAGEMENT
// ═══════════════════════════════════════════════════════════

fn write_dnsmasq_conf(state: &SystemState) -> Result<(), String> {
    let hosts_path = run_dir().join("hosts");
    ensure_run_dir()?;
    if !hosts_path.exists() {
        fs::write(&hosts_path, "").map_err(|e| format!("Cannot create hosts file: {e}"))?;
    }

    let conf = format!(
        r#"# local-dns — managed by local-dns CLI
port=5354
bind-interfaces
listen-address=127.0.0.1
conf-dir={}
addn-hosts={}
domain-needed
bogus-priv
no-hosts
no-resolv
cache-size=1000
pid-file={}
server={}#{}
log-queries
log-facility={}
"#,
        run_dir().display(),
        hosts_path.display(),
        pid_path().display(),
        state.upstream_dns,
        state.upstream_port,
        log_path().display()
    );
    fs::write(data_dir().join("dnsmasq.conf"), &conf)
        .map_err(|e| format!("Cannot write config: {e}"))
}

fn install_service() -> Result<(), String> {
    if supports_systemd() {
        let unit = format!(
            r#"[Unit]
Description=local-dns — Local DNS management
After=network.target
[Service]
Type=simple
RuntimeDirectory=local-dns
ExecStart={} -k --conf-file={}/dnsmasq.conf
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5
[Install]
WantedBy=multi-user.target
"#,
            dnsmasq_binary(),
            data_dir().display()
        );
        let path = format!("/etc/systemd/system/{DNSMASQ_SERVICE}.service");
        fs::write(&path, &unit).map_err(|e| format!("Cannot write systemd unit: {e}"))?;
        println!("  {} {}", "Systemd unit:".bold(), path.cyan());
    } else if supports_launchctl() {
        // macOS launchd plist
        let plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>Label</key><string>{DNSMASQ_SERVICE}</string>
    <key>ProgramArguments</key><array>
        <string>{}</string><string>-k</string><string>--conf-file={}/dnsmasq.conf</string>
    </array>
    <key>KeepAlive</key><true/>
    <key>RunAtLoad</key><true/>
</dict></plist>"#,
            dnsmasq_binary(),
            data_dir().display()
        );
        let path = format!("/Library/LaunchDaemons/{DNSMASQ_SERVICE}.plist");
        fs::write(&path, &plist).map_err(|e| format!("Cannot write launchd plist: {e}"))?;
        println!("  {} {}", "LaunchDaemon:".bold(), path.cyan());
    } else {
        println!(
            "  {}",
            "(no systemd/launchd detected — start dnsmasq manually)".yellow()
        );
    }
    Ok(())
}

fn reload_dnsmasq() -> Result<(), String> {
    // 1) systemd reload
    if supports_systemd() {
        if let Ok(s) = Command::new("systemctl")
            .args(["reload", DNSMASQ_SERVICE])
            .status()
        {
            if s.success() {
                return Ok(());
            }
        }
    }

    // 2) launchctl reload (macOS)
    if supports_launchctl() {
        if let Ok(s) = Command::new("launchctl")
            .args(["kickstart", &format!("system/{DNSMASQ_SERVICE}")])
            .status()
        {
            if s.success() {
                return Ok(());
            }
        }
    }

    // 3) Direct SIGHUP via PID file
    if let Ok(p) = fs::read_to_string(pid_path()) {
        if let Ok(pid) = p.trim().parse::<i32>() {
            #[cfg(unix)]
            {
                if Command::new("kill")
                    .args(["-HUP", &pid.to_string()])
                    .status()
                    .ok()
                    .is_some_and(|s| s.success())
                {
                    return Ok(());
                }
            }
            #[cfg(windows)]
            {
                if Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/F"])
                    .status()
                    .ok()
                    .is_some_and(|s| s.success())
                {
                    return Ok(());
                }
            }
        }
    }

    // 4) Try getting PID from systemctl
    if supports_systemd() {
        if let Ok(o) = Command::new("systemctl")
            .args(["show", "--property", "MainPID", DNSMASQ_SERVICE])
            .output()
        {
            if let Some(ps) = String::from_utf8_lossy(&o.stdout)
                .trim()
                .strip_prefix("MainPID=")
            {
                if let Ok(pid) = ps.trim().parse::<i32>() {
                    if pid > 1 {
                        #[cfg(unix)]
                        {
                            if Command::new("kill")
                                .args(["-HUP", &pid.to_string()])
                                .status()
                                .ok()
                                .is_some_and(|s| s.success())
                            {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }

    // 5) Restart as last resort
    if supports_systemd() {
        Command::new("systemctl")
            .args(["restart", DNSMASQ_SERVICE])
            .status()
            .map(|s| {
                if s.success() {
                    Ok(())
                } else {
                    Err("Restart failed".into())
                }
            })
            .unwrap_or_else(|e| Err(format!("{e}")))
    } else if supports_launchctl() {
        Command::new("launchctl")
            .args(["stop", &format!("system/{DNSMASQ_SERVICE}")])
            .status()
            .ok();
        Command::new("launchctl")
            .args(["start", &format!("system/{DNSMASQ_SERVICE}")])
            .status()
            .map(|s| {
                if s.success() {
                    Ok(())
                } else {
                    Err("Restart failed".into())
                }
            })
            .unwrap_or_else(|e| Err(format!("{e}")))
    } else {
        Err("Cannot reload — start dnsmasq manually".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ─── helpers ───────────────────────────────────────────────

    fn test_db() -> (Connection, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).expect("open db");
        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .expect("pragmas");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS profiles (
                id    INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT UNIQUE NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS zones (
                id INTEGER PRIMARY KEY AUTOINCREMENT, profile_id INTEGER NOT NULL,
                name TEXT NOT NULL, display_name TEXT, sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE,
                UNIQUE(profile_id, name)
            );
            CREATE TABLE IF NOT EXISTS groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT, zone_id INTEGER NOT NULL,
                name TEXT NOT NULL, display_name TEXT, sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (zone_id) REFERENCES zones(id) ON DELETE CASCADE,
                UNIQUE(zone_id, name)
            );
            CREATE TABLE IF NOT EXISTS entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT, group_id INTEGER NOT NULL,
                domain TEXT NOT NULL, ip TEXT NOT NULL, comment TEXT, sort_key TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE,
                UNIQUE(group_id, domain)
            );
            CREATE INDEX IF NOT EXISTS idx_zones_profile ON zones(profile_id);
            CREATE INDEX IF NOT EXISTS idx_groups_zone ON groups(zone_id);
            CREATE INDEX IF NOT EXISTS idx_entries_group ON entries(group_id);"
        ).expect("schema");
        (conn, dir)
    }

    fn insert_profile(conn: &Connection, name: &str, active: bool) {
        conn.execute(
            "INSERT INTO profiles (name, is_active) VALUES (?1, ?2)",
            params![name, active as i32],
        )
        .expect("insert profile");
    }

    fn insert_zone(conn: &Connection, pid: i64, name: &str, display: Option<&str>) -> i64 {
        conn.execute(
            "INSERT INTO zones (profile_id, name, display_name) VALUES (?1, ?2, ?3)",
            params![pid, name, display],
        )
        .expect("insert zone");
        conn.last_insert_rowid()
    }

    fn insert_group(conn: &Connection, zid: i64, name: &str, display: Option<&str>) -> i64 {
        conn.execute(
            "INSERT INTO groups (zone_id, name, display_name) VALUES (?1, ?2, ?3)",
            params![zid, name, display],
        )
        .expect("insert group");
        conn.last_insert_rowid()
    }

    fn insert_entry(conn: &Connection, gid: i64, domain: &str, ip: &str, comment: Option<&str>) {
        conn.execute(
            "INSERT INTO entries (group_id, domain, ip, comment, sort_key) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![gid, domain, ip, comment, domain.trim_start_matches("*.")],
        ).expect("insert entry");
    }

    // ═════════════════════════════════════════════════════════
    //  active_profile
    // ═════════════════════════════════════════════════════════

    #[test]
    fn active_profile_errors_with_no_profiles() {
        let (conn, _dir) = test_db();
        assert!(active_profile(&conn).is_err());
    }

    #[test]
    fn active_profile_errors_when_none_active() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "offline", false);
        insert_profile(&conn, "standby", false);
        assert!(active_profile(&conn).is_err());
    }

    #[test]
    fn active_profile_returns_matching_active() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "a", false);
        insert_profile(&conn, "b", true);
        insert_profile(&conn, "c", false);
        let p = active_profile(&conn).unwrap();
        assert_eq!(p.name, "b");
        assert!(p.is_active);
    }

    // ═════════════════════════════════════════════════════════
    //  profile_names
    // ═════════════════════════════════════════════════════════

    #[test]
    fn profile_names_empty_when_no_profiles() {
        let (conn, _dir) = test_db();
        assert!(profile_names(&conn).unwrap().is_empty());
    }

    #[test]
    fn profile_names_returns_sorted() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "z", false);
        insert_profile(&conn, "a", false);
        insert_profile(&conn, "m", false);
        assert_eq!(profile_names(&conn).unwrap(), vec!["a", "m", "z"]);
    }

    // ═════════════════════════════════════════════════════════
    //  resolve_zone  /  resolve_group
    // ═════════════════════════════════════════════════════════

    #[test]
    fn resolve_zone_returns_id_for_existing() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "mytest", None);
        assert_eq!(resolve_zone(&conn, pid, "mytest").unwrap(), zid);
    }

    #[test]
    fn resolve_zone_errors_on_missing() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let err = resolve_zone(&conn, pid, "nope").unwrap_err();
        assert!(err.contains("nope"));
    }

    #[test]
    fn resolve_zone_scoped_by_profile() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "a", true);
        insert_profile(&conn, "b", false);
        let pa = active_profile(&conn).unwrap();
        let pb = Profile {
            id: 2,
            name: "b".into(),
            is_active: false,
        };
        insert_zone(&conn, pa.id, "shared", None);
        insert_zone(&conn, pb.id, "shared", None);
        assert!(resolve_zone(&conn, pa.id, "shared").is_ok());
        assert!(resolve_zone(&conn, pb.id, "shared").is_ok());
    }

    #[test]
    fn resolve_group_returns_id_for_existing() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "z", None);
        let gid = insert_group(&conn, zid, "mygroup", None);
        assert_eq!(resolve_group(&conn, zid, "mygroup").unwrap(), gid);
    }

    #[test]
    fn resolve_group_errors_on_missing() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "z", None);
        let err = resolve_group(&conn, zid, "absent").unwrap_err();
        assert!(err.contains("absent"));
    }

    // ═════════════════════════════════════════════════════════
    //  ensure_defaults
    // ═════════════════════════════════════════════════════════

    #[test]
    fn ensure_defaults_creates_global_zone_and_main_group() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "default", true);
        let (zid, gid) = ensure_defaults(&conn).unwrap();
        assert!(zid > 0);
        assert!(gid > 0);
        let zones = zone_list(&conn, active_profile(&conn).unwrap().id).unwrap();
        assert!(zones.iter().any(|(n, _)| n == "global"));
        let groups = group_list(&conn, zid).unwrap();
        assert!(groups.iter().any(|(n, _)| n == "main"));
    }

    #[test]
    fn ensure_defaults_is_idempotent() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "default", true);
        let (z1, g1) = ensure_defaults(&conn).unwrap();
        let (z2, g2) = ensure_defaults(&conn).unwrap();
        assert_eq!(z1, z2);
        assert_eq!(g1, g2);
    }

    #[test]
    fn ensure_defaults_errors_without_active_profile() {
        let (conn, _dir) = test_db();
        assert!(ensure_defaults(&conn).is_err());
    }

    // ═════════════════════════════════════════════════════════
    //  zone_list  /  group_list
    // ═════════════════════════════════════════════════════════

    #[test]
    fn zone_list_empty_when_no_zones() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        assert!(zone_list(&conn, pid).unwrap().is_empty());
    }

    #[test]
    fn zone_list_returns_zones_ordered() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        insert_zone(&conn, pid, "external", Some("External Sites"));
        insert_zone(&conn, pid, "internal", None);
        let list = zone_list(&conn, pid).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], ("external".into(), Some("External Sites".into())));
        assert_eq!(list[1], ("internal".into(), None));
    }

    #[test]
    fn group_list_empty_when_no_groups() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "z", None);
        assert!(group_list(&conn, zid).unwrap().is_empty());
    }

    #[test]
    fn group_list_returns_groups_ordered() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "z", None);
        insert_group(&conn, zid, "alpha", Some("Alpha Group"));
        insert_group(&conn, zid, "beta", None);
        let list = group_list(&conn, zid).unwrap();
        assert_eq!(list[0], ("alpha".into(), Some("Alpha Group".into())));
        assert_eq!(list[1], ("beta".into(), None));
    }

    // ═════════════════════════════════════════════════════════
    //  all_entries  (backing list_entries)
    // ═════════════════════════════════════════════════════════

    #[test]
    fn all_entries_empty_when_no_entries() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        assert!(all_entries(&conn, pid).unwrap().is_empty());
    }

    #[test]
    fn all_entries_returns_full_entry_data() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "dev", None);
        let gid = insert_group(&conn, zid, "frontend", None);
        insert_entry(&conn, gid, "app.dev", "10.0.0.1", Some("main web app"));
        let entries = all_entries(&conn, pid).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].domain, "app.dev");
        assert_eq!(entries[0].ip, "10.0.0.1");
        assert_eq!(entries[0].comment.as_deref(), Some("main web app"));
        assert_eq!(entries[0].zone_name, "dev");
        assert_eq!(entries[0].group_name, "frontend");
    }

    #[test]
    fn all_entries_scoped_to_active_profile() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "a", true);
        insert_profile(&conn, "b", false);
        let pa = active_profile(&conn).unwrap();
        let pb = Profile {
            id: 2,
            name: "b".into(),
            is_active: false,
        };
        let za = insert_zone(&conn, pa.id, "z", None);
        let zb = insert_zone(&conn, pb.id, "z", None);
        let ga = insert_group(&conn, za, "g", None);
        let gb = insert_group(&conn, zb, "g", None);
        insert_entry(&conn, ga, "only-a.test", "1.1.1.1", None);
        insert_entry(&conn, gb, "only-b.test", "2.2.2.2", None);
        assert_eq!(all_entries(&conn, pa.id).unwrap().len(), 1);
        assert_eq!(all_entries(&conn, pa.id).unwrap()[0].domain, "only-a.test");
    }

    #[test]
    fn all_entries_wildcard_sort_key_derived() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "z", None);
        let gid = insert_group(&conn, zid, "g", None);
        insert_entry(&conn, gid, "*.wild", "10.0.0.1", None);
        let sk: String = conn
            .query_row(
                "SELECT sort_key FROM entries WHERE domain = '*.wild'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sk, "wild");
    }

    // ═════════════════════════════════════════════════════════
    //  enforce-entry (add_entry / remove_entry db logic)
    // ═════════════════════════════════════════════════════════

    #[test]
    fn add_entry_inserts_into_default_global_main_when_unspecified() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "default", true);
        let pid = active_profile(&conn).unwrap().id;
        let (_, gid) = ensure_defaults(&conn).unwrap();
        conn.execute(
            "INSERT INTO entries (group_id, domain, ip, comment, sort_key) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![gid, "foo.test", "10.0.0.1", None::<&str>, "foo.test"],
        ).unwrap();
        let entries = all_entries(&conn, pid).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].zone_name, "global");
        assert_eq!(entries[0].group_name, "main");
    }

    #[test]
    fn add_entry_duplicate_domain_in_group_is_rejected() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "z", None);
        let gid = insert_group(&conn, zid, "g", None);
        insert_entry(&conn, gid, "dup.test", "1.1.1.1", None);
        let err = conn.execute(
            "INSERT INTO entries (group_id, domain, ip, sort_key) VALUES (?1, 'dup.test', '2.2.2.2', 'dup.test')",
            params![gid],
        );
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("UNIQUE"));
    }

    #[test]
    fn remove_entry_deletes_by_domain_in_profile() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "z", None);
        let gid = insert_group(&conn, zid, "g", None);
        insert_entry(&conn, gid, "gone.test", "1.1.1.1", None);
        let n = conn
            .execute(
                "DELETE FROM entries WHERE id IN (SELECT e.id FROM entries e
             JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id
             WHERE z.profile_id = ?1 AND e.domain = ?2)",
                params![pid, "gone.test"],
            )
            .unwrap();
        assert_eq!(n, 1);
        assert!(all_entries(&conn, pid).unwrap().is_empty());
    }

    #[test]
    fn remove_entry_nonexistent_returns_zero() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let n = conn
            .execute(
                "DELETE FROM entries WHERE id IN (SELECT e.id FROM entries e
             JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id
             WHERE z.profile_id = ?1 AND e.domain = ?2)",
                params![pid, "ghost.test"],
            )
            .unwrap();
        assert_eq!(n, 0);
    }

    // ═════════════════════════════════════════════════════════
    //  move_entry / copy_entry / edit_entry
    // ═════════════════════════════════════════════════════════

    #[test]
    fn move_entry_moves_to_different_group() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "dev", None);
        let ga = insert_group(&conn, zid, "frontend", None);
        let gb = insert_group(&conn, zid, "backend", None);
        insert_entry(&conn, ga, "app.test", "1.1.1.1", None);
        conn.execute(
            "UPDATE entries SET group_id = ?1 WHERE domain = 'app.test'",
            params![gb],
        )
        .unwrap();
        conn.execute(
            "DELETE FROM entries WHERE group_id = ?1 AND domain = 'app.test'",
            params![ga],
        )
        .ok();
        let entries = all_entries(&conn, pid).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].group_name, "backend");
    }

    #[test]
    fn move_entry_to_same_group_fails() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "dev", None);
        let gid = insert_group(&conn, zid, "g", None);
        insert_entry(&conn, gid, "same.test", "1.1.1.1", None);
        // moving via DB insert+delete — same group implies same gid
        let existing: i64 = conn
            .query_row(
                "SELECT count(*) FROM entries WHERE id IN (SELECT e.id FROM entries e
             JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id
             WHERE z.profile_id = ?1 AND e.domain = ?2)",
                params![pid, "same.test"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(existing, 1);
    }

    #[test]
    fn copy_entry_creates_duplicate_in_new_group() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "dev", None);
        let ga = insert_group(&conn, zid, "g1", None);
        let gb = insert_group(&conn, zid, "g2", None);
        insert_entry(&conn, ga, "cp.test", "1.1.1.1", None);
        conn.execute(
            "INSERT INTO entries (group_id, domain, ip, comment, sort_key)
             SELECT ?1, domain, ip, comment, sort_key FROM entries WHERE group_id = ?2 AND domain = 'cp.test'",
            params![gb, ga],
        ).unwrap();
        let entries = all_entries(&conn, pid).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn copy_entry_missing_domain_errors() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "dev", None);
        let gid = insert_group(&conn, zid, "g", None);
        let exists: bool = conn.query_row(
            "SELECT count(*) FROM entries e JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id WHERE z.profile_id = ?1 AND e.domain = ?2",
            params![pid, "nonexistent"], |r| r.get::<_, i64>(0),
        ).unwrap() > 0;
        assert!(!exists);
        let _gid = gid; // avoid unused warning
    }

    #[test]
    fn edit_entry_updates_ip() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "z", None);
        let gid = insert_group(&conn, zid, "g", None);
        insert_entry(&conn, gid, "ed.test", "1.1.1.1", None);
        conn.execute(
            "UPDATE entries SET ip = ?1 WHERE id IN (SELECT e.id FROM entries e JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id WHERE z.profile_id = ?2 AND e.domain = ?3)",
            params!["9.9.9.9", pid, "ed.test"],
        ).unwrap();
        let entries = all_entries(&conn, pid).unwrap();
        assert_eq!(entries[0].ip, "9.9.9.9");
    }

    #[test]
    fn edit_entry_updates_comment() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "z", None);
        let gid = insert_group(&conn, zid, "g", None);
        insert_entry(&conn, gid, "ed.test", "1.1.1.1", Some("old comment"));
        conn.execute(
            "UPDATE entries SET comment = ?1 WHERE id IN (SELECT e.id FROM entries e JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id WHERE z.profile_id = ?2 AND e.domain = ?3)",
            params!["new comment", pid, "ed.test"],
        ).unwrap();
        let entries = all_entries(&conn, pid).unwrap();
        assert_eq!(entries[0].comment.as_deref(), Some("new comment"));
    }

    #[test]
    fn edit_entry_nonexistent_domain_errors() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let exists: bool = conn.query_row(
            "SELECT count(*) FROM entries e JOIN groups g ON e.group_id = g.id JOIN zones z ON g.zone_id = z.id WHERE z.profile_id = ?1 AND e.domain = ?2",
            params![1i64, "ghost.test"], |r| r.get::<_, i64>(0),
        ).unwrap() > 0;
        assert!(!exists);
    }

    // ═════════════════════════════════════════════════════════
    //  handle-profile database logic
    // ═════════════════════════════════════════════════════════

    #[test]
    fn profile_create_adds_new_profile() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "myprofile", false);
        assert!(profile_names(&conn).unwrap().contains(&"myprofile".into()));
    }

    #[test]
    fn profile_create_duplicate_is_rejected() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "dup", false);
        let err = conn.execute("INSERT INTO profiles (name) VALUES ('dup')", []);
        assert!(err.is_err());
    }

    #[test]
    fn profile_switch_updates_active_flag() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "default", true);
        insert_profile(&conn, "staging", false);
        conn.execute("UPDATE profiles SET is_active = 0", [])
            .unwrap();
        conn.execute(
            "UPDATE profiles SET is_active = 1 WHERE name = 'staging'",
            [],
        )
        .unwrap();
        assert_eq!(active_profile(&conn).unwrap().name, "staging");
    }

    #[test]
    fn profile_delete_removes_target() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "active", true);
        insert_profile(&conn, "old", false);
        conn.execute("DELETE FROM profiles WHERE name = 'old'", [])
            .unwrap();
        assert_eq!(profile_names(&conn).unwrap(), vec!["active"]);
    }

    // ═════════════════════════════════════════════════════════
    //  handle-zone / handle-group database logic
    // ═════════════════════════════════════════════════════════

    #[test]
    fn zone_create_inserts_with_display_name() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        insert_zone(&conn, pid, "custom", Some("Custom Zone"));
        let list = zone_list(&conn, pid).unwrap();
        assert!(list.contains(&("custom".into(), Some("Custom Zone".into()))));
    }

    #[test]
    fn zone_create_duplicate_rejected_within_profile() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        insert_zone(&conn, pid, "dup", None);
        let err = conn.execute(
            "INSERT INTO zones (profile_id, name) VALUES (?1, 'dup')",
            params![pid],
        );
        assert!(err.is_err());
    }

    #[test]
    fn zone_create_duplicate_allowed_across_profiles() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "a", true);
        insert_profile(&conn, "b", false);
        let (pa, pb) = (1, 2);
        insert_zone(&conn, pa, "shared", None);
        insert_zone(&conn, pb, "shared", None);
    }

    #[test]
    fn zone_delete_cascades_to_groups_and_entries() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "temp", None);
        let gid = insert_group(&conn, zid, "g", None);
        insert_entry(&conn, gid, "x.test", "1.1.1.1", None);
        conn.execute("DELETE FROM zones WHERE id = ?1", params![zid])
            .unwrap();
        let g: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM groups WHERE zone_id = ?1",
                params![zid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(g, 0);
        let e: i64 = conn
            .query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(e, 0);
    }

    #[test]
    fn group_create_inserts_with_display_name() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "z", None);
        insert_group(&conn, zid, "workers", Some("Worker Nodes"));
        assert!(group_list(&conn, zid)
            .unwrap()
            .contains(&("workers".into(), Some("Worker Nodes".into())),));
    }

    #[test]
    fn group_create_duplicate_rejected_within_zone() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "z", None);
        insert_group(&conn, zid, "dup", None);
        let err = conn.execute(
            "INSERT INTO groups (zone_id, name) VALUES (?1, 'dup')",
            params![zid],
        );
        assert!(err.is_err());
    }

    #[test]
    fn group_create_duplicate_allowed_across_zones() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let za = insert_zone(&conn, pid, "za", None);
        let zb = insert_zone(&conn, pid, "zb", None);
        insert_group(&conn, za, "shared", None);
        insert_group(&conn, zb, "shared", None);
    }

    #[test]
    fn group_delete_cascades_to_entries() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "z", None);
        let gid = insert_group(&conn, zid, "g", None);
        insert_entry(&conn, gid, "del.test", "1.1.1.1", None);
        conn.execute("DELETE FROM groups WHERE id = ?1", params![gid])
            .unwrap();
        let e: i64 = conn
            .query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(e, 0);
    }

    // ═════════════════════════════════════════════════════════
    //  cascading deletes — full chain
    // ═════════════════════════════════════════════════════════

    #[test]
    fn cascade_profile_deletes_zones_groups_entries() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "target", false);
        let pid: i64 = 1;
        let zid = insert_zone(&conn, pid, "z", None);
        let gid = insert_group(&conn, zid, "g", None);
        insert_entry(&conn, gid, "gone.test", "9.9.9.9", None);
        conn.execute("DELETE FROM profiles WHERE id = ?1", params![pid])
            .unwrap();
        let p_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM profiles", [], |r| r.get(0))
            .unwrap();
        assert_eq!(p_count, 0);
        let z_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM zones", [], |r| r.get(0))
            .unwrap();
        assert_eq!(z_count, 0);
        let e_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(e_count, 0);
    }

    #[test]
    fn cascade_zone_deletes_groups_and_entries() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "z", None);
        let gid = insert_group(&conn, zid, "g", None);
        insert_entry(&conn, gid, "gone.test", "9.9.9.9", None);
        conn.execute("DELETE FROM zones WHERE id = ?1", params![zid])
            .unwrap();
        let g: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM groups WHERE zone_id = ?1",
                params![zid],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(g, 0);
        let e: i64 = conn
            .query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(e, 0);
    }

    #[test]
    fn cascade_group_deletes_entries() {
        let (conn, _dir) = test_db();
        insert_profile(&conn, "p", true);
        let pid = active_profile(&conn).unwrap().id;
        let zid = insert_zone(&conn, pid, "z", None);
        let gid = insert_group(&conn, zid, "g", None);
        insert_entry(&conn, gid, "gone.test", "9.9.9.9", None);
        conn.execute("DELETE FROM groups WHERE id = ?1", params![gid])
            .unwrap();
        let e: i64 = conn
            .query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(e, 0);
    }

    // ═════════════════════════════════════════════════════════
    //  system detection — has_tool, supports_dnsmasq
    // ═════════════════════════════════════════════════════════

    #[test]
    fn has_tool_finds_existing_command() {
        if cfg!(unix) {
            assert!(has_tool("sh"));
        } else {
            assert!(has_tool("cmd.exe"));
        }
    }

    #[test]
    fn has_tool_returns_false_for_nonexistent() {
        assert!(!has_tool("this-command-definitely-does-not-exist-98765"));
    }

    #[test]
    fn supports_dnsmasq_never_panics() {
        let _ = supports_dnsmasq();
    }

    #[test]
    fn detect_dns_tools_never_panics() {
        let _ = detect_dns_tools();
    }

    #[test]
    fn detect_services_never_panics() {
        let _ = detect_services();
    }

    // ═════════════════════════════════════════════════════════
    //  format_detect
    // ═════════════════════════════════════════════════════════

    #[test]
    fn format_detect_empty_state() {
        let state = SystemState {
            port_listeners: HashMap::new(),
            services: vec![],
            resolv_conf: String::new(),
            upstream_dns: "127.0.0.1".into(),
            upstream_port: 53,
            dnsdist_snippet: None,
            resolved_conf: None,
            warnings: vec![],
        };
        let out = format_detect(&state);
        assert!(out.contains("Platform:"));
        assert!(out.contains("Port Usage:"));
        assert!(out.contains("none detected"));
        assert!(out.contains("DNS Services:"));
        assert!(out.contains("Upstream:"));
        assert!(out.contains("127.0.0.1:53"));
        assert!(out.contains("dnsmasq:"));
    }

    #[test]
    fn format_detect_with_port_listeners() {
        let mut ports = HashMap::new();
        ports.insert(53, "systemd-resolved".into());
        ports.insert(853, "dnscrypt-proxy".into());
        let state = SystemState {
            port_listeners: ports,
            services: vec![],
            resolv_conf: String::new(),
            upstream_dns: "127.0.0.1".into(),
            upstream_port: 53,
            dnsdist_snippet: None,
            resolved_conf: None,
            warnings: vec![],
        };
        let out = format_detect(&state);
        assert!(out.contains("53"));
        assert!(out.contains("853"));
        assert!(out.contains("systemd-resolved"));
        assert!(out.contains("dnscrypt-proxy"));
    }

    #[test]
    fn format_detect_with_services() {
        let state = SystemState {
            port_listeners: HashMap::new(),
            services: vec!["dnsmasq".into(), "unbound".into()],
            resolv_conf: String::new(),
            upstream_dns: "127.0.0.1".into(),
            upstream_port: 53,
            dnsdist_snippet: None,
            resolved_conf: None,
            warnings: vec![],
        };
        let out = format_detect(&state);
        assert!(out.contains("dnsmasq"));
        assert!(out.contains("unbound"));
    }

    #[test]
    fn format_detect_with_warnings() {
        let state = SystemState {
            port_listeners: HashMap::new(),
            services: vec![],
            resolv_conf: String::new(),
            upstream_dns: "127.0.0.1".into(),
            upstream_port: 53,
            dnsdist_snippet: None,
            resolved_conf: None,
            warnings: vec!["Port 5354 is in use".into()],
        };
        let out = format_detect(&state);
        assert!(out.contains("Warnings:"));
        assert!(out.contains("Port 5354 is in use"));
    }

    #[test]
    fn format_detect_marks_port_5354_with_local_dns() {
        let mut ports = HashMap::new();
        ports.insert(5354, "dnsmasq".into());
        ports.insert(53, "systemd-resolved".into());
        let state = SystemState {
            port_listeners: ports,
            services: vec![],
            resolv_conf: String::new(),
            upstream_dns: "127.0.0.1".into(),
            upstream_port: 53,
            dnsdist_snippet: None,
            resolved_conf: None,
            warnings: vec![],
        };
        let out = format_detect(&state);
        assert!(out.contains("5354"));
        assert!(out.contains("local-dns"));
    }

    #[test]
    fn format_detect_with_everything_populated() {
        let mut ports = HashMap::new();
        ports.insert(53, "systemd-resolved".into());
        ports.insert(5354, "dnsmasq".into());
        let state = SystemState {
            port_listeners: ports,
            services: vec!["dnsmasq".into(), "systemd-resolved".into()],
            resolv_conf: "nameserver 127.0.0.53".into(),
            upstream_dns: "192.168.1.1".into(),
            upstream_port: 53,
            dnsdist_snippet: Some("dnsdist snippet".into()),
            resolved_conf: Some("resolved config".into()),
            warnings: vec!["Port 5354 is in use by 'dnsmasq'.".into()],
        };
        let out = format_detect(&state);
        assert!(out.contains("192.168.1.1:53"));
        assert!(out.contains("dnsmasq"));
        assert!(out.contains("Warnings:"));
        assert!(out.contains("5354"));
    }
}
