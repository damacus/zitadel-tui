# Manage OIDC Applications

This guide is for Zitadel administrators who need to add, remove, or maintain
OIDC applications in `zitadel-tui`.

![Setup screen showing the TUI initial configuration form with OAuth device still marked as a placeholder and the templates path configured.](images/oidc-apps-setup.png)

## Before you start

You need:

- a Zitadel host URL
- a service-account JSON key, or a PAT for compatibility and quick/manual use
- an optional `apps.yml` if you want template-based quick setup

Why this can feel harder than it should:

- the credential that is best for app administration is the service-account
  JSON key, even though PATs are still checked first by the resolver
- OIDC Device Flow is limited to `auth login`, `auth logout`, and `auth status`;
  it does not run app administration commands
- application settings are not editable in place today; if redirect URIs or the
  client type change, the current workflow is to recreate the app

## Use the TUI

Once authenticated, open the application workspace. The action rail is where
you create apps, rotate secrets, delete apps, or run quick setup from templates.

![Application workspace showing mocked Grafana, Nextcloud, and Paperless-style apps in the selection tray.](images/oidc-apps-workspace.png)

For a manual app:

1. Open `Applications`.
2. Choose `Create application`.
3. Enter the app name and a comma-separated redirect URI list.
4. Leave `Public` off for confidential clients, or toggle it on for public
   clients.
5. Submit the form.

![Create application form filled with a Grafana example and redirect URIs.](images/oidc-apps-create.png)

For ongoing maintenance:

- use `Regenerate secret` for confidential clients
- use `Delete selected` when an app should be removed entirely
- recreate the app if you need to change redirect URIs or switch between public
  and confidential
- use `Quick setup` when the app already exists in `apps.yml`

## Use the CLI

List apps:

```bash
zitadel-tui apps list
```

Create a Grafana app manually:

```bash
zitadel-tui apps create \
  --name grafana \
  --redirect-uris https://grafana.example.com/login/generic_oauth,https://grafana.example.com/oauth2/callback
```

Create a native app for device-flow login:

```bash
zitadel-tui apps create-native --name zitadel-tui --device-code
```

When `--device-code` is used, the returned native app client ID is saved as
`device_client_id` for future `auth login` runs.

Delete an app:

```bash
zitadel-tui apps delete --app-id 123456789012345678
```

Rotate a confidential client secret:

```bash
zitadel-tui apps regenerate-secret --app-id 123456789012345678
```

Quick setup from templates:

```yaml
apps:
  grafana:
    redirect_uris:
      - https://grafana.example.com/oauth2/callback
      - https://grafana.example.com/login/generic_oauth
    public: false

  paperless:
    redirect_uris:
      - https://paperless.example.com/oauth2/callback
      - https://paperless.example.com/accounts/oidc/zitadel/login/callback/
    public: false

  nextcloud:
    redirect_uris:
      - https://nextcloud.example.com/apps/oidc_login/oidc
    public: false
```

Then run:

```bash
zitadel-tui apps quick-setup --names grafana,paperless,nextcloud
```

## If authentication is the blocker

For the smoothest path today:

1. Use a service-account JSON key for regular administration, automation, and
   shared admin tooling.
2. Use a PAT when you need compatibility or quick/manual access.
3. Use device flow only for `auth login`, `auth logout`, and `auth status`.

If `auth login` succeeds, the cached OIDC session still cannot run app
administration commands. Use service-account JSON or PAT credentials for the
commands in this guide.
