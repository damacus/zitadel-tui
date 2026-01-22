# Installation

## From Source

```bash
git clone https://github.com/damacus/zitadel-tui.git
cd zitadel-tui
bundle install
```

## Running

```bash
# Run the TUI
./bin/zitadel-tui

# Or with bundle
bundle exec ./bin/zitadel-tui
```

On first run, you'll be prompted to configure your Zitadel URL.

## Requirements

- Ruby >= 3.2
- kubectl configured with cluster access (for fetching secrets)

## Development

```bash
# Install dependencies
bundle install

# Run RuboCop
bundle exec rubocop

# Run tests
bundle exec rspec
```
