# Zitadel TUI

A beautiful, interactive terminal user interface for managing Zitadel identity provider configuration.

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

## Quick Start

```bash
git clone https://github.com/damacus/zitadel-tui.git
cd zitadel-tui
bundle install
./bin/zitadel-tui
```

## Requirements

- Ruby >= 3.2
- kubectl configured with cluster access (for fetching secrets)

## License

MIT
