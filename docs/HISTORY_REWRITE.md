# History rewrite runbook (NOT RUN)

The cleanup commit removed build artifacts from the current tree, but the
~1.2 GB of `crates/dots-family-ebpf/target/**` blobs remain in git history,
so fresh clones stay large. Only the repository owner may decide to run this.
**This runbook was not executed.**

## Before you start
1. Announce a freeze: no pushes to any branch during the rewrite.
2. Back up the repository: `git clone --mirror <url> /backup/dots-family-mode.git`
3. Have every collaborator push; unpushed local commits will not survive.
4. Install `git-filter-repo` (it is not part of git core).

## Rewrite
```bash
git filter-repo --path crates/dots-family-ebpf/target --invert-paths
```
Repeat with extra `--path` arguments if other large paths are found, then
check the result with `git count-objects -vH`.

## Publish
```bash
git remote add origin <url>      # filter-repo drops the remote
git push --force --all origin
git push --force --tags origin
```
`--force` is mandatory: every rewritten commit gets a new hash.

## Consequences
- Every clone must re-fetch, not pull: `git fetch --all && git reset --hard origin/<branch>`, or delete and re-clone.
- Open pull requests, forks and CI caches point at old hashes and must be recreated.
- Rewrite `review-fixes` from current `main` only after the owner confirms.

## Status
Not run. Guidance for the repository owner.
