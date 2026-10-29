# Agent-Consumable CLI + Maintainability Refactor

Make zitadel-tui's headless commands reliable for LLM agents (JSON envelopes,
exit codes, structured errors, name-based lookups, discovery surface) while
eliminating duplicated implementations via a typed API + unified ops layer.

## Spec / authority

Approved megaplan (this document is the binding spec). No external spec file.

## Global constraints (apply to every task)

- Rust crate `zitadel-tui`, edition 2021. No new runtime deps except
  `thiserror` (allowed, optional, only if the implementer prefers it for
  `CommandError`).
- **TDD is required** for every production change: write the failing test
  first, then implement. Report must include RED/GREEN evidence.
- Gates that must pass before every commit: `cargo fmt --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`.
- Conventional commits (`feat:`, `fix:`, `refactor:`, `test:`, `docs:`,
  `chore:`). lefthook enforces fmt on pre-commit, conventional messages on
  commit-msg, `cargo build` on pre-push. Breaking changes use `feat!:` or a
  `BREAKING CHANGE:` footer.
- Never print or log secrets: PAT, clientSecret, passwords, service-account
  keys must not appear in error text or logs (existing tests assert this).
- Work happens on the current boundary branch only; do not touch other
  branches or files outside the task scope.
- Shell on this machine is fish — use portable single-line commands.
- Each task lands on its designated stack boundary branch:
  Tasks 1 → `agent-cli/1-hygiene`; Tasks 2–3 → `agent-cli/2-typed-ops`;
  Tasks 4–5 → `agent-cli/3-contract`; Tasks 6–7 → `agent-cli/4-commands`.

## Architecture context (current state)

- `src/main.rs` — binary; parses `Cli`, calls `commands::execute_command`,
  wraps result in `CommandEnvelope`, prints pretty JSON. On error with
  `--json` prints `ok:false` and returns `Ok(())` (exit 0 — the bug being
  fixed in Task 4). No subcommand → TUI via `TuiConductor::bootstrap` +
  `tui_runtime::run_tui`.
- `src/cli.rs` — clap derive: global flags `--host`/`ZITADEL_URL`,
  `--project-id`/`ZITADEL_PROJECT_ID`, `--token`/`ZITADEL_TOKEN`,
  `--service-account-file`/`ZITADEL_SERVICE_ACCOUNT_FILE`, `--config`,
  `--json`, `--once`. Groups: apps, users, idps, auth, config.
  `command_name()` returns only the group name.
- `src/commands/{apps,users,idps,auth,config,shared}.rs` — headless
  implementations returning `serde_json::Value`. `shared.rs` has
  `authenticated_client`, `resolved_host`, `resolved_host_or_cache`,
  `resolved_project_id`.
- `src/conductor/` — TUI orchestration. `runtime.rs::refresh_runtime`
  resolves auth + project + lists records; `mutations.rs` re-implements
  every mutation as `CanvasMode` outcomes; `forms.rs` builds forms and has
  field helpers; `records.rs` maps `Value`→`Record` (incl.
  `classify_oidc_kind`); `support.rs::error_mode`. **`helpers.rs` is dead
  code — not declared in `mod.rs`.**
- `src/client.rs` — `ZitadelClient` (reqwest + bearer token). All methods
  return `serde_json::Value`/`Vec<Value>`; `api_request` is the single error
  boundary (401 → auth message; 403 + AUTHZ-Kl3p0 → session-token
  remediation; otherwise `API request failed ({status})`; never echoes body).
- `src/auth/` — `resolve_access_token` precedence: cli PAT → env PAT →
  config PAT → cli SA → env SA → config SA → token cache (refresh). Uses
  `starts_with("session token")` to detect OIDC sessions.
- `src/config.rs` — `AppConfig` TOML (XDG), redacted Serialize,
  `TemplatesFile`/`AppTemplate`/`UserTemplate` YAML,
  `save_to_canonical_path` (0600/0700).
- `src/output.rs` — `CommandEnvelope{command,ok,result}`, `print_human`
  (pretty-JSON anyway).
- `src/tui/` — ratatui App/types/state/render; `Record{id,name,kind,summary,
  detail,changed_at}`; `PendingAction` enum; `CanvasMode`.
- Untracked debris in the checkout: `lib/` (empty), `spec/` (empty),
  `.DS_Store`, `.ruby-lsp/`.

## Task 1: Hygiene + lib/bin split + CI trigger fix

Branch: `agent-cli/1-hygiene` (already checked out in the worktree).
Zero behaviour change — this is a preparation boundary.

1. Delete `src/conductor/helpers.rs` (dead code; not declared in
   `src/conductor/mod.rs`). Verify `cargo build` still passes — it is not
   compiled today.
2. `Cargo.toml`: remove `rpassword = "7"` (unused dependency). Run
   `cargo check` to regenerate `Cargo.lock`.
3. `.gitignore`: add `.ruby-lsp/` and `.claude/worktrees/` (`.DS_Store` is
   already ignored). Delete local debris dirs `lib/`, `spec/`, `.ruby-lsp/`,
   `.DS_Store` — these are untracked so nothing is committed for them.
4. lib/bin split: create `src/lib.rs` declaring `pub mod auth; pub mod cli;
   pub mod client; pub mod commands; pub mod conductor; pub mod config;
   pub mod oidc; pub mod output; pub mod token_cache; pub mod tui;
   pub mod tui_runtime;` plus `#[cfg(test)] pub mod test_support;`. Change
   `src/main.rs` `mod` declarations into `use zitadel_tui::…` imports (crate
   name `zitadel_tui`). Widen `pub(crate)` → `pub` ONLY where the
   compiler requires it for the bin target to call into the lib
   (e.g. `commands::execute_command`, `cli::{Cli,command_name}`,
   `config::AppConfig`, `conductor::TuiConductor`,
   `tui_runtime::run_tui`, `output::{CommandEnvelope,print_human}`,
   `TuiConductor::bootstrap`). Keep `mod tests` blocks as-is inside each
   file (they compile under `cfg(test)` within the lib target). Mechanical
   move only — no logic edits.
5. `.github/workflows/ci.yml`: change `pull_request: branches: [main]` to a
   bare `pull_request:` trigger (stacked PRs need checks on feature-branch
   bases).
6. Commit this plan file as `docs:` (it already exists at
   `docs/superpowers/plans/2026-10-29-agent-usable-cli.md`).

Tests:
- New `tests/bin_smoke.rs`: one test spawning
  `env!("CARGO_BIN_EXE_zitadel-tui")` with `--help` asserting exit status 0.
  No new dev-dependencies (std::process only).

Acceptance:
- `cargo fmt --check`, `cargo clippy --all-targets --all-features --
  -D warnings`, `cargo test`, `cargo build` all pass.
- `git show --stat` contains no logic edits; `cargo build` output identical
  behaviour (`zitadel-tui --help` works, `zitadel-tui` still launches TUI
  path).
- `src/conductor/helpers.rs` and `rpassword` are gone from the tree and
  `Cargo.lock` respectively.

## Task 2: Typed API layer `src/api/`

Branch: `agent-cli/2-typed-ops` (create from Task 1 tip).
Goal: `ZitadelClient` returns typed models instead of `serde_json::Value`;
raw payload retained on list/get models for the later `--full` flag.

1. Move `src/client.rs` → `src/api/client.rs`; create `src/api/mod.rs`
   (`pub mod client; pub mod models;`) and `src/api/models.rs`. Update
   `lib.rs` (`pub mod api` replacing `pub mod client`) and all imports
   (`crate::client::ZitadelClient` → `crate::api::client::ZitadelClient`).
2. `models.rs` — serde models (derive `Deserialize`, `Serialize`, `Clone`,
   `Debug`). Cover every field consumed anywhere in the codebase today plus
   projection fields. Use `#[serde(rename_all = "camelCase")]` and explicit
   `rename`/`alias` where Zitadel payloads vary (e.g. `WhoAmI.user` exposes
   `userId` or `id`; `preferredLoginName` or `userName` or `loginName`):
   - `Project { id, name, state, raw }`
   - `App { id, name, state, oidc_config: Option<OidcConfig>,
     api_config: Option<ApiConfig>, raw }` where
     `OidcConfig { client_id, auth_method_type, app_type, grant_types,
     redirect_uris }` and `ApiConfig { client_id, auth_method_type }`
   - `User { id, user_name, preferred_login_name, state,
     human: Option<HumanUser>, machine: Option<MachineUser>, raw }` where
     `HumanUser { profile { display_name }, email { email } }`,
     `MachineUser { name }`
   - `Idp { id, name, r#type, state, raw }` (`type` key in API)
   - `CreateAppResult { app_id, client_id, client_secret: Option<String>,
     raw }`, `CreateNativeAppResult` may reuse `CreateAppResult`
   - `CreateUserResult { user_id, raw }`, `SecretResult { client_secret,
     raw }`, `MemberResult { member_id, raw }`
   - `WhoAmI { user: WhoAmIUser, raw }`,
     `WhoAmIUser { user_id (aliases id,userId), preferred_login_name
     (aliases userName,loginName) }`
   - `raw: serde_json::Value` on every type, marked `#[serde(skip)]`, set by
     the client after decode (deserialize typed → attach the original
     `Value`). `state`/`type` and other optional-ish fields use
     `Option<String>` or `String` with `#[serde(default)]` as needed so
     unusual payloads still decode.
3. `client.rs` methods return: `whoami() -> WhoAmI`, `list_projects() ->
   Vec<Project>`, `get_default_project() -> Project`,
   `list_apps(project_id) -> Vec<App>`, `create_oidc_app(..) ->
   CreateAppResult`, `create_native_app(..) -> CreateAppResult`,
   `delete_app(..) -> ()` (204/empty), `regenerate_secret(..) ->
   SecretResult`, `list_users(limit) -> Vec<User>`, `create_human_user(..)
   -> CreateUserResult`, `import_human_user(..) -> CreateUserResult`,
   `grant_iam_owner(..) -> MemberResult`, `list_idps() -> Vec<Idp>`,
   `add_google_idp(..) -> Idp` (Zitadel returns the created idp object with
   `id`). `api_request` stays the single error boundary with identical
   messages (401 text, AUTHZ-Kl3p0 403 text, `API request failed
   ({status})`, decode error text, never echoing bodies). For 204/empty
   bodies on delete, return unit; otherwise deserialize `Value` first (keep
   it as `raw`), then deserialize into the typed model from that Value.
4. Port `conductor/records.rs` onto the typed models: `Record::from(&App)`,
   `Record::from(&User)`, `Record::from(&Idp)` (or `impl From<&App> for
   Record` in the api/models or a small `ops::records` module — implementer
   chooses location, but there must be exactly ONE record-projection
   implementation). `classify_oidc_kind` becomes `App::kind()` returning
   the same strings (`"device-code"`, `"native"`, `"public"`,
   `"confidential"`, `"api"`, `"oidc"`, `"unknown"`). `app_creation_summary`
   stays available (string helper) for conductor/consumers.
5. Update `commands/*` to consume typed results: `serde_json::to_value` of
   typed lists/results for envelope results (keeps identical JSON shapes
   today — verify with a test asserting `to_value` of a fixture `App`
   produces the same keys as before). `commands/auth.rs` `auth_status`
   continues to build its `json!` result from `WhoAmI.user` fields.
   `regenerate-secret`'s `--client-id` annotation applies to
   `SecretResult`→json (add `clientId` key as today).
   `apps::remember_device_client_id` reads `client_id` from
   `CreateAppResult`.
6. Update all existing tests: `mockito` bodies decode into typed models;
   assert typed fields. Error-path tests unchanged (status/text). Conductor
   tests: `map_app_record` etc. become `Record::from` tests — same
   assertions, typed inputs (build `App` via `serde_json::from_value` of the
   old fixture shapes).

Acceptance:
- Gates green. `grep -rn "serde_json::Value" src/api/` shows only `raw`
  fields and decode plumbing (no `Value` returns in public signatures).
- Every old fixture shape still round-trips: a decode test per model with a
  representative real-shaped payload (include one `App` with `apiConfig`,
  one `User` with `machine`, one minimal `Idp`).
- No behaviour change visible to CLI output yet (envelope results equal
  pre-change JSON — covered by the `to_value` equivalence test).

## Task 3: Unified `src/ops/` + rewire conductor and commands

Branch: `agent-cli/2-typed-ops` (same PR as Task 2).
Goal: exactly one implementation per operation, consumed by BOTH the
headless commands and the TUI conductor.

1. `src/ops/mod.rs` (`pub mod apps; pub mod users; pub mod idps;
   pub mod projects;` + shared items):
   - `resolve_project_id(client, cli_project: Option<String>,
     config_project: Option<String>) -> Result<String>` — single copy
     replacing `commands/shared.rs::resolved_project_id` and
     `conductor/runtime.rs::resolve_project_id` (both deleted).
   - Selector types + resolvers (used by Task 6 flags; define now):
     `AppSelector::{Id(String),Name(String)}`,
     `UserSelector::{Id(String),Username(String),Email(String)}`,
     `IdpSelector::{Id(String),Name(String)}` with
     `resolve_app(&client,&project,sel) -> Result<App>`,
     `resolve_user`, `resolve_idp` implementing exact-match lookup: `Id` →
     search the list for matching `id`; `Name`/`Username`/`Email` →
     exact-match on the corresponding model field(s) (`App.name`,
     `User.user_name`, `User.human.email.email`,
     `User.preferred_login_name` fallback for username; `Idp.name`). Zero
     matches → error `"... not found"`; >1 → error `"ambiguous ... matches
     N entries"` — a typed resolution outcome enum
     (`ResolveOutcome::{One(T),NotFound,Ambiguous(Vec<String>)}`) is
     preferred so Task 4 can map kinds cleanly.
2. `ops/apps.rs`: `list`, `get`, `create_oidc(client,&project,name,
   redirect_uris,public)`, `create_native(client,&project,name,device_code)`,
   `delete(client,&project,app_id)`, `regenerate_secret`, `quick_setup(
   client,&project,templates:&BTreeMap<String,AppTemplate>,names)` —
   single implementation; keep the ordered-index behaviour (results in
   requested order). `ops/users.rs`: `list`, `get`, `create_human`,
   `import_admin`, `grant_iam_owner`, `quick_setup(client,templates.users)`
   — one impl using the JoinSet-ordered approach (create user then grant if
   `admin`), results ordered by input index. `ops/idps.rs`: `list`, `get`,
   `configure_google`. `ops/projects.rs`: `list`, `default_id` (was
   get_default_project → id).
3. `commands/*.rs` become thin: build client (unchanged
   `shared::authenticated_client`/`resolved_host*`), call `ops::`, serialize
   result to `Value` for the envelope. `apps.rs::remember_device_client_id`
   + `config_with_device_client_id` stay command-side (config persistence).
   `commands/shared.rs` loses `resolved_project_id` (uses `ops::`).
4. `conductor/mutations.rs` rewires to `ops::`: form extraction stays in
   conductor (`form_value`/`optional_value`/`bool_value`/`split_csv`/
   `checkbox_enabled` in `forms.rs`), as do `CanvasMode` construction,
   `error_mode`, success-line formatting (keep message text identical to
   today — e.g. "Client ID: …", "This secret is only shown once.",
   quick-setup summary lines). `conductor/runtime.rs::refresh_runtime`
   uses `ops::resolve_project_id` and the Task-2 `Record::from` adaptors;
   `auth_label` precedence logic stays.
   `validate_and_save_setup`/`save_config_form` stay conductor-side
   (config, not API).
5. `auth/mod.rs`: `ResolvedAuth.source` becomes typed `AuthSource` enum
   (`CliPat, EnvPat, ConfigPat, CliServiceAccount, EnvServiceAccount,
   ConfigServiceAccount, SessionToken, RefreshedSessionToken`) with
   `Display` emitting the current label strings exactly (`"cli PAT"`,
   `"env PAT"`, `"config PAT"`, `"cli service account"`,
   `"env service account"`, `"config service account"`, `"session token"`,
   `"session token (refreshed)"`). `is_oidc_session()` matches the two
   session variants. `resolver.rs` returns variants; all `auth.source`
   comparisons/tests updated.
6. Move `create_quick_user` into `ops::users::quick_setup` internals;
   `app_creation_summary` lives wherever single-source record/line
   summaries do.

Acceptance:
- Gates green; `rg "resolve_project_id|resolved_project_id" src/` shows one
  definition; `rg "ZitadelClient" src/` shows direct client use only in
  `src/api/`, `src/ops/`, `commands/shared.rs::authenticated_client`,
  `commands/auth.rs` (whoami for status), `conductor/runtime.rs` (listing).
- `conductor/tests.rs` + `tui/tests.rs` unchanged-or-adapted and green —
  TUI-visible strings identical.
- No `commands/` or `conductor/` code builds API payloads anymore; they
  call `ops`.

## Task 4: Output contract + error model + exit codes

Branch: `agent-cli/3-contract` (create from Task 3 tip). BREAKING change
boundary (`feat!`).

1. `src/error.rs`: `ErrorKind` enum
   `{Usage,Auth,NotFound,Ambiguous,Api,Config,Io}` serializing snake_case;
   `CommandError { kind: ErrorKind, message: String, details:
   Option<serde_json::Value> }` (`thiserror` allowed or hand-rolled
   Display+Error). `From<anyhow::Error>` → `Api` kind fallback. Ops-level
   resolution outcomes map: `NotFound`→`NotFound`, `Ambiguous`→`Ambiguous`
   with `details.matches = [{id,name},…]`. Auth-resolution and credential
   errors map `Auth`. Config load/save errors map `Config`. Usage errors
   (missing required runtime values that clap can't express, e.g.
   `auth login` without client id, `--redirect-uris` missing without
   `--template`) map `Usage`.
2. `output.rs`: keep `CommandEnvelope{command,ok,result}` for success; add
   `ErrorEnvelope{command,ok:false,error{kind,message,details?}}` — or one
   serde-tagged shape; the printed wire format MUST be exactly:
   `{"command":"apps.list","ok":false,"error":{"kind":"not_found",
   "message":"…","details":{…}}}` (`details` omitted when None — use
   `skip_serializing_if`). Delete `print_human`, `format_human`,
   `empty_list_message`, `format_json_value` (human output dies; TUI is the
   human interface).
3. `cli.rs`: remove `--json` and `--once` args + `Cli.once`/`Cli.json`
   fields + `validate_cli`'s `--once` rule in `main.rs`. Replace
   `command_name` with `command_path(&Command) -> &'static str` returning
   dot-paths: `apps.list apps.create apps.create-native apps.delete
   apps.regenerate-secret apps.quick-setup apps.get users.list users.get
   users.create users.create-admin users.grant-iam-owner users.quick-setup
   idps.list idps.get idps.configure-google projects.list auth.login
   auth.logout auth.status config.show agent`. The Task-6/7 variants may
   not exist yet — implement `command_path` over the variants that exist in
   this task and extend it in Tasks 6–7.
4. `commands/` signatures: `execute_command` (and friends) return
   `Result<Value, CommandError>` — wrap ops errors by kind; `anyhow` may
   remain inside lower layers but the boundary produces `CommandError`.
5. `main.rs`: always print the envelope (`serde_json::to_string_pretty`) to
   stdout; on `Err` print error envelope and `std::process::exit(1)` (or
   return a non-zero `ExitCode`). Clap parse errors keep native exit 2 —
   verified by test, no code.
6. `commands/auth.rs`: delete `prompt_for_client_id`; when `--client-id`
   and config `device_client_id` are both absent → `Usage` error:
   `"client ID required: pass --client-id or set device_client_id in
   config"`. `auth login` result gains `verification_uri`,
   `verification_uri_complete` (when present), `user_code`. Keep the
   stderr URL/code lines for humans.
7. `rpassword`/`stdin`: assert no `stdin` reads remain anywhere in
   `src/` (grep in test or by inspection — report evidence).

Tests — new `tests/agent_contract.rs` (std::process + `mockito`, no new
dev-deps):
- success: `apps list` vs mock server → stdout parses as
  `{command:"apps.list",ok:true,result:[…]}`, exit 0 — WITHOUT `--json`.
- failure: server 500 → `ok:false`, `error.kind:"api"`, exit 1.
- not found: `apps delete --app-id missing` on empty app list →
  `kind:"not_found"` if resolution path used, else `api` — pin actual.
- usage: `auth login` with closed stdin + no client id/config →
  `kind:"usage"`, exit 1, returns quickly (no hang).
- clap: unknown flag → exit 2.
- envelope always JSON on stdout (no `--json` flag exists).

Acceptance: gates + contract tests green;
`zitadel-tui --json apps list` fails as unknown arg (breaking, intended).

## Task 5: Summary projections + `--full`

Branch: `agent-cli/3-contract` (same PR).
Goal: compact default output for list/get; `--full` emits raw payloads.

1. `cli.rs`: global `--full` bool flag (flag only, no env).
2. Models gain `projection()`/`summary()` (or a `project()` free fn) per
   Target design — exact wire shapes (serde field names snake_case):
   - `App` → `{id,name,client_id,kind,redirect_uris,state}` where `kind`
     reuses `App::kind()` (`"public"|"confidential"|"native"|
     "device-code"|"api"|"oidc"|"unknown"`), `client_id` from
     `oidc_config` else `api_config`, `redirect_uris` from `oidc_config`
     else `[]`
   - `User` → `{id,username,login_name,display_name,email,user_type,
     state}` (`user_type:"machine"|"human"`)
   - `Idp` → `{id,name,type,state}`
   - `Project` → `{id,name,state}`
3. `commands/` result construction: default `to_value(projections)`;
   `--full` → `to_value(raw)` (each model's `raw`). Applies to all list
   commands and (once Task 6 lands) `get` — wire it through
   `execute_command` signature (`args.full`) so Task 6 inherits it.
   `apps quick-setup`/`users quick-setup` keep full create-response
   results (they carry `clientSecret` — secrets in `result` are intended
   and documented).
4. Tests: fixture with 2 apps → default output has projection keys only;
   `--full` output equals mock's raw payload. Same for a user fixture.

Acceptance: default `apps list` item contains `client_id` + `kind` and no
`oidcConfig`; `--full` returns `oidcConfig`. Gates green.

## Task 6: New commands + selectors + `--limit` + secret env vars

Branch: `agent-cli/4-commands` (create from Task 5 tip). Feature boundary.

1. `cli.rs`:
   - New `Command::Projects(ProjectsCommand)` with `list` — wire
     `command_path` `projects.list`, `ops::projects::list` (same
     projection/`--full` path).
   - `apps get` (`--app-id`|`--name`, `ArgGroup required`),
     `apps delete` + `apps regenerate-secret` selector groups
     (`--app-id`|`--name`, required one-of), `users get`
     (`--user-id`|`--username`|`--email`), `users grant-iam-owner`
     (same group), `idps get` (`--idp-id`|`--name`).
   - `--limit <N>` u64 default 100, `value_parser` range 1..=1000 on
     `apps list`, `users list`, `idps list`, `projects list`; plumb to
     `client.list_users(limit)` and the `{query:{limit}}` bodies (add a
     `limit` param to `list_apps`/`list_projects`/`list_idps`).
   - `users create-admin --password` gains `env =
     "ZITADEL_USER_PASSWORD"`; `idps configure-google --client-secret`
     gains `env = "ZITADEL_GOOGLE_CLIENT_SECRET"` (clap `env` feature is
     already enabled).
2. `commands/`: selectors → `ops::resolve_*` outcomes → typed results;
   `not_found`/`ambiguous` kinds per Task 4 mapping. `get` commands emit
   the projection (or `raw` under `--full`) of the single resolved item.
3. `command_path` extended: `projects.list`, `apps.get`, `users.get`,
   `idps.get`.
4. Contract tests (`tests/agent_contract.rs` additions):
   - `apps get --name grafana` on 1-match fixture → projection + exit 0.
   - `apps delete --name grafana` resolves and DELETEs correct id path.
   - `--name` zero matches → `kind:"not_found"` + `details.lookup`.
   - `--name` two matches → `kind:"ambiguous"` + `details.matches` length 2.
   - `users grant-iam-owner --email a@b.c` resolves.
   - `apps list --limit 1` sends `"limit":1` in the search body (mock
     assert); `projects list` works.
   - Selector group violations (`apps delete` with neither flag, or both)
     → exit 2.

Acceptance: `projects list → apps get --name → apps delete --name` works
end-to-end vs mock without parsing raw payloads; gates green.

## Task 7: `agent` command + docs

Branch: `agent-cli/4-commands` (same PR).

1. `docs/AGENT.md` (new): agent guide — auth precedence & credential types
   (service-account recommended; PAT; session token limited), envelope
   schema, `error.kind` table (usage/auth/not_found/ambiguous/api/config/
   io), exit codes (0/1/2), complete command map with selectors and flags,
   notes: results may contain secrets for create/regen commands; no
   prompts exist anywhere; `auth login` requires `--client-id` or config;
   `--full` for raw payloads; `--limit`. Include 3 short recipes:
   bootstrap check (`auth status`), add OIDC app + capture clientSecret,
   create admin user + grant IAM_OWNER. Plain UK-adjacent technical
   English, scannable, no fluff.
2. `cli.rs`: `Command::Agent` (`agent`) — action-less subcommand.
   `command_path` → `agent`. `commands/` handles it by printing
   `include_str!("../docs/AGENT.md")` verbatim to stdout, exit 0 — NOT
   wrapped in the JSON envelope (it's a doc surface; the envelope applies
   to API commands). Implement via early return in `main.rs`/`commands`
   — document the choice in code minimally (one line).
3. `README.md`: update Global options (drop `--json`/`--once`; add
   `--full`, `--limit`, env vars for secrets), Command reference (new
   commands/selectors), Headless mode section → describe the always-JSON
   envelope + exit codes, new "Use by LLM agents" section pointing to
   `zitadel-tui agent` and `docs/AGENT.md`. Remove stale `--once`/`--json`
   examples. Keep the rest intact.
4. `AGENTS.md`: append a short "Agent-consumable CLI" pointer section
   (the binary's headless surface is JSON-envelope-first; see
   `docs/AGENT.md`; run `zitadel-tui agent`). Do NOT modify the existing
   landing-the-plane rules.
5. `markdown-lint-cli2` clean on changed md files (repo configures
   `.markdownlint-cli2.yaml`; run it if available, else keep style
   consistent with existing docs).

Acceptance: `zitadel-tui agent` prints the guide; README examples match
implemented flags exactly; gates green.

## Non-goals / out of scope

- MCP server, web UI, or any new runtime surface.
- Pagination beyond `--limit` (no offset support), machine-user filters.
- Migrating `serde_yaml` (deprecated) — file an issue.
- Splitting `tui/render.rs`/`tui/tests.rs` — file an issue if flagged.
- Changing TUI-visible strings or flows.
