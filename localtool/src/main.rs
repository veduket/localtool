use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = "localtool",
    about = "Local development toolkit — DNS management and HTTPS certificates",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage local DNS entries
    Dns {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Generate local HTTPS certificates
    Ssl {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dns { args } => {
            let mut full = vec!["local-dns".to_string()];
            full.extend(args);
            match local_dns::run(&full) {
                Ok(msg) => {
                    if !msg.is_empty() {
                        println!("{msg}");
                    }
                }
                Err(e) => eprintln!("{}", format!("Error: {e}").red().bold()),
            }
        }
        Commands::Ssl { args } => {
            let mut full = vec!["local-ssl".to_string()];
            full.extend(args);
            match local_ssl::run(&full) {
                Ok(msg) => {
                    if !msg.is_empty() {
                        println!("{msg}");
                    }
                }
                Err(e) => eprintln!("{}", format!("Error: {e}").red().bold()),
            }
        }
    }
}
