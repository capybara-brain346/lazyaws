#![forbid(unsafe_code)]

use lazyaws::config::{Cli, Config, USAGE};

fn main() -> std::process::ExitCode {
    let result = match Config::parse(std::env::args().skip(1)) {
        Ok(Cli::Help) => {
            println!("{USAGE}");
            return std::process::ExitCode::SUCCESS;
        }
        Ok(Cli::Version) => {
            println!("lazyaws {}", env!("CARGO_PKG_VERSION"));
            return std::process::ExitCode::SUCCESS;
        }
        Ok(Cli::Run(config)) => lazyaws::runtime::run(config).map_err(|error| error.to_string()),
        Err(error) => Err(format!("{error}\n\n{USAGE}")),
    };

    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lazyaws: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
