# Agent guide: zitadel-tui headless CLI

This file is the machine-facing contract for `zitadel-tui`. It is written
for LLM agents and automation driving the binary non-interactively. Running
`zitadel-tui agent` prints this file verbatim.

The binary has two modes: run with no subcommand and it opens an interactive
TUI for humans; run with a subcommand and it executes once and prints JSON.
There is no human text output mode for subcommands — parse the envelope.

## Output contract

Every API subcommand prints exactly one JSON envelope on stdout (pretty
printed) and nothing else on stdout. Diagnostics and the device-login
instructions go to stderr.

Success (exit 0):

```json
{
  "command": "apps.list",
  "ok": true,
  "result": []
}
```

Failure (exit 1):

```json
{
  "command": "apps.delete",
  "ok": false,
  "error": {
    "kind": "not_found",
    "message": "app 'grafana' not found",
    "details": { "lookup": { "field": "name", "value": "grafana" } }
  }
}
```

- `command` is the dot-path of the command that ran (see the command map).
- `result` is the command payload on success.
- `error.kind` classifies the failure; `error.message` is a human-readable
  sentence; `error.details` carries structured context and is omitted when
  there is none.
- `zitadel-tui agent` prints this document instead of an envelope and still
  exits 0.

Exit codes:

| Code | Meaning                                                      |
|------|--------------------------------------------------------------|
| 0    | Success — `ok:true` envelope (or the agent guide) on stdout  |
| 1    | Command error — `ok:false` envelope; branch on `error.kind`  |
| 2    | Argument parse failure — clap usage text on stderr, not JSON |

### `error.kind` values

| kind        | Meaning                                                                                                  |
|-------------|----------------------------------------------------------------------------------------------------------|
| `usage`     | Missing or invalid input clap cannot express: no `--password`, no host, `auth login` without a client id |
| `auth`      | Credential resolution failed, or an OIDC session token was used for an API command                       |
| `not_found` | A selector or template name matched zero resources                                                       |
| `ambiguous` | A selector matched more than one resource                                                                |
| `api`       | Zitadel API, transport, or decode failure (the fallback kind)                                            |
| `config`    | Config file load, parse, or save failure                                                                 |
| `io`        | Local filesystem failure outside config (e.g. token cache)                                               |

`error.details` shapes:

- `not_found` / `ambiguous`: `details.lookup` echoes the selector as
  `{ "field": ..., "value": ... }` where `field` is one of `app_id`, `name`,
  `user_id`, `username`, `email`, `idp_id`, `template`.
- `ambiguous` also carries `details.matches`: a list of `{ "id", "name" }`
  summaries. For user matches, `name` contains the username.

## Authentication

Credential resolution order (first match wins):

1. `--token`
2. `ZITADEL_TOKEN`
3. `pat` in config
4. `--service-account-file`
5. `ZITADEL_SERVICE_ACCOUNT_FILE`
6. `service_account_file` in config
7. Cached OIDC session token from `auth login` (auto-refreshed)

Credential types:

- **Service-account JSON key** — recommended for agents and automation.
  Exchanges a signed JWT for an access token; suitable for every API
  command.
- **PAT** — bearer token; checked first in resolution order for
  compatibility. Works for every API command.
- **OIDC session token** — produced by `auth login` (device flow) and cached
  in `~/.config/zitadel-tui/tokens.json`. It can only run `auth login`,
  `auth logout`, and `auth status`. Any API command fails with
  `error.kind:"auth"`. Do not provision sessions for agent use.

`auth status` reports which credential resolved in `result.auth_source`:
`cli PAT`, `env PAT`, `config PAT`, `cli service account`,
`env service account`, `config service account`, `session token`, or
`session token (refreshed)`.

A missing host is a `usage` error; failed or rejected credentials are `auth`
errors. Config lives at `~/.config/zitadel-tui/config.toml` (TOML); keys:
`zitadel_url`, `project_id`, `pat`, `service_account_file`,
`apps_config_file`, `device_client_id`.

## Global flags

| Flag                     | Env variable                   | Purpose                                                            |
|--------------------------|--------------------------------|--------------------------------------------------------------------|
| `--host`                 | `ZITADEL_URL`                  | Zitadel base URL (required unless in config)                       |
| `--project-id`           | `ZITADEL_PROJECT_ID`           | Project for `apps` commands; default project resolved when omitted |
| `--token`                | `ZITADEL_TOKEN`                | PAT credential                                                     |
| `--service-account-file` | `ZITADEL_SERVICE_ACCOUNT_FILE` | Service-account JSON key path                                      |
| `--config`               | —                              | Alternate TOML config path                                         |
| `--full`                 | —                              | Raw API payloads instead of summary projections                    |

Secret-bearing command flags also read env vars so secrets stay off the
command line:

| Flag                                    | Env variable                   |
|-----------------------------------------|--------------------------------|
| `users create-admin --password`         | `ZITADEL_USER_PASSWORD`        |
| `idps configure-google --client-secret` | `ZITADEL_GOOGLE_CLIENT_SECRET` |

Every `list` command accepts `--limit` (1–1000, default 100). There is no
offset or cursor pagination: only the first `--limit` items are returned.

## Command map

Exactly one selector flag is required where a selector column lists
alternatives. `command` is the value of the envelope's `command` field.

| command                  | Invocation                                                                                  |
|--------------------------|---------------------------------------------------------------------------------------------|
| `apps.list`              | `apps list [--limit N]`                                                                     |
| `apps.get`               | `apps get (--app-id ID \| --name NAME)`                                                     |
| `apps.create`            | `apps create (--template T \| --name N --redirect-uris URI[,URI...] [--public])`            |
| `apps.create-native`     | `apps create-native --name N [--device-code]`                                               |
| `apps.delete`            | `apps delete (--app-id ID \| --name NAME)`                                                  |
| `apps.regenerate-secret` | `apps regenerate-secret (--app-id ID \| --name NAME) [--client-id CID]`                     |
| `apps.quick-setup`       | `apps quick-setup [--names NAME[,NAME...]]`                                                 |
| `users.list`             | `users list [--limit N]`                                                                    |
| `users.get`              | `users get (--user-id ID \| --username NAME \| --email ADDR)`                               |
| `users.create`           | `users create --email ADDR --first-name F --last-name L [--username NAME]`                  |
| `users.create-admin`     | `users create-admin --username NAME --first-name F --last-name L --email ADDR --password P` |
| `users.grant-iam-owner`  | `users grant-iam-owner (--user-id ID \| --username NAME \| --email ADDR)`                   |
| `users.quick-setup`      | `users quick-setup`                                                                         |
| `idps.list`              | `idps list [--limit N]`                                                                     |
| `idps.get`               | `idps get (--idp-id ID \| --name NAME)`                                                     |
| `idps.configure-google`  | `idps configure-google --client-id CID [--client-secret S] [--name N]`                      |
| `projects.list`          | `projects list [--limit N]`                                                                 |
| `auth.login`             | `auth login [--client-id CID]`                                                              |
| `auth.logout`            | `auth logout`                                                                               |
| `auth.status`            | `auth status`                                                                               |
| `config.show`            | `config show`                                                                               |
| `agent`                  | `agent` — prints this document, no envelope                                                 |

Selector matching is exact and case-sensitive. `--username` matches either
the user's `user_name` or `preferred_login_name`; `--email` matches the
human user's email address.

## Result shapes

`list` and `get` commands return summary projections by default; pass
`--full` to get each item's verbatim raw API payload instead.

| Command                               | Default `result` shape                                                                                                                            |
|---------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------|
| `apps.list` / `apps.get`              | `{id, name, client_id, kind, redirect_uris, state}`; `kind` is one of `device-code`, `native`, `public`, `confidential`, `oidc`, `api`, `unknown` |
| `users.list` / `users.get`            | `{id, username, login_name, display_name, email, user_type, state}`; `user_type` is `human` or `machine`                                          |
| `idps.list` / `idps.get`              | `{id, name, type, state}`                                                                                                                         |
| `projects.list`                       | `{id, name, state}`                                                                                                                               |
| `apps.create` / `apps.create-native`  | `{appId?, clientId?, clientSecret?}`                                                                                                              |
| `apps.delete`                         | `{}`                                                                                                                                              |
| `apps.regenerate-secret`              | `{clientSecret?}` plus `clientId` when `--client-id` was passed                                                                                   |
| `apps.quick-setup`                    | Array of `{appId?, clientId?, clientSecret?}`                                                                                                     |
| `users.create` / `users.create-admin` | `{userId?}`                                                                                                                                       |
| `users.grant-iam-owner`               | `{memberId?}`                                                                                                                                     |
| `users.quick-setup`                   | Array of `{userId?}`                                                                                                                              |
| `idps.configure-google`               | `{id?, name?, type?, state?}`                                                                                                                     |
| `auth.login`                          | `{status:"authenticated", host, verification_uri, user_code, verification_uri_complete?, token_cache}`                                            |
| `auth.logout`                         | `{status:"logged out"}`                                                                                                                           |
| `auth.status`                         | `{host, auth_source, user_id, login_name}`                                                                                                        |
| `config.show`                         | The active config; `pat` is redacted to `"[REDACTED]"`                                                                                            |

## Behaviour notes

- **No prompts exist anywhere.** Nothing reads stdin. Every input comes
  from flags, env vars, or config — pass all required values or the command
  fails with `usage`.
- **`auth login` never prompts.** Without `--client-id` or `device_client_id`
  in config it fails with `usage`. On success it prints the verification URL
  and user code to stderr immediately, then polls until browser approval —
  the JSON result arrives only after approval, so surface stderr to the
  human. Login sessions still cannot run API commands.
- **Results may contain secrets.** `apps create`, `apps create-native`,
  `apps regenerate-secret`, and `apps quick-setup` return `clientSecret`
  verbatim for confidential clients — that is intended. Treat stdout as
  sensitive and store secrets before discarding output.
- **Selectors resolve within the first 100 resources only.** Selector
  resolution fetches one search page of 100 items and matches client-side;
  resources beyond that page report `not_found`. Prefer `--*-id` selectors,
  which are unique, over name/email selectors.
- **Reads before writes.** `apps delete`, `apps regenerate-secret`, and
  `users grant-iam-owner` perform a search call to resolve the selector
  before the mutation — the credential needs list/read permission on the
  resource, not just write permission. All `get` commands use the same
  search call.
- **`apps create`** requires `--template` or both `--name` and
  `--redirect-uris`; `users create-admin` requires `--password` (or
  `ZITADEL_USER_PASSWORD`); `idps configure-google` requires
  `--client-secret` (or `ZITADEL_GOOGLE_CLIENT_SECRET`). These are `usage`
  errors when missing.
- Templates live in the YAML file named by `apps_config_file` in config; an
  unknown template name is `not_found` with `details.lookup.field:"template"`.

## Recipes

### Check bootstrap credentials

```bash
zitadel-tui \
  --host https://zitadel.example.com \
  --service-account-file ./service-account.json \
  auth status
```

Expect `ok:true` with `result.auth_source` of `"cli service account"` and a
resolved `user_id`/`login_name`. `ok:false` with `auth` or `usage` means
credentials or host are missing.

### Create an OIDC app and capture the client secret

```bash
zitadel-tui \
  --host https://zitadel.example.com \
  --service-account-file ./service-account.json \
  --project-id 123456789 \
  apps create \
    --name grafana \
    --redirect-uris https://grafana.example.com/oauth2/callback
```

`result.clientSecret` is shown once — store it. Rotate later with
`apps regenerate-secret --app-id <appId>` (or `--name grafana`); the new
secret replaces the old one immediately.

### Create an admin user and grant IAM_OWNER

```bash
export ZITADEL_USER_PASSWORD='change-me-now'
zitadel-tui \
  --host https://zitadel.example.com \
  --service-account-file ./service-account.json \
  users create-admin \
    --username admin \
    --first-name Admin \
    --last-name User \
    --email admin@example.com
```

Read `result.userId`, then:

```bash
zitadel-tui \
  --host https://zitadel.example.com \
  --service-account-file ./service-account.json \
  users grant-iam-owner --user-id <userId>
```

Expect `result.memberId` on success.
