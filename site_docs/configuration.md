# Configuration

The TUI stores runtime configuration in `~/.config/zitadel-tui/config.toml`:

```toml
zitadel_url = "https://zitadel.example.com"
project_id = "123456789"
templates_file = "/path/to/apps.yml"
```

## Apps and Users Configuration

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
    admin: true  # Will be granted IAM_OWNER role

  - email: user@example.com
    first_name: Regular
    last_name: User
    admin: false
```

See `apps.yml.example` for more examples.

## Authentication

The TUI supports two authentication methods today:

1. **Service Account** - Uses a local service account file referenced by
   `service_account_file` in the TOML config, or passed by CLI/environment.

2. **Personal Access Token (PAT)** - Uses a token from `pat` in the TOML
   config, or passed by CLI/environment.

OAuth device flow remains visible in the TUI as a placeholder, but is not
implemented yet.
