use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const TELEMETRY_ENV_DISABLE: &str = "LOCAL_DNS_TELEMETRY_DISABLE";
const TELEMETRY_URL: &str = "https://localtool.vercel.app/api/telemetry";
const PUBLISH_URL: &str = "https://localtool.vercel.app";

pub struct Telemetry {
    pub uuid: String,
    pub enabled: bool,
    pub first_seen: u64,
    pub last_ping: u64,
    pub usage_days: u64,
    config_path: PathBuf,
}

impl Telemetry {
    pub fn load(data_dir: &PathBuf) -> Self {
        let config_path = data_dir.join("telemetry.json");
        let config_dir = config_path.parent().unwrap();
        let env_disabled = std::env::var(TELEMETRY_ENV_DISABLE).is_ok()
            || std::env::var("DO_NOT_TRACK").is_ok();
        let now = now_secs();

        if let Ok(json) = fs::read_to_string(&config_path) {
            let uuid = Self::parse_uuid(&json).unwrap_or_else(Self::generate_uuid);
            let enabled = !env_disabled && Self::parse_enabled(&json).unwrap_or(true);
            let first_seen = Self::parse_first_seen(&json).unwrap_or(now);
            let last_ping = Self::parse_last_ping(&json).unwrap_or(0);
            let mut t = Telemetry { uuid, enabled, first_seen, last_ping, usage_days: 0, config_path };
            t.usage_days = t.compute_usage_days();
            t
        } else {
            let uuid = Self::generate_uuid();
            let enabled = !env_disabled;
            let config = TelemetryConfig { uuid: uuid.clone(), enabled, first_seen: now, last_ping: now };
            if fs::create_dir_all(config_dir).is_ok() {
                config.write(&config_path).ok();
            }
            let usage_days = 0;
            Telemetry { uuid, enabled, first_seen: now, last_ping: now, usage_days, config_path }
        }
    }

    /// Fire a heartbeat if 24h has passed since last ping.
    /// Returns true if a heartbeat was sent (so callers can show a thank-you).
    pub fn maybe_heartbeat(&self) -> bool {
        if !self.enabled || TELEMETRY_URL.is_empty() { return false; }
        let now = now_secs();
        if now < self.last_ping + 86400 { return false; }

        let payload = format!(
            "uuid={}&tool=local-dns&version={}&os={}&event=heartbeat&age_days={}&ts={}",
            self.uuid, env!("CARGO_PKG_VERSION"), std::env::consts::OS, self.usage_days, now
        );
        let ok = Command::new("curl")
            .args(["-s", "-o", "/dev/null", "--max-time", "3"])
            .args(["-X", "POST", TELEMETRY_URL, "-d", &payload])
            .status().map(|s| s.success()).unwrap_or(false);

        if ok {
            // Persist updated last_ping
            let config = TelemetryConfig {
                uuid: self.uuid.clone(),
                enabled: self.enabled,
                first_seen: self.first_seen,
                last_ping: now,
            };
            config.write(&self.config_path).ok();
        }

        ok
    }

    /// Fire a single event (on command invocation).
    pub fn send_command_event(&self, command: &str, extra: &[(&str, &str)]) {
        if !self.enabled || TELEMETRY_URL.is_empty() { return; }
        let now = now_secs();
        let mut parts = vec![
            format!("uuid={}", self.uuid),
            format!("tool=local-dns"),
            format!("version={}", env!("CARGO_PKG_VERSION")),
            format!("os={}", std::env::consts::OS),
            format!("event=command"),
            format!("command={command}"),
            format!("age_days={}", self.usage_days),
            format!("ts={now}"),
        ];
        for (k, v) in extra { parts.push(format!("{k}={v}")); }
        let payload = parts.join("&");
        Command::new("curl")
            .args(["-s", "-o", "/dev/null", "--max-time", "3", "-X", "POST", TELEMETRY_URL, "-d", &payload])
            .spawn().ok();
    }

    pub fn enable(&mut self) -> Result<(), String> {
        self.enabled = true; self.save()
    }

    pub fn disable(&mut self) -> Result<(), String> {
        self.enabled = false; self.save()
    }

    pub fn status(&self) -> String {
        let status = if self.enabled { "enabled ✓".green() } else { "disabled".yellow() };
        let days = if self.usage_days == 0 { "first day!".green() } else { format!("{} days", self.usage_days).cyan() };
        format!(
            "Anonymous telemetry: {status}\n\
             UUID:                   {}\n\
             Using since:            {}\n\
             Tool age:               {days}\n\
             Last heartbeat:         {}\n\
             \n\
             To change:\n  local-dns telemetry enable\n  local-dns telemetry disable\n  local-dns telemetry status\n\
             \n\
             Publish URL: {PUBLISH_URL}",
            self.uuid,
            format_ts(self.first_seen),
            format_ts(self.last_ping),
        )
    }

    pub fn show_heartbeat_notice(&self) {
        if self.enabled && !TELEMETRY_URL.is_empty() {
            let days = if self.usage_days > 0 { format!("{} days ", self.usage_days) } else { String::new() };
            println!();
            println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
            println!(
                "{} Thank you for using local-dns! ({})",
                "❤".green(),
                format!("{days}and counting").cyan()
            );
            println!("{} {PUBLISH_URL}", "Discover more tools:".dimmed());
            println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".dimmed());
        } else if !self.enabled && TELEMETRY_URL.is_empty() {
            println!("{}", "ℹ Telemetry: set LOCAL_DNS_TELEMETRY_URL to enable anonymous usage reporting.".dimmed());
        }
    }

    fn save(&self) -> Result<(), String> {
        let config = TelemetryConfig {
            uuid: self.uuid.clone(),
            enabled: self.enabled,
            first_seen: self.first_seen,
            last_ping: self.last_ping,
        };
        config.write(&self.config_path)
    }

    fn compute_usage_days(&self) -> u64 {
        let now = now_secs();
        if now > self.first_seen { (now - self.first_seen) / 86400 } else { 0 }
    }

    fn generate_uuid() -> String {
        format!("t{:x}-p{:x}", now_secs(), std::process::id())
    }

    fn parse_uuid(json: &str) -> Option<String> {
        let s = json.find("\"uuid\":\"")?;
        let start = s + 8;
        let end = json[start..].find('"')?;
        Some(json[start..start + end].to_string())
    }

    fn parse_enabled(json: &str) -> Option<bool> {
        let s = json.find("\"enabled\":")?;
        let val = json[s + 9..].trim_start();
        if val.starts_with("true") { Some(true) }
        else if val.starts_with("false") { Some(false) }
        else { None }
    }

    fn parse_first_seen(json: &str) -> Option<u64> {
        let s = json.find("\"first_seen\":")?;
        let rest = json[s + 12..].trim_start();
        let end = rest.find(|c: char| !c.is_ascii_digit())?;
        rest[..end].parse().ok()
    }

    fn parse_last_ping(json: &str) -> Option<u64> {
        let s = json.find("\"last_ping\":")?;
        let rest = json[s + 11..].trim_start();
        let end = rest.find(|c: char| !c.is_ascii_digit())?;
        rest[..end].parse().ok()
    }
}

struct TelemetryConfig {
    uuid: String,
    enabled: bool,
    first_seen: u64,
    last_ping: u64,
}

impl TelemetryConfig {
    fn write(&self, path: &PathBuf) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("{e}"))?;
        }
        let json = format!(
            r#"{{"uuid":"{}","enabled":{},"first_seen":{},"last_ping":{}}}"#,
            self.uuid, self.enabled, self.first_seen, self.last_ping
        );
        fs::write(path, &json).map_err(|e| format!("Cannot save telemetry config: {e}"))
    }
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn format_ts(ts: u64) -> String {
    let days = ts / 86400;
    let y = 1970_f64 + days as f64 / 365.25;
    format!("epoch day {days} (~{:.0})", y)
}
