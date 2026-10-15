# Claude Code agents

The repository defines two [Claude Code](https://claude.com/claude-code)
subagents in `.claude/agents/`. Claude Code loads them automatically when it
runs in this repository. Using them is optional; neither replaces the CI
checks or human review.

To run one, ask Claude Code for it by name, for example "use the
pre-merge-reviewer agent on this branch".

| Agent | Definition | Changes files |
| --- | --- | --- |
| `docs-writer` | `.claude/agents/docs-writer.md` | Markdown documentation only |
| `pre-merge-reviewer` | `.claude/agents/pre-merge-reviewer.md` | Never, by instruction |

## docs-writer

Keeps `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, and `docs/` accurate and
current with the code.

**Run it** after a code change that affects what the documentation says, such
as a new feature, a changed command or flag, or a new CI step, before opening
the pull request. Tell it what changed; it reads the code to confirm the
details before writing.

What it does and does not do:

- It edits only those Markdown files. It never changes code, manifests,
  lockfiles, workflows, `CHANGELOG.md`, `app/README.md`, ADR decisions, or
  screenshots.
- In `CONTRIBUTING.md` and `docs/development/`, it updates commands, paths,
  and check names, but not the project's branch, commit, review, or release
  rules.
- It cannot run commands. Its final report lists the files it changed,
  anything it found out of date but could not fix within its scope, and any
  command it documented without seeing it used elsewhere in the repository.

Review its changes like any other diff before committing them.

## pre-merge-reviewer

Reviews a branch against `main` before its pull request merges. It has no
file-editing tools, and its instructions limit it to read-only git commands.
It does have shell access, though, and a subagent's tool list cannot restrict
which shell commands it runs. What stops a command that changes something is
Claude Code's permission prompt, which asks before any shell command outside
its [built-in read-only set](https://code.claude.com/docs/en/permissions#read-only-commands).
Run it in the default permission mode: a subagent inherits `acceptEdits`,
`auto`, or `bypassPermissions` from the main session, and those modes approve
some or all commands without asking.

**Run it** when a branch is ready for review, after its commits are final and
before the pull request merges. Because pull requests merge with merge
commits, every commit reaches the changelog individually, which is what this
review protects.

It checks that:

- each commit message follows Conventional Commits and its type matches the
  content, and that `fix` is used only for bugs that shipped in a release;
- nothing landed halfway, such as a later commit undoing an earlier one, code
  changes without matching documentation, or a manifest change without its
  lockfile;
- workflow and release-tooling changes are not mixed with application code;
- added code follows the project rules: no `unwrap()` or `panic!` outside
  Rust tests, a `// SAFETY:` comment on every `unsafe` block, no `any` or
  `@ts-ignore` in TypeScript, and synthetic test data only.

Its report starts with a verdict, **ready to merge** or **needs changes**,
then one section per check with evidence for each finding. It compares against
the local `main`, which may be behind the remote, so update `main` first. It
does not claim that code compiles or tests pass; run the checks in
[CONTRIBUTING](../../CONTRIBUTING.md#before-opening-a-pull-request) yourself.
