use std::io::{self, IsTerminal, Read};
use std::process;

use clap::{Parser, Subcommand};

use infrahub_credential_helper::{InfrahubConfig, fetch_credential};

#[derive(Parser)]
#[command(name = "infrahub-git-credential")]
struct Cli {
    #[arg(long, env = "INFRAHUB_CONFIG", default_value = "infrahub.toml")]
    config_file: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Retrieve credentials for a git repository
    Get {
        /// Input string (key=value pairs); read from stdin if omitted
        input_str: Option<String>,
    },
    /// Store credentials (no-op)
    Store { input_str: Option<String> },
    /// Erase credentials (no-op)
    Erase { input_str: Option<String> },
}

/// Parse the key=value input from the git credential protocol.
///
/// Splits on ALL `=` signs and takes index 1 only.
/// For `path=foo=bar`, this returns `"foo"` (not `"foo=bar"`).
fn parse_helper_get_input(text: &str) -> Result<String, String> {
    let mut protocol = None;
    let mut host = None;
    let mut path = None;

    for line in text.lines() {
        if !line.contains('=') {
            continue;
        }
        let key = line.split('=').next().unwrap_or("");
        let value = line.split('=').nth(1).unwrap_or("");
        match key {
            "protocol" => protocol = Some(value),
            "host" => host = Some(value),
            "path" => path = Some(value),
            _ => {}
        }
    }

    if protocol.is_none() || host.is_none() {
        return Err("Input format not supported.".to_string());
    }

    if path.is_none() {
        return Err(
            "Git usehttppath must be enabled to use this helper. You can active it with\n    git config --global credential.usehttppath true\n    ".to_string(),
        );
    }

    Ok(format!(
        "{}://{}/{}",
        protocol.unwrap(),
        host.unwrap(),
        path.unwrap()
    ))
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Store { .. } | Commands::Erase { .. } => process::exit(0),
        Commands::Get { input_str } => {
            let input = match input_str {
                Some(s) => s,
                None => {
                    if io::stdin().is_terminal() {
                        println!("No input provided.");
                        process::exit(1);
                    }
                    let mut buf = String::new();
                    io::stdin().read_to_string(&mut buf).unwrap_or_default();
                    buf.trim().to_string()
                }
            };

            let location = match parse_helper_get_input(&input) {
                Ok(loc) => loc,
                Err(msg) => {
                    println!("{msg}");
                    process::exit(1);
                }
            };

            let config = match InfrahubConfig::load(Some(&cli.config_file)) {
                Ok(c) => c,
                Err(msg) => {
                    println!("{msg}");
                    process::exit(1);
                }
            };

            match fetch_credential(&config, &location) {
                Ok((username, password)) => {
                    println!("username={username}");
                    println!("password={password}");
                }
                Err(msg) => {
                    println!("{msg}");
                    process::exit(1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_input() {
        let input = "protocol=https\nhost=github.com\npath=opsmill/repo.git";
        assert_eq!(
            parse_helper_get_input(input).unwrap(),
            "https://github.com/opsmill/repo.git"
        );
    }

    #[test]
    fn parse_missing_path() {
        let input = "protocol=https\nhost=github.com";
        let err = parse_helper_get_input(input).unwrap_err();
        assert!(err.contains("usehttppath"));
    }

    #[test]
    fn parse_missing_protocol() {
        let input = "host=github.com\npath=repo.git";
        let err = parse_helper_get_input(input).unwrap_err();
        assert_eq!(err, "Input format not supported.");
    }

    #[test]
    fn parse_missing_host() {
        let input = "protocol=https\npath=repo.git";
        let err = parse_helper_get_input(input).unwrap_err();
        assert_eq!(err, "Input format not supported.");
    }

    #[test]
    fn parse_equals_in_value_takes_index_1() {
        let input = "protocol=https\nhost=example.com\npath=foo=bar";
        let url = parse_helper_get_input(input).unwrap();
        assert_eq!(url, "https://example.com/foo");
    }

    #[test]
    fn parse_skips_lines_without_equals() {
        let input = "protocol=https\ngarbage\nhost=github.com\n\npath=repo.git";
        assert_eq!(
            parse_helper_get_input(input).unwrap(),
            "https://github.com/repo.git"
        );
    }

    #[test]
    fn parse_empty_input() {
        let err = parse_helper_get_input("").unwrap_err();
        assert_eq!(err, "Input format not supported.");
    }
}
