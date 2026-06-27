# ADR-002: Repository Safety Boundary

| Field | Value |
|---|---|
| Status | Accepted |
| Date | 2026-06-27 |
| Issue | CODEX-28 |
| Scope | Intelligent Terminal fork work by humans, Codex workers, Symphony workers, and other automation |

## Context

Intelligent Terminal is a forked codebase. Local work often needs to compare against upstream projects, but only fork-owned repositories are valid write targets. Agentic workers can run Git and GitHub CLI commands quickly, so the repository needs an explicit boundary that is easy to audit before any write-capable command runs.

This ADR defines repository write ownership for Intelligent Terminal work. It does not replace the public contribution process in `CONTRIBUTING.md`, and it does not authorize any upstream write.

## Decision

No worker may write to an upstream repository.

Repository roles:

| Repository | Role | Write policy |
|---|---|---|
| `TojotheTerror/intelligent-terminal` | Writable Intelligent Terminal fork | Allowed write target for this project when the task explicitly requires a branch, PR, or push |
| `TojotheTerror/symphony` | Related writable repository | Writable only when a task explicitly assigns Symphony work |
| `microsoft/intelligent-terminal` | Upstream Intelligent Terminal repository | Read-only |
| `openai/symphony` | Upstream Symphony repository | Read-only |

The default local remote layout for this project is:

```text
origin    https://github.com/TojotheTerror/intelligent-terminal.git  fetch/push
upstream  https://github.com/microsoft/intelligent-terminal.git      fetch
upstream  DISABLED                                                   push
```

Workers must inspect remotes before write-capable Git or GitHub CLI actions:

```bash
git status --short --branch
git remote -v
git remote get-url --push upstream
git config --get-all remote.upstream.pushurl
```

Disable upstream pushes locally:

```bash
git remote set-url --push upstream DISABLED
```

GitHub CLI commands that can write must use an explicit repository target. For Intelligent Terminal work, prefer:

```bash
gh pr create -R TojotheTerror/intelligent-terminal
gh pr view -R TojotheTerror/intelligent-terminal
gh issue view -R TojotheTerror/intelligent-terminal
```

Do not rely on `gh` defaults for write actions. Do not run `gh pr create`, `gh pr edit`, `gh issue create`, `gh release create`, or similar write commands unless `-R owner/repo` points at the approved fork.

## Local Safeguard

The repository carries a conservative pre-push hook template at `.githooks/pre-push`. It blocks:

- any remote whose name contains `upstream`;
- `microsoft/intelligent-terminal` push URLs;
- `openai/symphony` push URLs;
- any push URL other than `https://github.com/TojotheTerror/intelligent-terminal.git`.

Developers can enable the tracked hook locally with:

```bash
git config --local core.hooksPath .githooks
```

The hook is a local safeguard, not a substitute for reviewing the command target before every write-capable operation.

## Consequences

- Upstream fetch, diff, log, and analysis are allowed.
- Local branches may incorporate reviewed upstream changes only into the writable fork.
- Upstream PRs, branches, tags, releases, issues, and pushes are forbidden from this workspace.
- Symphony is out of scope unless the task explicitly assigns work in `TojotheTerror/symphony`.
- Ambiguous remotes are a hard stop until a human confirms the target repository.

## Recovery

If a command attempts an upstream write or remote targets are ambiguous:

1. Stop immediately.
2. Capture the exact command, remote, URL, branch, and timestamp.
3. Run `git remote -v` and `git remote get-url --push upstream`.
4. If safe, run `git remote set-url --push upstream DISABLED`.
5. Do not retry, delete, force-push, or open corrective PRs until a human reviews the incident.
