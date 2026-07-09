> **Historical milestone — superseded for final product scope**
>
> This document records the earlier Codex read-only milestone. It is not the current product contract.
>
> The final model uses **one canonical asset center, multiple runtime sources, and multiple compatible mount targets**. Codex-compatible Skills and MCP servers will support discovery, import into that shared canonical asset center, and compatible user/project mounting. Codex Commands, Command targets, `AGENTS.md` asset management, and Codex OAuth token management remain prohibited.
>
> The dedicated Desktop `list_codex_skills` and `list_codex_mcp_servers`
> commands have since been removed. Production Codex views use shared-core
> `discover_runtime_sources` filtered by `provider: "codex"`.
> Commands remain visible as canonical assets regardless of provider selection;
> Codex Command targets are still prohibited by compatibility validation.
>
> See `docs/final-product-model.md` and `my_agent_assets_final_goal.md` for authoritative scope. The historical implementation details below remain unchanged as milestone evidence.

# Codex Read-only Provider Support

My Agent Assets supports Codex as a read-only Asset Center provider. The existing Provider switch selects either `Claude Code` or `Codex` without changing the desktop window shell.

## Supported Assets

Codex currently exposes:

- Skills
- MCP Servers

At this historical milestone, Commands were hidden while the Codex provider was
selected. The final product keeps canonical Commands visible while still
prohibiting Codex Command targets. Codex AGENTS.md files, custom commands, and
OAuth token management remain unsupported.

## Skill Discovery

`list_codex_skills` reads:

- `$HOME/.agents/skills/<name>/SKILL.md`
- `.agents/skills/<name>/SKILL.md` from the current project and its ancestors up to the repository root
- `/etc/codex/skills/<name>/SKILL.md` when the system directory is readable

A directory is returned only when it contains `SKILL.md`. The result includes name, description, scope, path, modified time, `scripts/`, `references/`, `assets/`, `agents/openai.yaml`, symlink target, and warnings.

Symlinked Skill directories are identified and read only. No write operation follows or replaces them.

## MCP Discovery

`list_codex_mcp_servers` reads:

- `$HOME/.codex/config.toml`
- `<project-root>/.codex/config.toml`

It parses `[mcp_servers.<name>]` tables and reports scope, configuration path, transport, command, arguments, URL, enabled state, enabled/disabled tools, approval mode, and warnings.

Servers that appear to require authentication are shown with a warning. My Agent Assets does not read, store, refresh, or manage OAuth tokens.

## Safety Boundary

Both commands are read-only:

- They create no directories or files.
- They do not modify TOML.
- They do not import or mount Codex assets.
- They do not route Codex assets into Claude apply workflows.
- Missing paths and invalid TOML produce empty results or warnings rather than writes.

Production UI never substitutes Claude or Codex sample rows when discovery is empty or fails. Static fixtures are restricted to tests, Visual QA, and explicit demo mode.
