# Installation

## From source

```bash
git clone https://github.com/damacus/zitadel-tui.git
cd zitadel-tui
cargo build --release
```

## From `crates.io`

```bash
cargo install zitadel-tui
```

## Running

```bash
# Run the TUI
cargo run

# Or run the built binary directly
./target/release/zitadel-tui
```

On first run, the TUI opens the setup flow and writes configuration to
`~/.config/zitadel-tui/config.toml`.

## CLI help

Use clap help to inspect the current command tree:

```bash
zitadel-tui --help
zitadel-tui apps --help
zitadel-tui users --help
zitadel-tui idps --help
zitadel-tui auth --help
zitadel-tui config --help
```

Supplying a subcommand runs a one-shot command. The legacy `--once` flag still
works, but is no longer required.

## Headless examples

```bash
zitadel-tui apps list
zitadel-tui apps create --template grafana
zitadel-tui apps create \
  --name grafana \
  --redirect-uris https://grafana.example.com/login/generic_oauth,https://grafana.example.com/oauth2/callback \
  --public
zitadel-tui users create \
  --email alice@example.com \
  --first-name Alice \
  --last-name Admin \
  --username alice
zitadel-tui users create-admin \
  --username admin \
  --first-name Admin \
  --last-name User \
  --email admin@example.com \
  --password 'change-me-now'
zitadel-tui users grant-iam-owner --user-id 123456789012345678
zitadel-tui users quick-setup
zitadel-tui idps list
zitadel-tui idps configure-google \
  --client-id google-client-id \
  --client-secret google-client-secret
zitadel-tui --json auth status
zitadel-tui config show
```

## Requirements

- Rust 1.89 or newer
- A Zitadel personal access token or service account file

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo check
```
