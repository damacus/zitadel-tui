# Zitadel TUI

A beautiful, interactive terminal user interface for managing Zitadel identity
provider configuration.

## Features

- **OIDC Application Management**
  - List all applications
  - Create new applications (with predefined templates or custom)
  - Regenerate client secrets
  - Delete applications
  - Quick setup using YAML-configured app templates

- **User Management**
  - List all users
  - Create new users
  - Create admin users with password authentication
  - Grant IAM_OWNER role

- **Identity Provider Configuration**
  - List configured IDPs
  - Configure Google OAuth IDP
  - Fetch credentials from Kubernetes secrets

## Requirements

- Ruby >= 3.2
- kubectl configured with cluster access (for fetching secrets)

## Installation

```bash
git clone https://github.com/damacus/zitadel-tui.git
cd zitadel-tui
bundle install
```

## Usage

```bash
# Run the TUI
./bin/zitadel-tui

# Or with bundle
bundle exec ./bin/zitadel-tui
```

On first run, you'll be prompted to configure your Zitadel URL.

## Docker

```bash
# Pull from GitHub Container Registry
docker pull ghcr.io/damacus/zitadel-tui:latest

# Run with kubectl access
docker run -it --rm \
  -v ~/.kube:/root/.kube:ro \
  -v ~/.zitadel-tui.yml:/root/.zitadel-tui.yml \
  ghcr.io/damacus/zitadel-tui:latest
```

## Configuration

The TUI stores configuration in `~/.zitadel-tui.yml`:

```yaml
zitadel_url: https://zitadel.example.com
project_id: "123456789"
apps_config_file: /path/to/apps.yml
```

### Apps and Users Configuration

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

## Development

```bash
# Install dependencies
bundle install

# Run RuboCop
bundle exec rubocop

# Run tests
bundle exec rspec
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and linting
5. Submit a pull request

## License

MIT
