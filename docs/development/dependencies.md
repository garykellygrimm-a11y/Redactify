# Dependencies

Redactify has four sets of dependencies: Rust crates (`Cargo.toml` and
`Cargo.lock`), npm packages for the desktop app's frontend
(`app/package.json` and `app/package-lock.json`), the GitHub Actions the
workflows use, and the Rust toolchain (`rust-toolchain.toml`).

## Dependabot

Dependabot, configured in `.github/dependabot.yml`, checks all four weekly.

- For Cargo and npm, minor and patch updates arrive as one grouped pull
  request per ecosystem. Major updates arrive as separate pull requests, one
  per dependency, because they can break things and deserve their own review.
- For GitHub Actions, every update, major ones included, arrives in one
  grouped pull request.
- For the Rust toolchain, each new Rust release arrives as its own pull
  request that changes `channel` in `rust-toolchain.toml`.
- A new version is not proposed until it has been published for 7 days. This
  gives a broken or malicious release time to be caught and withdrawn.
  Security updates are not delayed.
- Commits are prefixed `chore(deps)` for Cargo, npm, and the Rust toolchain
  and `ci(deps)` for Actions, so dependency updates never trigger a release
  on their own.

## Reviewing a Dependabot pull request

1. Wait for the required checks. `Format, Lint, Test` and
   `Frontend type-check and build` show whether the update still compiles and
   passes; `cargo audit` shows whether it introduced an advisory.
2. If the pull request touches a Tauri crate or npm package, check that
   `Tauri crate/npm versions` passed. See [Tauri plugins](#tauri-plugins).
3. For a major update, including one inside the grouped Actions pull request,
   read the dependency's changelog for breaking changes before merging, even
   if the checks pass.
4. Merge with a merge commit.

## Tauri plugins

Tauri and each Tauri plugin ship in two halves: a Rust crate that runs in the
backend and an npm package the frontend calls. The two must share the same
major.minor version. `tauri build` refuses to build a mismatched pair, and the
`Tauri crate/npm versions` check fails on one.

The pairs in this project are:

| Rust crate | npm package |
| --- | --- |
| `tauri` | `@tauri-apps/api` |
| `tauri-plugin-dialog` | `@tauri-apps/plugin-dialog` |
| `tauri-plugin-opener` | `@tauri-apps/plugin-opener` |
| `tauri-plugin-window-state` | `@tauri-apps/plugin-window-state` |

Dependabot updates Cargo and npm in separate pull requests, so one half can
move without the other. When that happens, update the lagging half in the same
pull request before merging it:

```console
$ cargo update -p tauri-plugin-dialog                  # from the repository root
$ npm install @tauri-apps/plugin-dialog@<version>      # from app/
```

Then run `python scripts/check-tauri-versions.py` to confirm every pair
matches.

## Updating a dependency by hand

Update one crate at a time, by name:

```console
$ cargo update -p <crate>
```

Never run a bare `cargo update`. Without `-p`, Cargo updates every dependency
in the lockfile to the newest version its range allows, and the resulting diff
is too large to review.

For npm, run `npm install <package>@<version>` from `app/`, which updates both
`package.json` and `package-lock.json`.

After either, run the checks in
[CONTRIBUTING](../../CONTRIBUTING.md#before-opening-a-pull-request), and
commit the manifest and lockfile together.

## Toolchains

| Tool | Version | Where it is set |
| --- | --- | --- |
| Rust | pinned | `channel` in `rust-toolchain.toml` |
| Node.js | 24 | `node-version` in `ci.yml`, `release.yml`, and `codeql.yml` |
| Python | 3.11 or newer | needed by `scripts/check-tauri-versions.py` |
| Knope | pinned | `KNOPE_VERSION` in `prepare-release.yml` and `release.yml` |

`rust-toolchain.toml` sets the Rust version for every `cargo` command run in
the repository, locally and in CI. rustup reads it and installs that version,
with `rustfmt` and `clippy`, the first time it is needed. To upgrade Rust,
merge the Dependabot pull request that changes `channel`. If the new clippy
reports warnings, fix them in that pull request before merging.

Knope is downloaded directly rather than through an action, so Dependabot
cannot see it. Upgrade it by hand, keeping both workflows on the same version.
See [Releasing](releasing.md#tooling-notes).

## Security advisories

`cargo audit` checks every Rust dependency against the RustSec advisory
database on each pull request and weekly. See
[CI checks](ci.md#cargo-audit) for how to handle a new advisory.
