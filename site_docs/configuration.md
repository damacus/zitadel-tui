# Configuration

The TUI stores runtime configuration in `~/.config/zitadel-tui/config.toml`:

```toml
zitadel_url = "https://zitadel.example.com"
project_id = "123456789"
apps_config_file = "/path/to/apps.yml"
pat = "zitadel-pat"
service_account_file = "/path/to/service-account.json"
```

## Config fields

`zitadel_url`
: Default host used when `--host` is not passed.

`project_id`
: Default project ID used for app operations when `--project-id` is not
passed. In headless mode the CLI can resolve the default project if this is
omitted.

`apps_config_file`
: Path to the YAML templates file used by `apps create --template`,
`apps quick-setup`, and `users quick-setup`.

`pat`
: Personal access token used when `--token` and `ZITADEL_TOKEN` are not set.

`service_account_file`
: Path to a Zitadel service-account JSON key file used when PAT credentials are
not provided by CLI, environment, or config.

## Apps and users templates

Define your OIDC applications and predefined users in a separate YAML file:

```yaml
# OIDC Applications
apps:
  grafana:
    redirect_uris:
      - https://grafana.example.com/oauth2/callback
      - https://grafana.example.com/login/generic_oauth
    public: false

  mealie:
    redirect_uris:
      - https://mealie.example.com/login
      - https://mealie.example.com/api/auth/oauth/callback
    public: true

# Predefined Users
users:
  - email: admin@example.com
    first_name: Admin
    last_name: User
    admin: true

  - email: user@example.com
    first_name: Regular
    last_name: User
    admin: false
```

See `apps.yml.example` for more examples.

## Authentication precedence

Authentication is resolved in this order:

1. CLI flags
2. Environment variables
3. TOML config

Within each source, PAT credentials are checked before service-account
credentials.

Resolution order in practice:

1. `--token`
2. `ZITADEL_TOKEN`
3. `pat`
4. `--service-account-file`
5. `ZITADEL_SERVICE_ACCOUNT_FILE`
6. `service_account_file`

## CLI options tied to configuration

`--host <HOST>`
: Overrides `zitadel_url`.
Example: `zitadel-tui --host https://zitadel.example.com`

`--project-id <PROJECT_ID>`
: Overrides `project_id`.
Example: `zitadel-tui --once --project-id 123456789 apps list`

`--token <TOKEN>`
: Overrides `pat`.
Example: `zitadel-tui --once --token "$ZITADEL_PAT" auth validate`

`--service-account-file <SERVICE_ACCOUNT_FILE>`
: Overrides `service_account_file`.
Example: `zitadel-tui --once --service-account-file ./service-account.json auth validate`

`--config <CONFIG>`
: Loads a non-default TOML file.
Example: `zitadel-tui --config ./config.toml`

## Notes

- `auth validate` and `config show` do not accept command-specific flags
- OAuth device flow remains visible in the TUI as a placeholder, but is not
  implemented
