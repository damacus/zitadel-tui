# Installation

## From Source

```bash
git clone https://github.com/damacus/zitadel-tui.git
cd zitadel-tui
cargo build --release
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
