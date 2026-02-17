# Bug Report: Multiple Daemon Instances Stack Up, Preventing Clean Shutdown

## Summary

`wg service start` spawns a new daemon process without killing existing ones. Over a session, multiple daemon instances accumulate, all independently spawning agents. `wg service stop` only kills the most recent daemon PID, leaving the others running. This makes it impossible to cleanly stop the service.

## Severity

**High** - Users lose control of agent spawning. In our case, 5 stacked daemons spawned 470 agents over ~8 hours and could not be stopped without manually hunting and killing PIDs.

## Steps to Reproduce

1. `wg service start --max-agents 5`
2. `wg service stop` (agents continue running)
3. `wg service start --max-agents 5` (new daemon, old one still alive)
4. Repeat steps 2-3 several times
5. `ps aux | grep "wg.*daemon"` - observe multiple daemon processes

## Expected Behavior

- `wg service start` should refuse to start if a daemon is already running, OR kill existing daemons first
- `wg service stop` should kill ALL daemon processes for this workgraph directory, not just the latest PID
- Agents spawned by the service should be killed when the service stops (or at minimum, there should be a `--kill-agents` flag)

## Actual Behavior

- Each `wg service start` spawns a new daemon process alongside existing ones
- Each daemon independently polls for ready tasks and spawns agents
- `wg service stop` only kills the PID stored in the socket/pidfile, leaving older daemons running
- "agents continue running" message after stop, with no option to kill them
- Orphaned daemons keep spawning new agents indefinitely

## Observed in Production

During a phonon development session:
```
PID 1787    - from Feb 16 (original)
PID 161595  - from Feb 13 (3 days old!)
PID 1595461 - max-agents 3
PID 2084484 - max-agents 8
PID 2641130 - max-agents 4
```

5 daemons running simultaneously, 470 total agents spawned, could only be stopped by manually killing each PID.

## Suggested Fix

### Option A: PID lockfile (simplest)
In `wg service start`:
1. Check for existing PID file at `.workgraph/service/daemon.pid`
2. If PID file exists and process is alive, refuse to start (or kill it first with `--force`)
3. Write new PID file on startup
4. In `wg service stop`, read PID file, kill that process, also `pkill` any matching daemon processes for this directory

### Option B: Socket-based exclusion
The daemon already binds a Unix socket (`.workgraph/service/daemon.sock`). Use `SO_REUSEADDR` or check socket liveness before starting a new daemon. If the socket is active, refuse to start.

### Option C: Process group
Spawn the daemon and all its child agents in a process group. `wg service stop` sends SIGTERM to the entire group, ensuring everything dies.

## Additional Issue: `wg service stop` Should Have `--kill-agents` Flag

Currently "agents continue running" after stop. There should be:
- `wg service stop` - stop daemon, let agents finish (current behavior)
- `wg service stop --kill` - stop daemon AND kill all spawned agents

## Environment

- workgraph v0.1.0
- Linux 6.17.0-14-generic
- Multiple concurrent Claude Code sessions
