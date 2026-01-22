# Configuration

The TUI stores configuration in `~/.zitadel-tui.yml`:

```yaml
zitadel_url: https://zitadel.example.com
project_id: "123456789"
apps_config_file: /path/to/apps.yml
```

## Apps and Users Configuration

Define your OIDC applications and predefined users in a YAML file:

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

The TUI supports two authentication methods:

1. **Service Account (JWT)** - Uses a service account key from Kubernetes
   secret `zitadel-admin-sa` in namespace `authentication`

2. **Personal Access Token (PAT)** - Uses a PAT from Kubernetes secret
   `zitadel-admin-sa-pat` in namespace `authentication`
