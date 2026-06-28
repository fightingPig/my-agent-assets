# Codex Read-only Provider Support

My Agent Assets supports Codex as a read-only Asset Center provider. The existing Provider switch selects either `Claude Code` or `Codex` without changing the desktop window shell.

## Supported Assets

Codex currently exposes:

- Skills
- MCP Servers

Commands are hidden while the Codex provider is selected. Codex AGENTS.md files, custom commands, asset import, mount, adoption, configuration writes, and OAuth token management are not supported.

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
