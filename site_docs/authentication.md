# Authentication

`zitadel-tui` is intended for administrative Zitadel workflows. For admin use,
authenticate with a Zitadel service-account JSON key.

Most commands need a Zitadel host and API credentials before they can call admin
APIs. Set the host with `--host`, `ZITADEL_URL`, or `zitadel_url` in
`~/.config/zitadel-tui/config.toml`:

```bash
zitadel-tui --host https://zitadel.example.com auth status
```

## Recommended: Service-Account JSON Key

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

You can also set the file path with `ZITADEL_SERVICE_ACCOUNT_FILE` or
`service_account_file` in config:

```toml
zitadel_url = "https://zitadel.example.com"
service_account_file = "/path/to/service-account.json"
```

## Supported: Personal Access Token

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

You can also set the token with `ZITADEL_TOKEN` or `pat` in config.

## Limited: OAuth Device Flow

OAuth Device Flow is available for limited interactive/session use. It is not
the recommended authentication method for administration.

Cached OIDC sessions can run:

- `auth login`
- `auth logout`
- `auth status`

App, user, IDP, Auth API, Management API, and Admin API operations require
service-account JSON or PAT credentials.

Browser login requires a Zitadel native app with the Device Code grant enabled
and JWT access tokens enabled for userinfo access. If you already have admin API
credentials, create the native app from the CLI:

```bash
zitadel-tui --host https://zitadel.example.com apps create-native \
  --name zitadel-tui \
  --device-code
```

Then log in with the native app client ID:

```bash
zitadel-tui --host https://zitadel.example.com auth login --client-id <CLIENT_ID>
```

The command prints a browser URL and user code. Approve the request in the
browser, then the CLI stores the session in:

```text
~/.config/zitadel-tui/tokens.json
```

If `--client-id` is omitted, `zitadel-tui` uses `device_client_id` from config.
If that is also absent, it prompts for the client ID and saves it for later.

## Check Current Authentication

Use `auth status` to verify which credential source is active and which user it
resolves to:

```bash
zitadel-tui --host https://zitadel.example.com auth status
```

For machine-readable output:

```bash
zitadel-tui --host https://zitadel.example.com --json auth status
```

## Log Out

Use `auth logout` to remove the cached browser-login session:

```bash
zitadel-tui auth logout
```

This only removes the session token cache. It does not remove PATs, service
account paths, or other values from config or the environment.

## Credential Resolution

When a command needs authentication, `zitadel-tui` resolves credentials in this
exact order:

1. `--token`
2. `ZITADEL_TOKEN`
3. `pat` in config
4. `--service-account-file`
5. `ZITADEL_SERVICE_ACCOUNT_FILE`
6. `service_account_file` in config
7. Cached session token from `auth login`

This is resolution order, not recommendation order. PAT values are checked
first for compatibility, but service-account JSON keys are preferred for
regular administrative use.

The cached session token is used only for the same Zitadel host it was created
for. If the cached access token is expired and a refresh token is available,
`zitadel-tui` refreshes it automatically.

## ZITADEL Chart v10 Login Service Key

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
