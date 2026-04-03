use std::io::{self, Read};
use std::process;

use clap::Parser;
use regex::Regex;

use infrahub_credential_helper::{InfrahubConfig, fetch_credential};

#[derive(Parser)]
#[command(name = "infrahub-git-askpass")]
struct Cli {
    #[arg(long, env = "INFRAHUB_CONFIG", default_value = "infrahub.toml")]
    config_file: String,

    /// Prompt text (e.g., "Username for 'https://...':")
    text: Vec<String>,
}

fn main() {
    let cli = Cli::parse();

    let text = if cli.text.is_empty() {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).unwrap_or_default();
        buf.trim().to_string()
    } else {
        cli.text.join(" ")
    };

    let re_username = Regex::new(r"^Username.*'(.*)'").unwrap();
    let re_password = Regex::new(r"^Password.*'(.*://)(.*)@(.*)'").unwrap();

    let (location, request_type);

    if let Some(caps) = re_username.captures(&text) {
        location = caps.get(1).unwrap().as_str().to_string();
        request_type = "username";
    } else if let Some(caps) = re_password.captures(&text) {
        let scheme = caps.get(1).unwrap().as_str();
        let host_path = caps.get(3).unwrap().as_str();
        location = format!("{scheme}{host_path}");
        request_type = "password";
    } else {
        eprintln!("Unable to identify the request type in '{text}'");
        process::exit(1);
    }

    let config = match InfrahubConfig::load(Some(&cli.config_file)) {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("{msg}");
            process::exit(1);
        }
    };

    match fetch_credential(&config, &location) {
        Ok((username, password)) => {
            let value = match request_type {
                "username" => username,
                "password" => password,
                _ => unreachable!(),
            };
            println!("{value}");
        }
        Err(msg) => {
            eprintln!("{msg}");
            process::exit(1);
        }
    }
}
