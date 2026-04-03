use std::env;
use std::fs;
use std::time::Duration;

use serde::Deserialize;

#[derive(Deserialize)]
struct TomlConfig {
    main: Option<MainSection>,
}

#[derive(Deserialize)]
struct MainSection {
    internal_address: Option<String>,
}

pub struct InfrahubConfig {
    pub address: String,
    pub api_token: Option<String>,
}

impl InfrahubConfig {
    /// Load configuration with priority: env vars > TOML file.
    ///
    /// Address: INFRAHUB_INTERNAL_ADDRESS env > `[main].internal_address` from TOML.
    /// Config file path: `--config-file` arg > INFRAHUB_CONFIG env > `infrahub.toml`.
    pub fn load(config_file_override: Option<&str>) -> Result<Self, String> {
        let api_token = env::var("INFRAHUB_API_TOKEN").ok();

        if let Ok(address) = env::var("INFRAHUB_INTERNAL_ADDRESS") {
            return Ok(Self { address, api_token });
        }

        let config_path = config_file_override
            .map(String::from)
            .or_else(|| env::var("INFRAHUB_CONFIG").ok())
            .unwrap_or_else(|| "infrahub.toml".to_string());

        if let Ok(content) = fs::read_to_string(&config_path) {
            let toml_config: TomlConfig = toml::from_str(&content)
                .map_err(|e| format!("Failed to parse config file: {e}"))?;

            if let Some(main) = toml_config.main
                && let Some(address) = main.internal_address
            {
                return Ok(Self { address, api_token });
            }
        }

        Err(
            "No Infrahub server address configured. Set INFRAHUB_INTERNAL_ADDRESS or configure [main].internal_address in config file.".to_string(),
        )
    }
}

fn build_agent() -> ureq::Agent {
    ureq::config::Config::builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .new_agent()
}

fn post_graphql(
    agent: &ureq::Agent,
    url: &str,
    query: &str,
    api_token: Option<&str>,
) -> Result<serde_json::Value, String> {
    let body = serde_json::json!({ "query": query });

    let mut req = agent.post(url).header("Content-Type", "application/json");

    if let Some(token) = api_token {
        req = req.header("X-INFRAHUB-KEY", token);
    }

    let mut resp = req.send_json(&body).map_err(|e| format!("{e}"))?;
    resp.body_mut()
        .read_json::<serde_json::Value>()
        .map_err(|e| format!("{e}"))
}

fn escape_graphql_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                for unit in c.encode_utf16(&mut [0; 2]) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
            c => out.push(c),
        }
    }
    out
}

fn is_valid_graphql_type_name(s: &str) -> bool {
    !s.is_empty()
        && s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Fetch credentials for a git repository from the Infrahub GraphQL API.
pub fn fetch_credential(
    config: &InfrahubConfig,
    location: &str,
) -> Result<(String, String), String> {
    let agent = build_agent();
    let url = format!("{}/graphql/main", config.address.trim_end_matches('/'));

    let query1 = format!(
        r#"query {{ CoreGenericRepository(location__value: "{}") {{ edges {{ node {{ id display_label credential {{ node {{ id display_label __typename }} }} }} }} }} }}"#,
        escape_graphql_string(location)
    );

    let data = post_graphql(&agent, &url, &query1, config.api_token.as_deref())?;

    let edges = data["data"]["CoreGenericRepository"]["edges"]
        .as_array()
        .ok_or_else(|| "Repository not found in the database.".to_string())?;

    if edges.is_empty() {
        return Err("Repository not found in the database.".to_string());
    }

    let cred_node = &edges[0]["node"]["credential"]["node"];

    if cred_node.is_null() {
        return Err("Repository doesn't have credentials defined.".to_string());
    }

    let cred_id = cred_node["id"]
        .as_str()
        .ok_or_else(|| "Repository doesn't have credentials defined.".to_string())?;
    let cred_typename = cred_node["__typename"]
        .as_str()
        .ok_or_else(|| "Repository doesn't have credentials defined.".to_string())?;

    if !is_valid_graphql_type_name(cred_typename) {
        return Err("Invalid credential type received from API.".to_string());
    }
    let query2 = format!(
        r#"query {{ {}(ids: ["{}"]) {{ edges {{ node {{ id username {{ value }} password {{ value }} }} }} }} }}"#,
        cred_typename,
        escape_graphql_string(cred_id)
    );

    let data2 = post_graphql(&agent, &url, &query2, config.api_token.as_deref())?;

    let edges2 = data2["data"][cred_typename]["edges"]
        .as_array()
        .ok_or_else(|| "Failed to fetch credentials.".to_string())?;

    if edges2.is_empty() {
        return Err("Failed to fetch credentials.".to_string());
    }

    let cred = &edges2[0]["node"];
    let username = cred["username"]["value"]
        .as_str()
        .ok_or_else(|| "Failed to extract credentials.".to_string())?
        .to_string();
    let password = cred["password"]["value"]
        .as_str()
        .ok_or_else(|| "Failed to extract credentials.".to_string())?
        .to_string();

    Ok((username, password))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_graphql_quotes_and_backslashes() {
        assert_eq!(escape_graphql_string(r#"a"b\c"#), r#"a\"b\\c"#);
    }

    #[test]
    fn escape_graphql_newlines_and_tabs() {
        assert_eq!(escape_graphql_string("a\nb\tc\r"), "a\\nb\\tc\\r");
    }

    #[test]
    fn escape_graphql_control_characters() {
        assert_eq!(escape_graphql_string("a\x00b\x1fc"), "a\\u0000b\\u001fc");
    }

    #[test]
    fn escape_graphql_injection_attempt() {
        let malicious = r#"repo") { injected }"#;
        let escaped = escape_graphql_string(malicious);
        assert_eq!(escaped, r#"repo\") { injected }"#);
    }

    #[test]
    fn valid_graphql_type_names() {
        assert!(is_valid_graphql_type_name("CorePasswordCredential"));
        assert!(is_valid_graphql_type_name("_Private"));
    }

    #[test]
    fn invalid_graphql_type_names_reject_injection() {
        assert!(!is_valid_graphql_type_name(""));
        assert!(!is_valid_graphql_type_name("1StartsWithDigit"));
        assert!(!is_valid_graphql_type_name("has-dash"));
        assert!(!is_valid_graphql_type_name("has space"));
        // Injection attempts
        assert!(!is_valid_graphql_type_name("Type{evil}"));
        assert!(!is_valid_graphql_type_name("Type(ids:[])"));
    }

    // SAFETY: `env::set_var`/`remove_var` are unsafe because concurrent access to
    // env vars is undefined behavior. These tests run single-threaded (`--test-threads=1`
    // in Makefile) so no concurrent access occurs.

    unsafe fn set_env(key: &str, val: &str) {
        unsafe { env::set_var(key, val) };
    }

    unsafe fn remove_env(key: &str) {
        unsafe { env::remove_var(key) };
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("infrahub-test-{name}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn config_env_var_overrides_everything() {
        let dir = temp_dir("env-override");
        let path = dir.join("test.toml");
        fs::write(&path, "[main]\ninternal_address = \"http://toml:9000\"\n").unwrap();

        unsafe { set_env("INFRAHUB_INTERNAL_ADDRESS", "http://env:8000") };
        let config = InfrahubConfig::load(Some(path.to_str().unwrap())).unwrap();
        assert_eq!(config.address, "http://env:8000");

        unsafe { remove_env("INFRAHUB_INTERNAL_ADDRESS") };
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn config_falls_back_to_toml() {
        unsafe {
            remove_env("INFRAHUB_INTERNAL_ADDRESS");
            remove_env("INFRAHUB_API_TOKEN");
        }

        let dir = temp_dir("toml-fallback");
        let path = dir.join("test.toml");
        fs::write(&path, "[main]\ninternal_address = \"http://toml:9000\"\n").unwrap();

        let config = InfrahubConfig::load(Some(path.to_str().unwrap())).unwrap();
        assert_eq!(config.address, "http://toml:9000");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn config_api_token_from_env() {
        unsafe {
            set_env("INFRAHUB_INTERNAL_ADDRESS", "http://test:8000");
            set_env("INFRAHUB_API_TOKEN", "secret123");
        }
        let config = InfrahubConfig::load(None).unwrap();
        assert_eq!(config.api_token.as_deref(), Some("secret123"));
        unsafe {
            remove_env("INFRAHUB_INTERNAL_ADDRESS");
            remove_env("INFRAHUB_API_TOKEN");
        }
    }

    #[test]
    fn config_errors_when_no_address_available() {
        unsafe { remove_env("INFRAHUB_INTERNAL_ADDRESS") };
        let result = InfrahubConfig::load(Some("/nonexistent/path.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn config_errors_on_toml_without_address() {
        unsafe { remove_env("INFRAHUB_INTERNAL_ADDRESS") };

        let dir = temp_dir("no-address");
        let path = dir.join("test.toml");
        fs::write(&path, "[main]\n").unwrap();

        let result = InfrahubConfig::load(Some(path.to_str().unwrap()));
        assert!(result.is_err());

        fs::remove_dir_all(&dir).ok();
    }
}
