use std::env;
use std::fs;
use std::time::Duration;

use graphql_client::GraphQLQuery;
use serde::Deserialize;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schema/schema.graphql",
    query_path = "src/graphql/get_repo_credential.graphql",
    response_derives = "Debug"
)]
struct GetRepoCredential;

use get_repo_credential::GetRepoCredentialCoreGenericRepositoryEdgesNodeCredentialNodeOn as CredentialOn;

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

/// Fetch credentials for a git repository from the Infrahub GraphQL API.
pub fn fetch_credential(
    config: &InfrahubConfig,
    location: &str,
) -> Result<(String, String), String> {
    let agent = build_agent();
    let url = format!("{}/graphql/main", config.address.trim_end_matches('/'));

    let variables = get_repo_credential::Variables {
        location: location.to_string(),
    };
    let request_body = GetRepoCredential::build_query(variables);

    let mut req = agent.post(&url).header("Content-Type", "application/json");
    if let Some(token) = &config.api_token {
        req = req.header("X-INFRAHUB-KEY", token);
    }

    let mut resp = req.send_json(&request_body).map_err(|e| format!("{e}"))?;
    let response_body: graphql_client::Response<get_repo_credential::ResponseData> =
        resp.body_mut().read_json().map_err(|e| format!("{e}"))?;

    extract_credential(response_body)
}

fn extract_credential(
    response: graphql_client::Response<get_repo_credential::ResponseData>,
) -> Result<(String, String), String> {
    let data = response.data.ok_or("No data in API response")?;

    let edges = &data.core_generic_repository.edges;
    if edges.is_empty() {
        return Err("Repository not found in the database.".to_string());
    }

    let repo_node = edges[0]
        .node
        .as_ref()
        .ok_or("Repository not found in the database.")?;
    let cred_node = repo_node
        .credential
        .node
        .as_ref()
        .ok_or("Repository doesn't have credentials defined.")?;

    match cred_node.on {
        CredentialOn::CorePasswordCredential(ref cred) => {
            let username = cred
                .username
                .as_ref()
                .and_then(|u| u.value.clone())
                .ok_or("Failed to extract username from credentials.")?;
            let password = cred
                .password
                .as_ref()
                .and_then(|p| p.value.clone())
                .ok_or("Failed to extract password from credentials.")?;
            Ok((username, password))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn mock_response(json: &str) -> graphql_client::Response<get_repo_credential::ResponseData> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn extract_credential_success() {
        let resp = mock_response(
            r#"{
                "data": {
                    "CoreGenericRepository": {
                        "edges": [{
                            "node": {
                                "__typename": "CoreRepository",
                                "id": "repo-1",
                                "credential": {
                                    "node": {
                                        "__typename": "CorePasswordCredential",
                                        "id": "cred-1",
                                        "username": { "value": "myuser" },
                                        "password": { "value": "mypass" }
                                    }
                                }
                            }
                        }]
                    }
                }
            }"#,
        );
        let (username, password) = extract_credential(resp).unwrap();
        assert_eq!(username, "myuser");
        assert_eq!(password, "mypass");
    }

    #[test]
    fn extract_credential_empty_edges() {
        let resp = mock_response(
            r#"{
                "data": {
                    "CoreGenericRepository": {
                        "edges": []
                    }
                }
            }"#,
        );
        let err = extract_credential(resp).unwrap_err();
        assert_eq!(err, "Repository not found in the database.");
    }

    #[test]
    fn extract_credential_no_credential_node() {
        let resp = mock_response(
            r#"{
                "data": {
                    "CoreGenericRepository": {
                        "edges": [{
                            "node": {
                                "__typename": "CoreRepository",
                                "id": "repo-1",
                                "credential": {
                                    "node": null
                                }
                            }
                        }]
                    }
                }
            }"#,
        );
        let err = extract_credential(resp).unwrap_err();
        assert_eq!(err, "Repository doesn't have credentials defined.");
    }

    #[test]
    fn extract_credential_missing_username() {
        let resp = mock_response(
            r#"{
                "data": {
                    "CoreGenericRepository": {
                        "edges": [{
                            "node": {
                                "__typename": "CoreRepository",
                                "id": "repo-1",
                                "credential": {
                                    "node": {
                                        "__typename": "CorePasswordCredential",
                                        "id": "cred-1",
                                        "username": null,
                                        "password": { "value": "mypass" }
                                    }
                                }
                            }
                        }]
                    }
                }
            }"#,
        );
        let err = extract_credential(resp).unwrap_err();
        assert_eq!(err, "Failed to extract username from credentials.");
    }

    #[test]
    fn extract_credential_no_data() {
        let resp: graphql_client::Response<get_repo_credential::ResponseData> =
            graphql_client::Response {
                data: None,
                errors: None,
                extensions: None,
            };
        let err = extract_credential(resp).unwrap_err();
        assert_eq!(err, "No data in API response");
    }
}
