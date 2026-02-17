# Bug: Spawned Claude agents fail with nesting protection error when coordinator runs inside Claude Code

## Summary

When `wg service start` (or `wg spawn --executor claude`) is invoked from within a Claude Code session, all spawned Claude child processes immediately fail because they inherit the `CLAUDECODE=1` environment variable from the parent session. Claude Code's nesting protection detects this variable and refuses to start.

## Steps to Reproduce

1. Open a Claude Code session (interactive or via any mechanism that sets `CLAUDECODE=1` in the environment).
2. Initialize a workgraph with at least one ready task.
3. Run `wg service start` (or `wg spawn <task-id> --executor claude`).
4. Observe the spawned agent's output log at `.workgraph/agents/agent-N/output.log`.

## Expected Behavior

The spawned Claude agent process starts successfully and begins working on the assigned task.

## Actual Behavior

The spawned Claude agent immediately exits with:

```
Error: Claude Code cannot be launched inside another Claude Code session.
Nested sessions share runtime resources and will crash all active sessions.
To bypass this check, unset the CLAUDECODE environment variable.
```

The wrapper script then detects the non-zero exit code and marks the task as failed via `wg fail`.

## Root Cause Analysis

Claude Code sets the environment variable `CLAUDECODE=1` when it starts a session. This variable is inherited by all child processes. When workgraph spawns a new `claude` process, it inherits the full parent environment (including `CLAUDECODE=1`), which triggers Claude Code's built-in nesting protection.

The problem exists in two code paths:

### Primary path: wrapper script generation in `src/commands/spawn.rs`

Both the `run()` function (lines 245-276) and the `spawn_agent()` function (lines 529-560) generate a `run.sh` wrapper script that directly invokes the claude command without clearing the `CLAUDECODE` environment variable. The generated script looks like:

```bash
#!/bin/bash
TASK_ID="<task-id>"
OUTPUT_FILE="<output-file>"

# Run the agent command
cat '<prompt-file>' | 'claude' '--print' '--verbose' ... >> "$OUTPUT_FILE" 2>&1
EXIT_CODE=$?
...
```

The wrapper script is written at line 279 (`run()`) and line 564 (`spawn_agent()`):
```rust
let wrapper_path = output_dir.join("run.sh");
fs::write(&wrapper_path, &wrapper_script)
```

Although the spawning code at lines 291-298 (`run()`) and 575-582 (`spawn_agent()`) sets explicit environment variables via `cmd.env()`, it only adds variables -- it does not remove inherited ones like `CLAUDECODE`.

### Secondary path: direct spawn in `src/service/claude.rs`

The `ClaudeExecutor::spawn()` method (lines 146-207) uses `Command::new("stdbuf")` to spawn the claude process directly. While it sets environment variables from config via `cmd.env()` (line 182), it does not remove `CLAUDECODE` from the inherited environment.

## Suggested Fix

### Option A: Unset `CLAUDECODE` in the generated wrapper script (recommended)

In `/home/erik/workgraph/src/commands/spawn.rs`, modify the wrapper script template in both `run()` (around line 246) and `spawn_agent()` (around line 530) to include `unset CLAUDECODE` before the inner command:

```rust
let wrapper_script = format!(
    r#"#!/bin/bash
TASK_ID="{task_id}"
OUTPUT_FILE="{output_file}"

# Prevent Claude Code nesting protection from blocking child agents
unset CLAUDECODE

# Run the agent command
{inner_command} >> "$OUTPUT_FILE" 2>&1
EXIT_CODE=$?
...
```

### Option B: Remove `CLAUDECODE` from the child process environment

In `/home/erik/workgraph/src/commands/spawn.rs`, after building the `Command` (line 291 in `run()`, line 575 in `spawn_agent()`), add:

```rust
cmd.env_remove("CLAUDECODE");
```

And in `/home/erik/workgraph/src/service/claude.rs`, after building the `Command` (line 163), add:

```rust
cmd.env_remove("CLAUDECODE");
```

### Recommendation

Option A is preferred because the wrapper script is the actual process that invokes `claude`, and the `unset` will apply regardless of how the wrapper is launched. Option B would also work but requires changes in more locations and does not protect against cases where the wrapper script is re-run manually.

Both options should ideally be applied together for defense in depth.

## Affected Files

| File | Lines | Description |
|------|-------|-------------|
| `src/commands/spawn.rs` | 245-276 | `run()` wrapper script template |
| `src/commands/spawn.rs` | 291-298 | `run()` Command construction (inherits env) |
| `src/commands/spawn.rs` | 529-560 | `spawn_agent()` wrapper script template |
| `src/commands/spawn.rs` | 575-582 | `spawn_agent()` Command construction (inherits env) |
| `src/service/claude.rs` | 146-207 | `ClaudeExecutor::spawn()` direct Command construction |

## Severity

**High** -- This bug makes it impossible to use `wg service start` or `wg spawn --executor claude` from within any Claude Code session, which is the primary intended workflow described in the project's CLAUDE.md.
