# Release Process

This document describes how to cut a new release.

## Prerequisites

- Clean git working tree
- On `main` branch, in sync with `origin/main`
- `gh` CLI authenticated
- `cargo` available (to update `Cargo.lock`)

## Steps

1. **Update `CHANGELOG.md`** — add a new section at the top for the version you are releasing:

   ```markdown
   ## [X.Y.Z] - YYYY-MM-DD

   ### Added / Changed / Fixed
   - ...
   ```

2. **Bump version in `Cargo.toml`** to match.

3. **Run `cargo check`** so `Cargo.lock` picks up the new version.

4. **Commit** the three files with message `Release vX.Y.Z`.

5. **Tag** the commit: `git tag vX.Y.Z`.

6. **Push** both the commit and the tag: `git push origin main vX.Y.Z`.

7. **Create the GitHub release** with notes copied from the `CHANGELOG.md` entry:

   ```sh
   gh release create vX.Y.Z --title vX.Y.Z --notes "<changelog entry>"
   ```

   The release workflow will then build Docker images and binary archives, attaching them to the release.

## Automation

The `/release` slash command automates steps 2-7. The only manual step is updating `CHANGELOG.md` first.

Usage:

```
/release X.Y.Z
```

## Versioning

Follow [Semantic Versioning](https://semver.org/):

- **PATCH** (`0.2.0` → `0.2.1`): bug fixes, no API/behavior changes
- **MINOR** (`0.2.0` → `0.3.0`): new features, backward-compatible changes
- **MAJOR** (`0.2.0` → `1.0.0`): breaking changes
