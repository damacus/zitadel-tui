use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub zitadel_url: Option<String>,
    pub project_id: Option<String>,
    pub apps_config_file: Option<PathBuf>,
    pub pat: Option<String>,
    pub service_account_file: Option<PathBuf>,
    pub oauth_refresh_token: Option<String>,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("zitadel_url", &self.zitadel_url)
            .field("project_id", &self.project_id)
            .field("apps_config_file", &self.apps_config_file)
            .field("pat", &self.pat.as_ref().map(|_| "[REDACTED]"))
            .field("service_account_file", &self.service_account_file)
            .field(
                "oauth_refresh_token",
                &self.oauth_refresh_token.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct TemplatesFile {
    pub apps: BTreeMap<String, AppTemplate>,
    pub users: Vec<UserTemplate>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct AppTemplate {
    pub redirect_uris: Vec<String>,
    pub public: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct UserTemplate {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub admin: bool,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        for path in config_paths() {
            if path.exists() {
                let contents = fs::read_to_string(&path)
                    .with_context(|| format!("failed to read config {}", path.display()))?;
                let config: AppConfig = toml::from_str(&contents)
                    .with_context(|| format!("failed to parse config {}", path.display()))?;
                return Ok(config);
            }
        }

        Ok(Self::default())
    }

    pub fn save_to_canonical_path(&self) -> Result<PathBuf> {
        let path = Self::canonical_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(path)
    }

    pub fn canonical_path() -> Result<PathBuf> {
        dirs::config_dir()
            .map(|dir| dir.join("zitadel-tui").join("config.toml"))
            .context("could not determine config directory")
    }

    pub fn templates(&self) -> Result<TemplatesFile> {
        let Some(path) = &self.apps_config_file else {
            return Ok(TemplatesFile::default());
        };

        if !path.exists() {
            return Ok(TemplatesFile::default());
        }

        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read templates file {}", path.display()))?;
        let templates = serde_yaml::from_str::<TemplatesFile>(&contents)
            .with_context(|| format!("failed to parse templates file {}", path.display()))?;
        Ok(templates)
    }

    #[cfg(test)]
    pub fn load_from_str(toml_str: &str) -> Result<Self> {
        Ok(toml::from_str(toml_str)?)
    }
}

fn config_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("config.toml"));
    }

    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join("zitadel-tui").join("config.toml"));
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn defaults_are_empty_and_safe() {
        let config = AppConfig::default();
        assert!(config.zitadel_url.is_none());
        assert!(config.project_id.is_none());
        assert!(config.apps_config_file.is_none());
        assert!(config.pat.is_none());
        assert!(config.service_account_file.is_none());
    }

    #[test]
    fn load_from_full_toml() {
        let config = AppConfig::load_from_str(
            r#"
                zitadel_url = "https://zitadel.example.com"
                project_id = "123"
                apps_config_file = "/tmp/apps.yml"
                pat = "token"
                service_account_file = "/tmp/sa.json"
            "#,
        )
        .unwrap();

        assert_eq!(
            config.zitadel_url.as_deref(),
            Some("https://zitadel.example.com")
        );
        assert_eq!(config.project_id.as_deref(), Some("123"));
        assert_eq!(
            config.apps_config_file.as_deref(),
            Some(Path::new("/tmp/apps.yml"))
        );
        assert_eq!(config.pat.as_deref(), Some("token"));
        assert_eq!(
            config.service_account_file.as_deref(),
            Some(Path::new("/tmp/sa.json"))
        );
    }

    #[test]
    fn invalid_toml_is_rejected() {
        assert!(AppConfig::load_from_str("zitadel_url = [").is_err());
    }

    #[test]
    fn parses_templates_file() {
        let templates: TemplatesFile = serde_yaml::from_str(
            r#"
apps:
  grafana:
    redirect_uris:
      - https://grafana.example.com/oauth2/callback
    public: false
users:
  - email: admin@example.com
    first_name: Admin
    last_name: User
    admin: true
"#,
        )
        .unwrap();

        assert_eq!(templates.apps["grafana"].redirect_uris.len(), 1);
        assert!(!templates.apps["grafana"].public);
        assert_eq!(templates.users.len(), 1);
        assert!(templates.users[0].admin);
    }
}
