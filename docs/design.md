# My Agent Assets V1 Design

## Architecture

The project is a Rust workspace:

- `crates/core`: asset discovery, planning, execution, backup, MCP compilation,
  config persistence, and platform filesystem operations.
- `crates/cli`: CLI argument parsing and human-readable output.

The future Tauri desktop app should call `crates/core` directly rather than
wrapping the CLI.

## Data Model

Asset IDs use `type:name`:

- `skill:review`
- `command:commit`
- `mcp:github`

Command assets only support `.md` in V1. The asset name removes `.md`, while the
registry stores the physical file name.

The asset center structure is:

```text
~/.my-agent-assets/
  assets/
    skills/
    commands/
    mcps/
  backups/
  config.yaml
  assets.yaml
  mounts.yaml
```

`maa init --apply` initializes Git in the asset center itself. The source code
checkout is not part of the product's asset-sync repository.

## Plan / Apply

The core returns a plan for mutating workflows. A plan lists actions, source
paths, target paths, and risk. The executor only mutates files when `apply` is
explicitly requested.

Adoption uses:

```text
backup -> copy into asset center -> verify -> replace runtime path with symlink -> verify
```

This avoids direct moves and gives restore a concrete backup manifest.

## MCP Extract / Compile

MCP is a logical asset, not a file asset. Extraction reads:

- `~/.claude.json.mcpServers`
- `~/.claude.json.projects["<project_path>"].mcpServers`
- `<project>/.mcp.json.mcpServers`

Each server becomes:

```text
assets/mcps/<name>.json
asset id: mcp:<name>
```

The file contains only the server config body. Compilation writes managed MCP
servers back into the correct scope while preserving unknown top-level JSON
fields and unmanaged MCP servers.

## Cross Platform Notes

Skill and Command assets use symlinks. On Windows, symlink creation may require
Developer Mode or administrator privileges. V1 reports a structured error and
does not copy as a fallback, preserving the asset center as the single source of
truth.

## Git

Git operations are explicit:

```bash
maa sync pull
maa sync push
```

`scan` never performs hidden network or Git operations.
