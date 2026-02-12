# Bug Report: Dead Agents Stall Coordinator Pipeline

**Date**: 2026-02-12
**Severity**: Medium-High
**Component**: Coordinator Service / Agent Lifecycle

## Summary

When a spawned agent dies (process exits) while working on a task, the task remains in `in-progress` status with the dead agent still assigned. The coordinator does not automatically detect and cleanup dead agents, causing the entire pipeline to stall indefinitely.

## Reproduction Steps

1. Start coordinator service: `wg service start --max-agents 4`
2. Add a chain of dependent tasks: `wg add "Task A" && wg add "Task B" --blocked-by task-a`
3. Let coordinator spawn an agent for Task A
4. Wait for agent to complete Task A (or have it die mid-task)
5. Observe: Task B never gets spawned because Task A's agent is "dead" but task is still "in-progress"

## Expected Behavior

The coordinator should:
1. Detect when an agent process has exited (via PID check or heartbeat timeout)
2. Automatically unclaim tasks from dead agents
3. Re-spawn agents for tasks that become ready after unclaim

## Actual Behavior

- Coordinator logs show: `No ready tasks (done: X/Y)` indefinitely
- Agent registry shows dead agent still "assigned" to the task
- Task status remains `in-progress` despite agent being dead
- Pipeline completely stalls until manual intervention

## Evidence from Logs

```
2026-02-12T10:47:12.273Z [INFO] Coordinator tick #388 complete: agents_alive=0, tasks_ready=0, spawned=0
2026-02-12T10:48:12.344Z [INFO] Coordinator tick #389 complete: agents_alive=0, tasks_ready=0, spawned=0
... (hundreds of ticks with agents_alive=0, tasks_ready=0)
```

The coordinator sees 0 alive agents but reports 0 ready tasks because tasks are stuck in `in-progress`.

After manual `wg unclaim`:
```
2026-02-12T10:59:36.384Z [INFO] GraphChanged received, scheduling immediate coordinator tick
[coordinator] Spawning agent for: opt1-segment-cache-2
[coordinator] Spawned agent-23 (PID 127571)
```

## Current Workaround

Run a watchdog script that periodically cleans up dead agents:

```bash
#!/bin/bash
while true; do
    wg dead-agents --cleanup 2>/dev/null
    sleep 300  # Check every 5 minutes
done
```

This works but shouldn't be necessary.

## Proposed Fix

### Option 1: Coordinator Auto-Cleanup (Recommended)

Add dead agent detection to the coordinator tick:

```rust
// In coordinator tick loop
fn tick(&mut self) {
    // Check for dead agents first
    let dead = self.detect_dead_agents(threshold: Duration::from_secs(300));
    for agent in dead {
        self.unclaim_task(&agent.task_id);
        self.remove_agent(&agent.id);
        log::info!("Auto-cleaned dead agent {} from task {}", agent.id, agent.task_id);
    }

    // Then proceed with normal spawn logic
    let ready = self.get_ready_tasks();
    // ...
}
```

### Option 2: Heartbeat-Based Detection

Require agents to send periodic heartbeats. If no heartbeat received within threshold, mark as dead and unclaim.

### Option 3: Process Exit Handler

When spawning an agent, set up a callback for when the process exits:
- If exit code 0 and task done: normal completion
- If exit code 0 and task not done: unclaim (agent gave up)
- If exit code != 0: unclaim and optionally mark task as failed

## Impact

- **Autonomous pipelines break**: Any multi-step workflow stalls when an agent dies
- **User frustration**: Requires manual `wg dead-agents --cleanup` or `wg unclaim`
- **Silent failure**: No warning that pipeline is stalled; just "no ready tasks"

## Related Commands

- `wg dead-agents --check` - Detects dead agents (works correctly)
- `wg dead-agents --cleanup` - Unclaims tasks from dead agents (works correctly)
- `wg dead-agents --remove` - Removes dead agents from registry (works correctly)

The functionality exists; it just needs to be integrated into the coordinator loop.

## Environment

- workgraph version: 0.1.0
- OS: Linux 6.8.0-84-generic
- Rust version: (check Cargo.toml)
