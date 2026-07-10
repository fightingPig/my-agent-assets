# My Agent Assets Final Product Model

> Canonical product model for the final V1 implementation.
>
> This document supersedes provider-specific and read-only milestone descriptions when they conflict with the final goal in `my_agent_assets_final_goal.md`.

## Product Identity

My Agent Assets is a local-first desktop application and CLI for managing reusable agent assets.

The product model is:

```text
one canonical asset center
+ multiple runtime sources
+ multiple compatible mount targets
```

Claude Code and Codex are not separate asset centers. They are runtime providers with source discovery and target adapter rules.

The fixed canonical asset center is:

```text
~/.my-agent-assets
```

It is the single source of truth for managed assets.

## Canonical Assets

Each managed asset has one canonical copy:

```text
~/.my-agent-assets/assets/skills/<name>/
~/.my-agent-assets/assets/commands/<name>.md
~/.my-agent-assets/assets/mcps/<name>.json
```

Asset identity is:

```text
asset type + name
```

The source provider, source path, project name, and mount target do not participate in asset identity or canonical naming.

The asset center must not be divided into provider-owned trees such as `assets/claude/` and `assets/codex/`.

## Runtime Sources

A runtime source is a location from which an unmanaged asset can be discovered and imported.

Supported final V1 sources include:

- Claude Code user, project, local, and explicitly authorized custom sources.
- Codex user, project, and explicitly authorized custom sources.
- Compatible custom Skill directories.
- Claude-compatible MCP JSON configurations.
- Codex-compatible MCP TOML configurations.

Scanning a source is read-only. Import copies or extracts a selected source asset into the canonical asset center. Import does not automatically write to another runtime configuration.

After import, source metadata may be retained only in machine-local operation history for diagnostics. It must not affect canonical identity, compatibility, Git synchronization, or portable backups.

## Mount Targets

A mount target is an explicitly registered and authorized runtime destination.

Apply operations accept a `targetId`; the Rust core resolves and validates the corresponding path and adapter from the machine-local Target Registry. The frontend must not provide an arbitrary path directly to a write operation.

Target bindings are machine-local and are not part of the Git-synchronized canonical asset.

## Compatibility Matrix

| Asset type | Claude Code target | Codex target | Custom target |
|---|---:|---:|---:|
| Skill | Supported | Supported | Compatible Skill directory only |
| Command | Supported | Not supported | Claude-compatible Command directory only |
| MCP Server | Claude JSON adapter | Codex TOML adapter | Explicit Claude JSON or Codex TOML adapter only |

Final Codex support therefore includes:

- Discovering compatible Codex Skills and MCP servers.
- Importing compatible Codex Skills and MCP servers into the same canonical asset center used by Claude assets.
- Mounting a canonical Skill to compatible Codex user or project Skill targets.
- Compiling a canonical MCP server into compatible Codex user or project TOML targets.

Final Codex support explicitly excludes:

- Codex Command assets or Command targets.
- Automatic Command-to-Skill conversion.
- Codex `AGENTS.md` asset management.
- Codex OAuth token reading, storage, refresh, or management.

An MCP server that requires OAuth may expose a warning or manual setup guidance, but My Agent Assets must not manage the token.

## Skill And Command Mounting

Canonical Skills and Commands are mounted rather than copied:

- macOS/Linux Skill and Command targets use symbolic links.
- Windows Skill directories use directory junctions.
- Windows Command files use file symbolic links.
- Windows permission failure must be reported and must not silently fall back to copying.
- Hard links are not a supported mount mechanism.

Unmount removes only a link or junction that still points to the expected canonical asset. It must not delete a user-replaced runtime file.

Commands are never mounted to Codex targets.

## MCP Canonical Model

MCP assets use a file-based canonical model:

```text
~/.my-agent-assets/assets/mcps/<name>.json
```

There is no SQLite database, MCP table, DAO layer, or database cache.

The canonical JSON is the synchronized source of truth. Claude and Codex live configuration files are compiled artifacts.

Target enablement and synchronization state belong to machine-local target bindings, not to the canonical MCP JSON.

MCP implementation is separated into:

```text
file repository
→ MCP service
→ Claude JSON renderer / Codex TOML renderer
```

The service coordinates import, validation, target binding, synchronization, removal, backup, and rollback. It must not directly implement JSON or TOML patch details.

### Claude Renderer

The Claude renderer patches only the selected `mcpServers` container:

- User: root `mcpServers` in `~/.claude.json`.
- Local: `projects["<canonical project path>"].mcpServers` in `~/.claude.json`.
- Project: root `mcpServers` in `<project>/.mcp.json`.

It preserves all unrelated fields and servers. It never symlinks the whole JSON file.

### Codex Renderer

The Codex renderer uses `toml_edit` to patch only `[mcp_servers]` in the selected user or project `config.toml`.

It preserves unrelated configuration, servers, and comments. Canonical `headers` compile to Codex `http_headers`. Removal also cleans the legacy invalid `[mcp.servers.<name>]` form when present.

It never symlinks the whole TOML file.

### Import And Synchronization

Importing a Claude or Codex MCP entry:

1. Reads the selected live entry.
2. Converts it to the canonical MCP model.
3. Stores the canonical asset.
4. Records the source target state locally.

Import must not automatically write back to the source or distribute the entry to other targets. Live configuration changes happen only after an explicit upsert, toggle, mount, unmount, delete, or sync operation.

Disabling a target removes only that server from that target. Deleting an MCP asset removes it from all previously enabled targets before deleting canonical storage.

## Machine-local State

The file-based state model is:

```text
assets.yaml   # synchronized canonical asset index
config.yaml   # machine-local settings
targets.yaml  # machine-local authorized targets
mounts.yaml   # machine-local asset-target bindings and state
```

Every state file has a `schemaVersion`. Unknown newer versions block writes. Corrupt state is diagnosed rather than silently replaced.

`config.yaml`, `targets.yaml`, and `mounts.yaml` are not Git-synchronized.

## Git And Backups

Git synchronization uses an allowlist. Portable content includes:

- `assets/`
- `assets.yaml`
- portable canonical backups
- required schema/version metadata

Machine-local state, logs, locks, journals, live-config backups, and target bindings are excluded.

Local backups protect runtime writes on the current device. Portable backups contain canonical asset snapshots suitable for Git synchronization.

The application provides backup history, file-location access, and a manual restore guide. It does not expose a general automatic Restore operation. Users may preview and explicitly delete one selected backup; this is blocked for a backup referenced by an incomplete operation journal. Automatic rollback of an unfinished operation journal is transaction recovery, not user-facing historical restore.

Canonical MCP files may contain sensitive values in this version. Remote Push is therefore blocked unless the configured GitHub repository is authenticated and verified as `PRIVATE` immediately before Push. The application does not provide GitHub login or OAuth.

## Safety And Execution Boundary

All business logic lives in the shared Rust core:

- Scan, import, adopt, mount, unmount, delete, backup, transaction recovery, target validation, and MCP rendering.
- Tauri commands adapt DTOs and manage desktop process state.
- CLI commands parse terminal input and call the same Rust core.
- React never manipulates the filesystem or implements provider-specific business rules.

Every write operation requires a preview. Apply revalidates preview fingerprints, target authorization, compatibility, and current file state under path locks.

Writes use backup, operation journal, same-directory temporary files, flush/sync, and atomic replacement. A stale preview or partial failure must not be reported as success.

High-risk operations use prominent impact messaging and explicit button confirmation. Typed confirmation text is not required.

## Product Boundaries

- Local-first; no login, account center, cloud account, team, billing, or subscription UI.
- No Codex OAuth management.
- No Codex Commands.
- No automatic merge, rebase, reset, stash, or force push.
- No automatic Git synchronization during scan, import, mount, or application startup.
- No automatic creation of user-level Claude or Codex configuration before that runtime has been initialized.
- No automatic repair of canonical registry/content inconsistencies.
- No general-purpose Skill or Command editor in V1.
- Existing canonical asset IDs are not directly renamed in V1.
- Simplified Chinese is the only V1 UI language.

## Historical Documentation

The following documents describe completed milestones and remain useful as implementation history, but they do not define final product scope:

- `docs/codex-readonly-support.md`
- `docs/desktop-static-gui-freeze.md`
- `docs/read-only-integration.md`

Where those documents describe Codex as permanently read-only, typed `APPLY` confirmation, automatic Restore workflows, provider-owned assets, or other superseded constraints, this document and `my_agent_assets_final_goal.md` take precedence.
