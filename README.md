# Zitadel TUI

A Rust terminal UI and headless CLI for managing Zitadel applications, users,
identity providers, and runtime configuration.

## Status

The Rust migration is in progress and is now the primary implementation path.

Implemented today:
- Cargo-based crate, build, lint, and test flow
- Headless commands for apps, users, IDPs, auth validation, and config
- Token and direct service-account authentication with no Kubernetes dependency
- Legacy Ruby YAML config import into canonical Rust TOML config
- `Command Atelier` TUI shell with resource workspace switching and popup inspector
- GitHub Actions release wiring for binaries, Docker images, and `crates.io`

Still in progress:
- full interactive mutation flows inside the TUI
- OAuth device flow
- final removal of the legacy Ruby runtime

## Features

- **Applications**
  - list OIDC applications
  - create applications from flags or templates
  - delete applications
  - regenerate confidential client secrets
  - quick setup from templates YAML

- **Users**
  - list users
  - create human users
  - create imported local admin users
  - grant `IAM_OWNER`
  - quick setup from templates YAML

- **Identity Providers**
  - list IDPs
  - configure Google manually

- **Configuration and Auth**
  - TOML config in XDG config space
  - legacy `~/.zitadel-tui.yml` import
  - auth precedence `CLI > env > config > setup`
  - PAT and direct service-account file support

## Installation

### Build locally

```bash
git clone https://github.com/damacus/zitadel-tui.git
cd zitadel-tui
cargo build --release
```

Run the binary:

```bash
./target/release/zitadel-tui
```

### Install from `crates.io`

```bash
cargo install zitadel-tui
```

## Usage

### Interactive TUI

```bash
zitadel-tui
```

### Headless mode

Non-interactive commands require `--once`. Use `--json` for machine-readable
output.

```bash
zitadel-tui --once apps list
zitadel-tui --once --json auth validate
zitadel-tui --once apps create --name grafana --redirect-uris https://grafana.example.com/login/generic_oauth
zitadel-tui --once users create-admin \
  --username admin \
  --first-name Admin \
  --last-name User \
  --email admin@example.com \
  --password 'change-me-now'
zitadel-tui --once idps configure-google \
  --client-id google-client-id \
  --client-secret google-client-secret
```

## Configuration

Canonical config lives at:

```text
~/.config/zitadel-tui/config.toml
```

Example:

```toml
zitadel_url = "https://zitadel.example.com"
project_id = "123456789"
apps_config_file = "/path/to/apps.yml"
pat = "zitadel-pat"
service_account_file = "/path/to/service-account.json"
```

Legacy import:

```bash
zitadel-tui --once config import-legacy
```

## Templates File

The apps/users templates file remains YAML for compatibility during the
migration.

```yaml
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

## Authentication

Supported today:

1. `--token`, `ZITADEL_TOKEN`, or `pat` in config
2. `--service-account-file`, `ZITADEL_SERVICE_ACCOUNT_FILE`, or
   `service_account_file` in config

Not supported anymore:

- Kubernetes secret lookup
- `kubectl`-backed auth bootstrap

Planned next:

- OAuth device flow with persisted session material

## Docker

```bash
docker build -t zitadel-tui .
docker run -it --rm \
  -v ~/.config/zitadel-tui:/root/.config/zitadel-tui:ro \
  zitadel-tui
```

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo check
```

## Release

The release workflow is tag-driven through `release-please` and publishes:

- GitHub release artifacts
- GHCR container images
- the Rust crate to `crates.io`

The publish job expects `CARGO_REGISTRY_TOKEN` in GitHub Actions secrets.

## License

MIT
