---
description: Cut a new release (bumps version, commits, tags, pushes, creates GitHub release)
argument-hint: <version>
---

Cut a new release for version `$ARGUMENTS`.

Follow the process documented in `dev/RELEASE.md`. The user has already updated `CHANGELOG.md` with a new section for this version.

Do the following, stopping and reporting if any check fails:

1. **Preflight checks**:
   - Working tree is clean (`git status --porcelain` is empty)
   - Current branch is `main`
   - Local `main` matches `origin/main` (run `git fetch origin main` first)
   - Tag `v$ARGUMENTS` does not already exist
   - `CHANGELOG.md` contains a `## [$ARGUMENTS]` section at the top
   - `$ARGUMENTS` is a valid semver (`X.Y.Z`)

2. **Bump version**: update the `version = "..."` line in `Cargo.toml` to `$ARGUMENTS`.

3. **Sync lockfile**: run `cargo check` so `Cargo.lock` reflects the new version.

4. **Extract release notes**: read the section for `$ARGUMENTS` from `CHANGELOG.md` (everything between the `## [$ARGUMENTS]` heading and the next `## [` heading, excluding the heading itself).

5. **Show the release notes to the user and ask for confirmation** before proceeding. If the user declines, revert `Cargo.toml` and `Cargo.lock` changes and stop.

6. On confirmation:
   - Commit `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md` with message `Release v$ARGUMENTS`
   - Create tag `v$ARGUMENTS`
   - Push `main` and the tag: `git push origin main v$ARGUMENTS`
   - Create the GitHub release: `gh release create v$ARGUMENTS --title v$ARGUMENTS --notes "<extracted notes>"`

7. Report the release URL.

Do NOT add a Claude co-author line to the commit.
