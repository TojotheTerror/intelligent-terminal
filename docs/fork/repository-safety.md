# Repository Safety

This guide is the fork-safety checklist for Intelligent Terminal work. It applies to humans, Codex workers, Symphony workers, and other automation.

## Non-Negotiable Rule

Do not write to upstream repositories.

No worker may push branches or tags, create releases, open or edit PRs, open or edit issues, or perform any other write action in:

- `microsoft/intelligent-terminal`
- `openai/symphony`
- any remote named or treated as `upstream`

## Repository Boundary

| Repository | Role | Write policy |
|---|---|---|
| `TojotheTerror/intelligent-terminal` | Writable Intelligent Terminal fork | Allowed write target for this project when explicitly needed |
| `TojotheTerror/symphony` | Related writable repository | Writable only when the task explicitly assigns Symphony work |
| `microsoft/intelligent-terminal` | Upstream Intelligent Terminal repository | Read-only |
| `openai/symphony` | Upstream Symphony repository | Read-only |

Do not touch Symphony during Intelligent Terminal work unless the task explicitly assigns Symphony files or repositories.

## Remote Layout

Recommended remotes:

```text
origin    https://github.com/TojotheTerror/intelligent-terminal.git  fetch/push
upstream  https://github.com/microsoft/intelligent-terminal.git      fetch
upstream  DISABLED                                                   push
```

Inspect the current layout before any write-capable Git command:

```bash
git status --short --branch
git remote -v
git remote get-url origin
git remote get-url --push origin
git remote get-url upstream
git remote get-url --push upstream
git config --get-all remote.upstream.pushurl
```

Disable upstream pushes:

```bash
git remote set-url --push upstream DISABLED
```

If a second upstream-style remote exists, disable its push URL too:

```bash
git remote set-url --push <remote-name> DISABLED
```

## GitHub CLI Rules

Always pass `-R owner/repo` for GitHub CLI commands that can read or write repository state. For this project, use the writable fork unless the command is explicitly read-only and intentionally targets an upstream repository.

Safe target examples:

```bash
gh repo view TojotheTerror/intelligent-terminal
gh pr view 1 -R TojotheTerror/intelligent-terminal
gh pr create -R TojotheTerror/intelligent-terminal
gh issue view 123 -R TojotheTerror/intelligent-terminal
```

Forbidden patterns:

```bash
gh pr create
gh pr edit
gh issue create
gh release create
gh repo edit
```

Those commands are forbidden when they omit `-R`, because `gh` may infer an upstream repository from the current checkout, default remote, or previous state.

Never run a GitHub CLI write action with:

```bash
-R microsoft/intelligent-terminal
-R openai/symphony
```

## Local Pre-Push Safeguard

The tracked template `.githooks/pre-push` blocks pushes unless the push URL is exactly:

```text
https://github.com/TojotheTerror/intelligent-terminal.git
```

Enable it locally:

```bash
git config --local core.hooksPath .githooks
```

Check syntax without pushing:

```bash
sh -n .githooks/pre-push
```

The hook intentionally blocks SSH remotes, URL variants, token-bearing URLs, upstream remotes, and every repository except the approved Intelligent Terminal fork. That strictness is deliberate.

## Worker Safety Checklist

Before work starts:

- Confirm the current repository is `TojotheTerror/intelligent-terminal`.
- Confirm the branch is the task branch, not `main` or an upstream-tracking branch.
- Run `git status --short --branch`.
- Run `git remote -v`.
- Confirm `origin` push targets `https://github.com/TojotheTerror/intelligent-terminal.git`.
- Confirm every upstream push URL is `DISABLED`.
- Confirm the task does not require Symphony writes.
- Confirm no local-only agent state such as `.codex/` or `.serena/` needs to be read, modified, staged, deleted, ignored, or excluded.

Before a write-capable command:

- Re-check the remote target.
- Use explicit repository targets for `gh`.
- Stop if the target is ambiguous.
- Stop if a command would write to `microsoft/intelligent-terminal` or `openai/symphony`.

Before final handoff:

- Report changed files.
- Report remotes.
- Report whether upstream writes were attempted.
- Report whether forbidden files were touched.
- Report whether local-only agent state was untouched.

## Hard-Fail Conditions

Stop immediately if any of these are true:

- The current repository is not the expected Intelligent Terminal fork.
- The current branch is wrong for the assigned task.
- `origin` does not point to `TojotheTerror/intelligent-terminal`.
- Any upstream remote has a real push URL.
- A GitHub CLI write action lacks `-R TojotheTerror/intelligent-terminal`.
- A command would write to `microsoft/intelligent-terminal`.
- A command would write to `openai/symphony`.
- A task asks for Symphony writes without explicitly assigning `TojotheTerror/symphony`.
- A required write target is ambiguous.
- A change would require secrets or local-only credentials.
- A change would require touching local agent state such as `.codex/` or `.serena/`.

## Recovery

If upstream push is enabled:

```bash
git remote -v
git remote get-url --push upstream
git remote set-url --push upstream DISABLED
git remote -v
```

If a push is blocked by the hook:

1. Do not bypass the hook.
2. Run `git remote -v`.
3. Confirm the intended push target.
4. Fix the remote push URL only if the approved fork target is clear.
5. Ask for human review if the target remains ambiguous.

If an upstream write was attempted or may have succeeded:

1. Stop all write actions.
2. Record the command, remote, URL, branch, commit, timestamp, and observed output.
3. Do not delete branches, force-push, close PRs, or edit issues to hide the write.
4. Escalate for human review and follow the repository owner's remediation decision.

## Related Docs

- [ADR-002: Repository Safety Boundary](./ADR-002-repository-safety-boundary.md)
- [Upstream Intake](./upstream-intake.md)
- [Contributor Guide](../../CONTRIBUTING.md)
