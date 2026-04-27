# Authentication

Most `zitadel-tui` commands need a Zitadel host and an access token before they
can call admin APIs.

Set the host with `--host`, `ZITADEL_URL`, or `zitadel_url` in
`~/.config/zitadel-tui/config.toml`:

```bash
zitadel-tui --host https://zitadel.example.com auth status
```

## Recommended: Browser Login

Use browser login for interactive administrator sessions.

Browser login requires a Zitadel native app with the Device Code grant enabled
and JWT access tokens enabled for API access.

If you already have another CLI credential, create the native app from the CLI:

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

Future commands use the cached session automatically:

```bash
zitadel-tui --host https://zitadel.example.com apps list
zitadel-tui --host https://zitadel.example.com users list
```

If `--client-id` is omitted, `zitadel-tui` uses `device_client_id` from config.
If that is also absent, it prompts for the client ID and saves it for later.

## Alternative: Personal Access Token

Use a personal access token when you want to pass an existing administrator
token directly.

```bash
zitadel-tui --host https://zitadel.example.com \
  --token "$ZITADEL_PAT" \
  auth status
```

You can also set the token with `ZITADEL_TOKEN` or `pat` in config.

## Alternative: Service Account File

Use a service account file for non-interactive administrator workflows.

```bash
zitadel-tui --host https://zitadel.example.com \
  --service-account-file ./service-account.json \
  auth status
```

You can also set the file path with `ZITADEL_SERVICE_ACCOUNT_FILE` or
`service_account_file` in config.

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

The cached session token is used only for the same Zitadel host it was created
for. If the cached access token is expired and a refresh token is available,
`zitadel-tui` refreshes it automatically.
