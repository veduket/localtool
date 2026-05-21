use clap::{Parser, Subcommand};
use colored::Colorize;

mod ca;
mod cert;
mod telemetry;
mod trust;
mod util;

#[derive(Parser)]
#[command(
    name = "local-ssl",
    about = "Local HTTPS certificates for development — trust locally, never prod",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a local Certificate Authority and install system trust
    Init,
    /// Generate an HTTPS certificate for a development domain
    Generate {
        /// Domain(s) to generate cert for (first is primary, rest are SANs)
        domains: Vec<String>,
        /// Output directory (default: /etc/local-ssl/certs/<domain>/)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// List all generated certificates
    List,
    /// Show certificate details
    Show {
        /// Domain to inspect
        domain: String,
    },
    /// (Re)install CA into system trust store
    Trust,
    /// Show CA status and expiry
    Status,
    /// Check certificate validity for a domain (or "all")
    Check {
        /// Domain to check, or "all" for all certificates
        domain: String,
    },
    /// Manage anonymous usage telemetry
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

pub fn run(args: &[String]) -> Result<String, String> {
    let store = ca::CaStore::new();
    let tel = telemetry::Telemetry::load(&store.dir);
    let cli = Cli::parse_from(args);
    let is_telemetry_cmd = matches!(&cli.command, Commands::Telemetry { .. });
    let result = execute(cli, &tel);
    if !is_telemetry_cmd && tel.maybe_heartbeat() {
        tel.show_heartbeat_notice();
    }
    result
}

fn execute(cli: Cli, tel: &telemetry::Telemetry) -> Result<String, String> {
    let store = ca::CaStore::new();

    let cmd_name = match &cli.command {
        Commands::Telemetry { .. } => "telemetry",
        Commands::Init => "init",
        Commands::Generate { .. } => "generate",
        Commands::List => "list",
        Commands::Show { .. } => "show",
        Commands::Check { .. } => "check",
        Commands::Trust => "trust",
        Commands::Status => "status",
    };
    let stats = if matches!(cmd_name, "init" | "status" | "telemetry") {
        vec![]
    } else {
        collect_tool_stats(&store)
    };
    let stats_refs: Vec<(&str, &str)> = stats.iter().map(|(k, v)| (*k, v.as_str())).collect();
    tel.send_command_event(cmd_name, &stats_refs);

    match cli.command {
        Commands::Telemetry { action } => handle_telemetry(action, &store),
        Commands::Init => cmd_init(&store),
        Commands::Generate { domains, output } => cmd_generate(&store, &domains, output),
        Commands::List => cmd_list(&store),
        Commands::Show { domain } => cmd_show(&store, &domain),
        Commands::Check { domain } => cmd_check(&store, &domain),
        Commands::Trust => cmd_trust(&store),
        Commands::Status => cmd_status(&store),
    }
}

fn handle_telemetry(action: TelemetryAction, store: &ca::CaStore) -> Result<String, String> {
    match action {
        TelemetryAction::Enable => {
            let mut t = telemetry::Telemetry::load(&store.dir);
            t.enable()?;
            Ok(format!(
                "{} Anonymous telemetry enabled\n{}",
                "✓".green(),
                t.status()
            ))
        }
        TelemetryAction::Disable => {
            let mut t = telemetry::Telemetry::load(&store.dir);
            t.disable()?;
            Ok(format!(
                "{} Anonymous telemetry disabled\n{}",
                "✓".yellow(),
                t.status()
            ))
        }
        TelemetryAction::Status => {
            let t = telemetry::Telemetry::load(&store.dir);
            Ok(t.status())
        }
    }
}

fn cmd_init(store: &ca::CaStore) -> Result<String, String> {
    if store.exists() {
        println!(
            "{}",
            "CA already exists. Run `local-ssl trust` to reinstall trust.".yellow()
        );
        return Ok(String::new());
    }

    println!(
        "{}",
        "Generating local Certificate Authority...".cyan().bold()
    );
    store.init()?;
    println!(
        "{} CA key:  {}",
        "✓".green(),
        store.key_path.display().to_string().cyan()
    );
    println!(
        "{} CA cert: {}",
        "✓".green(),
        store.cert_path.display().to_string().cyan()
    );

    println!(
        "\n{}",
        "Installing CA into system trust store...".cyan().bold()
    );
    trust::install_ca(&store.cert_path)?;
    println!("{} CA trusted system-wide", "✓".green());

    println!(
        "\n{}",
        "Ready. Generate certs for local development with:".bold()
    );
    println!("  {}", "local-ssl generate myapp.test".green());
    println!(
        "  {}",
        "local-ssl generate api.test www.test --trust".green()
    );

    Ok(String::new())
}

fn cmd_generate(
    store: &ca::CaStore,
    domains: &[String],
    _output: Option<String>,
) -> Result<String, String> {
    if !store.exists() {
        return Err("CA not initialized. Run `local-ssl init` first.".into());
    }

    if domains.is_empty() {
        return Err("At least one domain is required.".into());
    }

    let primary = &domains[0];
    let sans: Vec<String> = domains[1..].to_vec();

    let bundle = cert::generate(primary, store, &sans)?;
    println!("{} Certificate for '{}'", "✓".green(), primary.bold());
    println!("  {} {}", "Cert:".bold(), bundle.cert_path.cyan());
    println!("  {} {}", "Key:".bold(), bundle.key_path.cyan());
    println!();
    println!("{}", "Example local HTTPS usage:".bold());
    println!(
        "  {} curl --cacert /etc/local-ssl/ca-cert.pem https://{}/",
        "Test:".dimmed(),
        primary
    );
    println!(
        "  {} node server.js --key {} --cert {}",
        "Node:".dimmed(),
        bundle.key_path,
        bundle.cert_path
    );
    println!("  {} local-dns add {} 127.0.0.1", "DNS:".dimmed(), primary);
    println!();
    println!(
        "{}",
        "⚠ This cert is for LOCAL DEVELOPMENT ONLY. Never use it in production."
            .yellow()
            .bold()
    );

    Ok(String::new())
}

fn cmd_list(store: &ca::CaStore) -> Result<String, String> {
    let domains = cert::list(store)?;
    if domains.is_empty() {
        return Ok(format!("{}", "No certificates generated yet.".yellow()));
    }
    let mut out = format!("{}\n", "Generated certificates:".bold());
    for d in &domains {
        out.push_str(&format!("  {}\n", d.green()));
    }
    out.push_str(&format!(
        "\nLocation: {}",
        store.dir.join("certs").display().to_string().cyan()
    ));
    Ok(out)
}

fn cmd_check(store: &ca::CaStore, domain: &str) -> Result<String, String> {
    if domain == "all" {
        return cert::check_all_local(store);
    }
    // Try remote check if domain contains a colon (host:port)
    if let Some((host, port_str)) = domain.split_once(':') {
        if let Ok(port) = port_str.parse::<u16>() {
            return cert::check_remote(host, port);
        }
    }
    // Default: local check
    cert::check_local(domain, store)
}

fn cmd_show(store: &ca::CaStore, domain: &str) -> Result<String, String> {
    cert::show(domain, store)
}

fn cmd_trust(store: &ca::CaStore) -> Result<String, String> {
    if !store.exists() {
        return Err("CA not initialized. Run `local-ssl init` first.".into());
    }
    if trust::is_ca_trusted(&store.cert_path) {
        return Ok(format!("{} CA is already trusted.", "✓".green()));
    }
    println!("{}", "Installing CA into system trust store...".cyan());
    trust::install_ca(&store.cert_path)?;
    Ok(format!("{} CA trusted system-wide", "✓".green()))
}

fn cmd_status(store: &ca::CaStore) -> Result<String, String> {
    if !store.exists() {
        return Ok(format!(
            "{}",
            "CA not initialized. Run `local-ssl init` first.".yellow()
        ));
    }

    let trusted = trust::is_ca_trusted(&store.cert_path);
    let trust_status: String = if trusted {
        "trusted ✓".green().to_string()
    } else {
        "not trusted".red().to_string()
    };

    println!("{}", "CA Status:".bold());
    println!("{}", store.status()?);
    println!("{} {}", "System trust:".bold(), trust_status);

    let count = cert::list(store)?.len();
    println!("{} {}", "Certificates:".bold(), count.to_string().cyan());

    Ok(String::new())
}

fn collect_tool_stats(store: &ca::CaStore) -> Vec<(&'static str, String)> {
    let mut stats: Vec<(&'static str, String)> = Vec::new();
    if let Ok(domains) = cert::list(store) {
        stats.push(("certs", domains.len().to_string()));
    }
    stats
}
