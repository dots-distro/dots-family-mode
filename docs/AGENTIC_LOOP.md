# agentic-loop on this repository

Binary `/home/shift/code/agentic-loop/.agentic-loop/cargo-target/debug/agentic-loop`.
Licence: a **starter dev licence**; source
`/home/shift/code/agentic-loop/.dev-license/starter.jwt.env`. Never print, log,
commit or paste it.

## Environment
```
AGENTIC_LOOP_REPO_PATH=/tmp/opencode/dots-family-mode
AGENTIC_LOOP_WORKFLOWS_PATH=/home/shift/code/agentic-loop/examples/workflows
CARGO_TARGET_DIR=/tmp/opencode/dots-family-mode/target
```
`AGENTIC_LOOP_REPO_PATH` selects the repository and its config, so the store is
`/tmp/opencode/dots-family-mode/.agentic-loop/data`, not the agentic-loop one.
`AGENTIC_LOOP_WORKFLOWS_PATH` supplies `worktree_fix.workflow`;
`CARGO_TARGET_DIR` reuses the main checkout's compiled dependencies.

## One process at a time
The embedded store takes `<repo>/.agentic-loop/data/LOCK`; a second
`agentic-loop run` fails while the first holds it. Check
`pgrep -af 'agentic-loop run'` first. A leftover `LOCK` file is harmless; a live
process is not. Never delete a live holder's lock.

## Create a task with a VERIFY line
Use the binary's MCP tools (`engram_store`) or the daemon/API against the same
`AGENTIC_LOOP_REPO_PATH`; the `engram` CLI defaults to a different (`.engram`)
store. Create one `task` with status `todo`. The description names one
behaviour, one file, one test and one command:
`VERIFY: cargo test -p <crate> <test_name>`. A task without `VERIFY:` is
skipped. The task id is the id passed to the workflow; never create a second one.

## Run it
```
cd /tmp/opencode/dots-family-mode
nix develop --command "$BIN" run worktree_fix "$DESCRIPTION" \
  --var task_id=<uuid> --var repo_url=/tmp/opencode/dots-family-mode \
  --var base_branch=review-fixes \
  --var verify_command='cargo test -p <crate> <test_name>' \
  -p openrouter -m deepseek/deepseek-v4.1-flash-1017
```
States `create -> inspect -> fix -> check -> verify -> review`; the worktree is
`.workspaces/<task-id>` cut from `review-fixes`; nothing is committed, merged or
pushed. `run-worktree-fix.sh` assumes engram git-refs storage, so call the
binary directly for this SurrealKV store. The runner executes `verify_command`
after the verify stage; a non-zero exit fails the stage. A green compile is not
a passing check.

## Evidence
- task status/description/failure reason: `<repo>/.agentic-loop/data`;
- run log: the shell redirect, e.g. `/tmp/opencode/loop-on-dots.log`;
- evidence: `<repo>/.agentic-loop/evidence.jsonl`; incidents in
  `<repo>/.agentic-loop/incidents.jsonl`;
- diff: `git -C .workspaces/<task-id> diff`.
