# Releasing

Releases are automated with [Knope](https://knope.tech/), configured in
`knope.toml`. The desktop app and the CLI ship together on one version, and
one GitHub release carries the installers and the CLI archives.

## How a release happens

1. On every push to `main`, the `Prepare Release` workflow runs Knope. If
   there are releasable changes since the last tag, Knope bumps the version,
   writes a `CHANGELOG.md` section, force-pushes the result to the
   `chore/release` branch, and opens or updates one pull request titled
   `chore: release <version>`.
2. Merging that pull request is what releases. The `Release` workflow runs on
   every push to `main` and releases only when both are true:
   - the version in `Cargo.toml` has no `v<version>` tag, and
   - `CHANGELOG.md` has a `## <version> ` section.

   Only the release pull request writes that section, so a version changed
   any other way does not publish.
3. The `Release` workflow builds the installers and CLI archives for Windows,
   macOS, and Linux, strictly from the lockfiles. Knope then creates the
   GitHub release as a draft, uploads every file, and publishes it.

Leave the `chore/release` pull request open while work accumulates. It
updates itself after every merge to `main`.

## Where the version lives

The single source of truth is `version` under `[workspace.package]` in the
root `Cargo.toml`. Each crate inherits it with `version.workspace = true`.

Knope updates all of these together:

- `Cargo.toml` (the workspace version)
- `Cargo.lock` (the entries for `redactify-app`, `redactify-cli`, and
  `redactify-core`)
- `app/package.json`
- `app/package-lock.json`

`Cargo.lock` appears three times in `knope.toml` because each entry updates
one package. Never edit any of these versions by hand.

## What counts as a release

Only `feat` and `fix` commits, breaking changes, and changesets produce a
release. Other types, such as `docs`, `chore`, and `ci`, never do.

`knope.toml` sets `scopes = ["app", "cli", "core"]`:

- A scoped commit counts only if its scope is one of those three, so
  `fix(ci):` never triggers a release.
- A commit with no scope always counts, so `feat:` and `fix:` without a scope
  will trigger a release.

Below 1.0, Knope shifts every level down one. A breaking change bumps the
minor version (0.6.2 to 0.7.0), and a `feat` or `fix` bumps the patch version
(0.6.2 to 0.6.3).

## Choosing the version

To release a bigger bump than the commits imply, for example to finish a
milestone as 0.7.0, use either method.

**A changeset.** Run `knope document-change`, choose `major` (a minor bump
below 1.0), and write a summary. Commit the file it creates under
`.changeset/` along with your change. The summary becomes the changelog
entry. The command exists only because `knope.toml` declares it: defining any
workflows there replaces Knope's built-in commands.

**An override.** In the Actions tab, run `Prepare Release` manually and set
`override_version` to the exact version, such as `0.7.0`.

## Tooling notes

- Knope is downloaded at a pinned version in both `prepare-release.yml` and
  `release.yml`. Dependabot cannot see it, so upgrade it by hand and keep both
  files on the same version.
- The release branch is `chore/release` rather than Knope's default
  `release` so that it passes the branch-name check.
- `knope.toml` uploads `artifacts/*`. That glob does not cross directories,
  so the `Release` workflow copies every installer and archive into one flat
  directory first.
- Both workflows authenticate with the `RELEASE_PLZ_TOKEN` secret, a
  fine-grained personal access token. A push made with the built-in
  `GITHUB_TOKEN` does not trigger checks on the release pull request, which
  is why the token is needed.
- The repository allows merge commits only. Knope reads each conventional
  commit individually, and squashing would collapse a pull request into one
  message it may not recognize.

## Troubleshooting

**The release pull request didn't appear.** Check the `Prepare Release` run.
If it reports that nothing is releasable, no `feat` or `fix` commit with a
matching scope has landed since the last tag. Add a changeset if the work
should still release.

**A release build failed with "the lock file needs to be updated".** The
release builds with `--locked`, so a manifest was changed without committing
the matching lockfile. Run `cargo build` (or `npm install` in `app/`) locally
and merge the updated lockfile in a normal pull request.

**A release build failed with a Tauri version mismatch.** A Tauri crate and
its npm package differ in major.minor. See
[Dependencies](dependencies.md#tauri-plugins).

**Both workflows fail with permission errors.** `RELEASE_PLZ_TOKEN` has
probably expired. Generate a new token and update the secret.
