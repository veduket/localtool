use colored::Colorize;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match local_ssl::run(&args) {
        Ok(msg) => {
            if !msg.is_empty() {
                println!("{msg}");
            }
        }
        Err(e) => eprintln!("{}", format!("Error: {e}").red().bold()),
    }
}
