# Docker

## Pull from GitHub Container Registry

```bash
docker pull ghcr.io/damacus/zitadel-tui:latest
```

## Run with TOML config

```bash
docker run -it --rm \
  -v ~/.config/zitadel-tui:/root/.config/zitadel-tui:ro \
  ghcr.io/damacus/zitadel-tui:latest
```

## Docker Compose

```yaml
services:
  zitadel-tui:
    image: ghcr.io/damacus/zitadel-tui:latest
    volumes:
      - ~/.config/zitadel-tui:/root/.config/zitadel-tui:ro
    stdin_open: true
    tty: true
```
