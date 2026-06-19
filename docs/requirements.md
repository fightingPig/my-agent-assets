# My Agent Assets V1 Requirements

## Background

Claude users accumulate local assets over time: Skills, Commands, and MCP server
configuration. These assets may live in user-level Claude runtime files or in
project-level runtime files. They are hard to reuse across projects, migrate
between machines, and sync through Git when they stay scattered in runtime
locations.

My Agent Assets V1 introduces an asset center as the single source of truth.
Runtime locations become materialized views through symlink mounting or MCP JSON
compilation.

## Goals

- Import existing Claude assets into an asset center.
- Mount assets from the center into user or project Claude runtimes.
- Support Git-based sync through explicit commands.
- Preserve Claude behavior after adoption.
- Provide a core model that can later power a Tauri desktop GUI.
- Avoid touching real user Claude data during automated verification.

## V1 Scope

- Project name: `my-agent-assets`
- CLI binary: `maa`
- Default asset center: `~/.my-agent-assets`
- Runtime provider: Claude only
- Asset types:
  - Skill: `.claude/skills/<name>/`
  - Command: `.claude/commands/<name>.md`
  - MCP: `mcpServers.<name>` from Claude MCP config sources

## MCP Sources

V1 supports these Claude MCP scopes:

- `user`: `~/.claude.json.mcpServers`
- `local`: `~/.claude.json.projects["<project_path>"].mcpServers`
- `project`: `<project>/.mcp.json.mcpServers`

V1 does not read or write `~/.claude/mcp.json`.

## CLI

```bash
maa init
maa scan
maa scan --apply
maa list
maa status
maa doctor
maa mount <name> --type skill|command|mcp
maa unmount <name> --type skill|command|mcp
maa remove <name> --type skill|command|mcp
maa restore <backup-id>
maa sync pull
maa sync push
```

All mutating operations default to plan output. `--apply` performs changes.

MCP conflicts require an explicit decision. When an incoming MCP has the same
name as an existing asset but different JSON, the plan must show both original
JSON values and require skip, overwrite, or rename.

`maa init --apply` creates the asset center and initializes Git inside that
asset center. It must not initialize Git in the runtime directory or source code
checkout.

## Non Goals

- GUI implementation
- Multi-agent support beyond Claude
- AI deduplication
- Automatic conflict resolution
- Cloud service or marketplace
- Hidden Git pull/push inside scan

## Acceptance Criteria

- `scan` does not mutate runtime files.
- `scan --apply` imports fake runtime assets into a fake asset center.
- Skill and Command runtime paths become symlinks after adoption.
- MCP assets are extracted into the asset center without deleting or immediately
  rewriting the original Claude JSON source.
- Backup manifests are created for mutating adoption.
- `restore --apply` restores fake runtime paths from backup.
- Tests and e2e scripts never access real `~/.claude`, `~/.claude.json`, or
  `~/.my-agent-assets`.
- Restore rejects unsafe backup ids and manifests that reference paths outside
  allowed runtime roots.
