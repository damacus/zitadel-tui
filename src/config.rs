use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

#[derive(Clone, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub zitadel_url: Option<String>,
    pub project_id: Option<String>,
    pub apps_config_file: Option<PathBuf>,
    pub pat: Option<String>,
    pub service_account_file: Option<PathBuf>,
    pub device_client_id: Option<String>,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("zitadel_url", &self.zitadel_url)
            .field("project_id", &self.project_id)
            .field("apps_config_file", &self.apps_config_file)
            .field("pat", &self.pat.as_ref().map(|_| "[REDACTED]"))
            .field("service_account_file", &self.service_account_file)
            .field("device_client_id", &self.device_client_id)
            .finish()
    }
}

impl Serialize for AppConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AppConfig", 6)?;
        state.serialize_field("zitadel_url", &self.zitadel_url)?;
        state.serialize_field("project_id", &self.project_id)?;
        state.serialize_field("apps_config_file", &self.apps_config_file)?;
        state.serialize_field("pat", &self.pat.as_ref().map(|_| "[REDACTED]"))?;
        state.serialize_field("service_account_file", &self.service_account_file)?;
        state.serialize_field("device_client_id", &self.device_client_id)?;
        state.end()
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
        Self::load_from_paths(config_paths())
    }

    pub fn save_to_canonical_path(&self) -> Result<PathBuf> {
        let path = Self::canonical_path()?;
        self.write_to_path(&path)?;
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

    fn load_from_paths<I>(paths: I) -> Result<Self>
    where
        I: IntoIterator<Item = PathBuf>,
    {
        for path in paths {
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

    fn write_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            set_dir_permissions(parent)?;
        }

        let contents = toml::to_string_pretty(&PersistedAppConfig::from(self))?;
        write_secure_file(path, &contents)?;
        Ok(())
    }
}

fn config_paths() -> Vec<PathBuf> {
    dirs::config_dir()
        .map(|config_dir| config_dir.join("zitadel-tui").join("config.toml"))
        .into_iter()
        .collect()
}

#[derive(Clone, Deserialize, Serialize, Default)]
#[serde(default)]
struct PersistedAppConfig {
    zitadel_url: Option<String>,
    project_id: Option<String>,
    apps_config_file: Option<PathBuf>,
    pat: Option<String>,
    service_account_file: Option<PathBuf>,
    device_client_id: Option<String>,
}

impl From<&AppConfig> for PersistedAppConfig {
    fn from(value: &AppConfig) -> Self {
        Self {
            zitadel_url: value.zitadel_url.clone(),
            project_id: value.project_id.clone(),
            apps_config_file: value.apps_config_file.clone(),
            pat: value.pat.clone(),
            service_account_file: value.service_account_file.clone(),
            device_client_id: value.device_client_id.clone(),
        }
    }
}

#[cfg(unix)]
fn set_dir_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_dir_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn write_secure_file(path: &Path, contents: &str) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mut file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn write_secure_file(path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env, fs,
        path::{Path, PathBuf},
        sync::{Mutex, OnceLock},
        time::{SystemTime, UNIX_EPOCH},
    };

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = env::temp_dir().join(format!("zitadel-tui-{name}-{unique}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

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
    fn serialization_redacts_tokens() {
        let config = AppConfig {
            zitadel_url: Some("https://zitadel.example.com".to_string()),
            project_id: Some("123".to_string()),
            apps_config_file: Some(PathBuf::from("/tmp/apps.yml")),
            pat: Some("secret-token".to_string()),
            service_account_file: Some(PathBuf::from("/tmp/sa.json")),
            device_client_id: None,
        };

        let value = serde_json::to_value(&config).unwrap();
        assert_eq!(value["pat"], "[REDACTED]");
        assert_eq!(value["service_account_file"], "/tmp/sa.json");
        assert_eq!(value["zitadel_url"], "https://zitadel.example.com");
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
    fn load_uses_xdg_config_only() {
        let _guard = test_lock();
        let cwd_config = temp_dir("cwd").join("config.toml");
        let xdg_config = temp_dir("xdg").join("config.toml");

        fs::write(&cwd_config, r#"zitadel_url = "https://cwd.example.com""#).unwrap();
        fs::write(&xdg_config, r#"zitadel_url = "https://xdg.example.com""#).unwrap();

        let config = AppConfig::load_from_paths(vec![xdg_config.clone()]).unwrap();

        assert_eq!(
            config.zitadel_url.as_deref(),
            Some("https://xdg.example.com")
        );
        assert_ne!(cwd_config, xdg_config);
        assert_eq!(config_paths().len(), 1);
        assert_ne!(config_paths()[0], cwd_config);
    }

    #[test]
    fn save_writes_secret_config_with_restricted_permissions() {
        let _guard = test_lock();
        let path = temp_dir("save").join("config.toml");
        let config = AppConfig {
            zitadel_url: Some("https://zitadel.example.com".to_string()),
            project_id: Some("123".to_string()),
            apps_config_file: Some(PathBuf::from("/tmp/apps.yml")),
            pat: Some("secret-token".to_string()),
            service_account_file: Some(PathBuf::from("/tmp/sa.json")),
            device_client_id: None,
        };

        config.write_to_path(&path).unwrap();
        let contents = fs::read_to_string(&path).unwrap();

        assert!(contents.contains(r#"pat = "secret-token""#));
        assert!(contents.contains(r#"service_account_file = "/tmp/sa.json""#));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let file_mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            let dir_mode = fs::metadata(path.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(file_mode, 0o600);
            assert_eq!(dir_mode, 0o700);
        }
    }

    #[test]
    fn load_device_client_id_from_toml() {
        let config =
            AppConfig::load_from_str(r#"device_client_id = "native-app-client-id""#).unwrap();
        assert_eq!(
            config.device_client_id.as_deref(),
            Some("native-app-client-id")
        );
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
