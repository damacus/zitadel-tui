# Zitadel TUI

A Rust terminal UI and headless CLI for managing Zitadel applications, users,
identity providers, and runtime configuration.

## Status

The migration is complete: the project is now Rust-only.

Current status:

- Rust crate, TUI, CLI, and release automation are the only supported runtime path
- runtime config is TOML-only in the XDG config directory
- service-account JSON authentication is recommended for administration; PATs
  are supported for compatibility, and OAuth Device Flow is available for
  limited interactive/session use
- app and user templates remain YAML-based

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
  - auth precedence `CLI > env > config > session token`
  - PAT credentials are checked before service-account credentials
  - service-account JSON authentication for administrative workflows
  - PAT support for compatibility and quick/manual use
  - OAuth Device Flow (`auth login`) for limited interactive/session use
  - OIDC session tokens cached in `~/.config/zitadel-tui/tokens.json` with auto-refresh

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

Supplying a subcommand runs the existing one-shot command path. The deprecated
`--once` flag is still accepted for compatibility, but is no longer required.
`--once` on its own is invalid because there is no subcommand to run.

Use `--json` for machine-readable output.

```bash
zitadel-tui apps list
zitadel-tui --json auth status
zitadel-tui apps create --name grafana --redirect-uris https://grafana.example.com/login/generic_oauth
zitadel-tui users create-admin \
  --username admin \
  --first-name Admin \
  --last-name User \
  --email admin@example.com \
  --password 'change-me-now'
zitadel-tui idps configure-google \
  --client-id google-client-id \
  --client-secret google-client-secret
```

### Global options

`--host <HOST>`
: Override the Zitadel base URL. Also available as `ZITADEL_URL`.
Example: `zitadel-tui --host https://zitadel.example.com`

`--project-id <PROJECT_ID>`
: Use a specific project for app operations. Also available as
`ZITADEL_PROJECT_ID`. In headless mode this is optional because the CLI can
resolve the default project when omitted.
Example: `zitadel-tui --project-id 123456789 apps list`

`--token <TOKEN>`
: Authenticate with a PAT. Also available as `ZITADEL_TOKEN`.
Example: `zitadel-tui --token "$ZITADEL_PAT" auth status`

`--service-account-file <SERVICE_ACCOUNT_FILE>`
: Authenticate with a Zitadel service-account JSON key file. Also available as
`ZITADEL_SERVICE_ACCOUNT_FILE`.
Example: `zitadel-tui --service-account-file ./service-account.json auth status`

`--config <CONFIG>`
: Read runtime configuration from a non-default TOML file instead of the
canonical XDG path.
Example: `zitadel-tui --config ./config.toml`

`--json`
: Print JSON envelopes for headless commands.
Example: `zitadel-tui --json config show`

`--once`
: Deprecated compatibility flag for one-shot subcommand execution. Subcommands
now run one-shot without it.
Example: `zitadel-tui --once users list`

### Command reference

#### `apps`

`apps list`
: List OIDC applications for the active project.
Example: `zitadel-tui apps list`

`apps create`
: Create an OIDC application. Use either `--template <TEMPLATE>` or the manual
combination of `--name <NAME>` plus at least one `--redirect-uris <URI>`.
Example: `zitadel-tui apps create --template grafana`
Example: `zitadel-tui apps create --name grafana --redirect-uris https://grafana.example.com/login/generic_oauth,https://grafana.example.com/oauth2/callback --public`

`--name <NAME>`
: App name when creating manually. Ignored when `--template` is used.

`--redirect-uris <REDIRECT_URIS>`
: Comma-delimited redirect URI list for manual app creation.

`--public`
: Create the app as a public client for manual app creation.

`--template <TEMPLATE>`
: Create the app from a named entry in `apps_config_file`.

`apps create-native`
: Create a native OIDC application. With `--device-code`, the CLI configures JWT access tokens and saves the returned client ID as `device_client_id` so the app can be used for `auth login`.
Example: `zitadel-tui apps create-native --name zitadel-tui --device-code`

`--name <NAME>`
: Display name for the native application.

`--device-code`
: Enable the Device Code grant for CLI login sessions. This also switches the generated client to JWT access tokens.

`apps delete`
: Delete an application by Zitadel app ID.
Example: `zitadel-tui apps delete --app-id 123456789012345678`

`--app-id <APP_ID>`
: Target application ID for `apps delete` and `apps regenerate-secret`.

`apps regenerate-secret`
: Regenerate a confidential client's secret.
Example: `zitadel-tui apps regenerate-secret --app-id 123456789012345678`

`--client-id <CLIENT_ID>`
: Optional client ID annotation included in the command result.

`apps quick-setup`
: Create apps from all configured templates, or only the comma-delimited names
passed with `--names`.
Example: `zitadel-tui apps quick-setup`
Example: `zitadel-tui apps quick-setup --names grafana,mealie`

`--names <NAMES>`
: Comma-delimited subset of app template names to create.

#### `users`

`users list`
: List users.
Example: `zitadel-tui users list`

`users create`
: Create a human user.
Example: `zitadel-tui users create --email alice@example.com --first-name Alice --last-name Admin --username alice`

`--email <EMAIL>`
: Email address for `users create` and `users create-admin`.

`--first-name <FIRST_NAME>`
: First name for `users create` and `users create-admin`.

`--last-name <LAST_NAME>`
: Last name for `users create` and `users create-admin`.

`--username <USERNAME>`
: Optional login name for `users create`; required for `users create-admin`.

`users create-admin`
: Import a local admin user and grant admin access. In headless mode
`--password <PASSWORD>` is required.
Example: `zitadel-tui users create-admin --username admin --first-name Admin --last-name User --email admin@example.com --password 'change-me-now'`

`--password <PASSWORD>`
: Password for `users create-admin` in headless mode.

`users grant-iam-owner`
: Grant the `IAM_OWNER` role to an existing user.
Example: `zitadel-tui users grant-iam-owner --user-id 123456789012345678`

`--user-id <USER_ID>`
: Target user ID for `users grant-iam-owner`.

`users quick-setup`
: Create every user from the YAML templates file. This command has no
command-specific flags.
Example: `zitadel-tui users quick-setup`

#### `idps`

`idps list`
: List configured identity providers.
Example: `zitadel-tui idps list`

`idps configure-google`
: Create a Google identity provider. In headless mode `--client-secret` is
required.
Example: `zitadel-tui idps configure-google --client-id google-client-id --client-secret google-client-secret`

`--client-id <CLIENT_ID>`
: Google OAuth client ID.

`--client-secret <CLIENT_SECRET>`
: Google OAuth client secret. Required in headless mode.

`--name <NAME>`
: Display name for the provider. Defaults to `Google`.

#### `auth`

`auth login`
: Authenticate via the OAuth 2.0 Device Authorization Grant for limited
interactive/session use. Prints a URL and short code, waits for browser
approval, then saves the access and refresh tokens to
`~/.config/zitadel-tui/tokens.json`. Requires a Zitadel native app with the
Device Code grant enabled and JWT access tokens configured for userinfo access.
The cached session is usable for `auth status`, `auth login`, and `auth logout`
only; use service-account JSON or PAT credentials for app, user, IDP, and admin
API operations.
Example: `zitadel-tui --host https://zitadel.example.com auth login`

`--client-id <CLIENT_ID>`
: The Zitadel native app client ID. If omitted and not set in config, the
command prompts interactively and saves the value to config for future use.
Also available as `device_client_id` in config.

`auth logout`
: Remove the stored session token. Subsequent commands will require explicit
credentials or a new `auth login`.
Example: `zitadel-tui auth logout`

`auth status`
: Resolve credentials, authenticate, and report the active auth source plus the
current user identity. Works with any credential source including a cached session token.
Example: `zitadel-tui --json auth status`

#### `config`

`config show`
: Print the active runtime configuration with secrets redacted. This command
has no command-specific flags.
Example: `zitadel-tui config show`

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
device_client_id = "your-native-app-client-id"
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

`zitadel-tui` is intended for administrative Zitadel workflows. For admin use,
authenticate with a Zitadel service-account JSON key.

Authentication is resolved in this order:

1. `--token` / `ZITADEL_TOKEN` / `pat` in config (PAT)
2. `--service-account-file` / `ZITADEL_SERVICE_ACCOUNT_FILE` / `service_account_file` in config
3. Cached session token from `auth login` (with automatic refresh)

This is resolution order, not recommendation order. PAT values are checked first
for compatibility, but service-account JSON keys are preferred for regular
administrative use.

### Recommended: Service-Account JSON Key

Service-account authentication uses a private key JWT flow: the CLI signs a
short-lived assertion with the private key from the JSON key file, exchanges it
with Zitadel for an access token, and uses that token for API calls.

This is the recommended authentication method for:

- managing OIDC applications
- creating and updating users
- configuring identity providers
- bootstrap and recovery workflows
- repeatable automation

```bash
zitadel-tui \
  --host https://zitadel.example.com \
  --service-account-file ./service-account.json \
  auth status
```

The same value can be configured with:

```bash
export ZITADEL_SERVICE_ACCOUNT_FILE=./service-account.json
```

or in `~/.config/zitadel-tui/config.toml`:

```toml
zitadel_url = "https://zitadel.example.com"
service_account_file = "/path/to/service-account.json"
```

### Supported: Personal Access Token

PAT authentication remains supported for compatibility, quick testing, and
manual workflows.

PATs are bearer tokens: anyone with the token can use it directly until it
expires or is revoked. Prefer service-account JSON keys for regular
administrative use.

```bash
zitadel-tui \
  --host https://zitadel.example.com \
  --token "$ZITADEL_TOKEN" \
  auth status
```

### Limited: OAuth Device Flow

Register a native app in your Zitadel instance with the **Device Code** grant
type enabled and JWT access tokens enabled, then log in:

```bash
zitadel-tui --host https://zitadel.example.com auth login
```

The command prints a URL and a short code. Open the URL in your browser,
enter the code, and approve the request. The CLI polls in the background and
saves the access and refresh tokens to `~/.config/zitadel-tui/tokens.json`
(mode `0600`).

OAuth Device Flow is available for limited interactive/session use, but it is
not the recommended authentication method for administration. Cached OIDC
sessions can run `auth status`, `auth login`, and `auth logout`; app, user, IDP,
Auth API, Management API, and Admin API operations require service-account JSON
or PAT credentials.

Tokens are silently refreshed when they expire. Log out with:

```bash
zitadel-tui auth logout
```

### Token cache

The session token cache lives at:

```text
~/.config/zitadel-tui/tokens.json
```

It is created with mode `0600`. The cache stores the access token, refresh
token, expiry timestamp, client ID, and host. The `device_client_id` config
field remembers your client ID so you only need `--client-id` once.

### ZITADEL Chart v10 Login Service Key

The ZITADEL Helm chart v10 uses an internal login-service keypair for the hosted
Login UI. That keypair is not used by `zitadel-tui`.

Do not confuse these credentials:

- `zitadel-login-service-key`: internal Kubernetes/chart credential for the
  hosted ZITADEL Login UI
- service-account JSON key: client-side admin credential for tools like
  `zitadel-tui`
- public HTTPS certificate: browser/client TLS for your Zitadel URL

`zitadel-tui` should authenticate with a Zitadel service-account JSON key for
administrative workflows. Do not reuse the chart's `zitadel-login-service-key`
certificate or private key as a CLI credential.

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

### Git hooks

This repo uses `lefthook` for local guardrails:

```bash
lefthook install
```

Configured hooks:

- `pre-commit`: `cargo fmt --check`
- `commit-msg`: enforce Conventional Commits
- `pre-push`: `cargo build`

## Release

The release workflow is tag-driven through `release-please` and publishes:

- GitHub release artifacts
- GHCR container images
- the Rust crate to `crates.io`

The publish job expects `CARGO_REGISTRY_TOKEN` in GitHub Actions secrets.
If a GitHub outage or transient runner failure interrupts publishing after a
tag is created, run the `release` workflow manually with the existing tag name
to retry the release jobs.

## License

MIT
