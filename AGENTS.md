# Agent Instructions

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:

   ```bash
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```

5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**

- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

## Agent-consumable CLI

The binary's headless surface is JSON-envelope-first: every subcommand prints
`{"command":…,"ok":…,"result"|"error":…}` on stdout and exits 0 (success), 1
(command error — branch on `error.kind`), or 2 (arg parse failure). No command
prompts. The full machine-readable contract — command map, selectors, error
kinds, auth precedence, recipes — lives in `docs/AGENT.md`; run
`zitadel-tui agent` to print it verbatim.
