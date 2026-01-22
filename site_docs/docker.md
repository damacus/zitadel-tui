# Docker

## Pull from GitHub Container Registry

```bash
docker pull ghcr.io/damacus/zitadel-tui:latest
```

## Run with kubectl access

```bash
docker run -it --rm \
  -v ~/.kube:/root/.kube:ro \
  -v ~/.zitadel-tui.yml:/root/.zitadel-tui.yml \
  ghcr.io/damacus/zitadel-tui:latest
```

## Docker Compose

```yaml
services:
  zitadel-tui:
    image: ghcr.io/damacus/zitadel-tui:latest
    volumes:
      - ~/.kube:/root/.kube:ro
      - ~/.zitadel-tui.yml:/root/.zitadel-tui.yml
    stdin_open: true
    tty: true
```
