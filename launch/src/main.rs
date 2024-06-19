use clap::Parser;

fn main() {
    time_local::init();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if let Err(error) = launch::cli::Cli::parse().run() {
        const BOLD_RED: &str = "\x1b[1;31m";
        const BOLD: &str = "\x1b[1m";
        const RESET: &str = "\x1b[0m";
        eprintln!("{BOLD_RED}error{RESET}{BOLD}:{RESET} {error}");
        std::process::exit(1);
    }
}
