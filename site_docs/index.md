# Zitadel TUI

A Rust terminal UI and headless CLI for managing Zitadel applications, users,
identity providers, and runtime configuration.

## Features

- **Applications**
  - List OIDC applications
  - Create applications from manual flags or templates
  - Regenerate confidential client secrets
  - Delete applications
  - Quick setup from YAML templates

- **Users**
  - List users
  - Create human users
  - Create imported local admin users
  - Grant `IAM_OWNER`
  - Quick setup from YAML templates

- **Identity Providers**
  - List configured IDPs
  - Configure Google manually

- **Configuration and Auth**
  - Canonical TOML config in XDG config space
  - Auth precedence `CLI > env > config`
  - PAT precedence over service-account credentials within each source
  - OAuth device flow remains visible as a placeholder only

## Quick Start

```bash
git clone https://github.com/damacus/zitadel-tui.git
cd zitadel-tui
cargo build --release
./target/release/zitadel-tui
```

## CLI Overview

Run the interactive TUI:

```bash
zitadel-tui
```

Run any headless command:

```bash
zitadel-tui --once apps list
```

Important behavior:

- every headless command requires `--once`
- `--json` switches headless output to JSON envelopes
- `--project-id` is optional in headless mode because the CLI can resolve the
  default project when omitted

Global options:

- `--host`: override `ZITADEL_URL`
- `--project-id`: override `ZITADEL_PROJECT_ID`
- `--token`: use `ZITADEL_TOKEN`
- `--service-account-file`: use `ZITADEL_SERVICE_ACCOUNT_FILE`
- `--config`: load a non-default TOML file
- `--json`: print machine-readable output
- `--once`: enable non-interactive mode

Subcommands:

- `apps`: `list`, `create`, `delete`, `regenerate-secret`, `quick-setup`
- `users`: `list`, `create`, `create-admin`, `grant-iam-owner`, `quick-setup`
- `idps`: `list`, `configure-google`
- `auth`: `validate`
- `config`: `show`

See the top-level [README](../README.md) for the complete option-by-option CLI
reference.

## Runtime Configuration

The runtime config lives at:

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

Templates for apps and users remain YAML-based.

## Requirements

- Rust 1.89 or newer
- A Zitadel PAT or service account file

## License

MIT
