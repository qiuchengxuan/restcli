#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

mod cli;
mod config;
mod format;
mod prefix;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    quiet: bool,
    #[clap(short, action = clap::ArgAction::Count)]
    verbosity: u8,
    /// Specify config path
    #[clap(short = 'f', long, default_value = "/etc/restcli/config.yaml")]
    config_path: String,
}

fn load_config(path: &str) -> Result<config::Config, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    serde_yaml::from_reader(file).map_err(|e| e.to_string())
}

fn run(config_path: &str) -> Result<(), String> {
    let config =
        load_config(config_path).map_err(|e| format!("Load config {} fail: {}", config_path, e))?;
    cli::CLI::new(config.url, config.apis).run()
}

fn main() {
    let args = Args::parse();
    let level = match (args.quiet, args.verbosity) {
        (true, _) => log::LevelFilter::Off,
        (_, 0) => log::LevelFilter::Info,
        (_, 1) => log::LevelFilter::Debug,
        (_, _) => log::LevelFilter::Trace,
    };
    log::set_max_level(level);
    env_logger::builder().filter(Some("restcli"), level).target(env_logger::Target::Stdout).init();
    if let Some(err) = run(&args.config_path).err() {
        error!("{}", err);
        std::process::exit(1)
    }
}
