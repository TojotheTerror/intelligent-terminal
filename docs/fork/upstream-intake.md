# Upstream Intake

This runbook covers reviewed intake of upstream changes into the writable Intelligent Terminal fork. It is not the public contribution workflow, and it is not the OpenConsole `inbox` or Git2Git flow described by older inherited docs.

## Boundary

Allowed:

- fetch upstream branches and tags;
- inspect upstream logs, diffs, and files;
- analyze conflicts and compatibility;
- create local intake branches in the writable fork after review;
- cherry-pick or merge reviewed upstream changes into local fork branches;
- open PRs only against `TojotheTerror/intelligent-terminal`.

Forbidden:

- pushing to `microsoft/intelligent-terminal`;
- pushing to `openai/symphony`;
- creating upstream branches, tags, releases, issues, or PRs;
- using `gh` write commands without explicit `-R TojotheTerror/intelligent-terminal`;
- using `doc/submitting_code.md` as the intake procedure for Intelligent Terminal upstream pulls.

## Preflight

Start from a clean, unambiguous workspace:

```bash
git status --short --branch
git remote -v
git remote get-url origin
git remote get-url --push origin
git remote get-url upstream
git remote get-url --push upstream
```

Expected write target:

```text
origin push:   https://github.com/TojotheTerror/intelligent-terminal.git
upstream push: DISABLED
```

If upstream push is not disabled:

```bash
git remote set-url --push upstream DISABLED
```

Hard fail if the worktree is dirty before intake starts, unless the task explicitly says to continue from those exact local changes.

## Fetch, Diff, Analyze

Fetching and analysis are read-only and allowed:

```bash
git fetch upstream
git log --oneline --decorate --graph origin/main..upstream/main
git diff --stat origin/main...upstream/main
git diff --name-only origin/main...upstream/main
```

Record the upstream range, branch, tag, or commit list before making an intake branch. Do not cherry-pick a commit just because it fetches cleanly; first classify what it touches.

## Classification

Classify upstream changes before applying them locally:

| Area | Review requirement |
|---|---|
| Shared Windows Terminal or OpenConsole code | Check for conflicts with fork-specific changes |
| `src/cascadia/TerminalApp`, `src/cascadia/WindowsTerminal`, protocol, settings, package identity | Require explicit Intelligent Terminal review because these areas overlap agent integration |
| `tools/wta/**`, WTCLI, hooks, ACP, autofix | Treat as fork-owned behavior; do not overwrite without a task-specific decision |
| `oss/`, dependency manifests, generated notices | Check provenance and generated-file expectations |
| Docs-only changes | Check whether the upstream audience matches this fork |
| Generated files | Regenerate or verify with the documented toolchain |

## Local Intake

Create an intake branch only in the fork:

```bash
git switch -c codex/upstream-intake-<short-topic>
```

Use an explicit, reviewed strategy:

```bash
git cherry-pick -x <upstream-commit>
```

or:

```bash
git merge --no-ff <reviewed-upstream-ref>
```

Do not continue through conflicts mechanically. Preserve Intelligent Terminal-specific WTA, WTCLI, COM protocol, agent-pane, package-identity, hooks, and autofix behavior unless the task explicitly says otherwise.

## Validation

Choose validation based on touched areas. For agent or terminal implementation changes, the normal local build sequence is Rust WTA first, then Terminal/MSBuild:

```bash
cargo build --target x86_64-pc-windows-msvc --manifest-path tools/wta/Cargo.toml
cmd.exe //c "tools\razzle.cmd && bcz no_clean"
```

For docs-only intake, review markdown links and changed files instead of running heavy builds. For dependency or packaging changes, include the relevant package or installer validation from existing project docs.

## PR Summary Requirements

An upstream-intake PR must state:

- upstream source repository and range;
- strategy used, such as cherry-pick or merge;
- skipped commits and why;
- conflicts and resolutions;
- fork-specific behavior that was preserved;
- validation commands and results;
- any remaining risks or required owner reviews.

Use explicit GitHub CLI targets:

```bash
gh pr create -R TojotheTerror/intelligent-terminal
```

## Hard-Fail Conditions

Stop immediately if:

- remotes are missing, ambiguous, or point to the wrong repository;
- any upstream push URL is enabled;
- the worktree is dirty before intake starts without explicit approval;
- the process starts following `inbox`, Git2Git, or OS-repo submission instructions;
- conflict resolution removes Intelligent Terminal-specific agent, WTA, WTCLI, COM, package identity, hooks, or autofix behavior;
- upstream changes touch `oss/` without dependency provenance review;
- validation fails in a way that is not understood.

## Recovery

If an intake operation fails, abort the operation that is in progress:

```bash
git cherry-pick --abort
git merge --abort
git rebase --abort
```

Use only the abort command that matches the active operation. Do not use destructive reset or force-push as a default recovery step.

If remotes are ambiguous or an upstream write is attempted:

1. Stop immediately.
2. Run `git remote -v`.
3. Disable upstream push URLs with `git remote set-url --push upstream DISABLED` when safe.
4. Record the attempted command and observed output.
5. Ask for human review before retrying.
