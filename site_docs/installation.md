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
zitadel-tui --once apps --help
zitadel-tui --once users --help
zitadel-tui --once idps --help
zitadel-tui --once auth --help
zitadel-tui --once config --help
```

Every headless command requires `--once`.

## Headless examples

```bash
zitadel-tui --once apps list
zitadel-tui --once apps create --template grafana
zitadel-tui --once apps create \
  --name grafana \
  --redirect-uris https://grafana.example.com/login/generic_oauth,https://grafana.example.com/oauth2/callback \
  --public
zitadel-tui --once users create \
  --email alice@example.com \
  --first-name Alice \
  --last-name Admin \
  --username alice
zitadel-tui --once users create-admin \
  --username admin \
  --first-name Admin \
  --last-name User \
  --email admin@example.com \
  --password 'change-me-now'
zitadel-tui --once users grant-iam-owner --user-id 123456789012345678
zitadel-tui --once users quick-setup
zitadel-tui --once idps list
zitadel-tui --once idps configure-google \
  --client-id google-client-id \
  --client-secret google-client-secret
zitadel-tui --once --json auth status
zitadel-tui --once config show
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
