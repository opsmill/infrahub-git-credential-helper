# Packaging & Distribution

The helpers are distributed as a **Python wheel built by [maturin](https://www.maturin.rs/)**
(`bindings = "bin"` in `pyproject.toml`), the same model `ruff` and `ty` use.
Installing the package with `pip`/`uv`/`poetry` drops both binaries on `PATH`
(in `<env>/bin`) like a Python console script — no runtime download or
`docker COPY`. Build locally with `make wheel`.

Two things worth knowing that aren't obvious from `pyproject.toml`:

- Both binaries ship in **one** wheel — maturin supports multiple `[[bin]]`
  targets since v0.12.20 ([PR #948](https://github.com/PyO3/maturin/pull/948)).
- The version comes from `Cargo.toml` (`dynamic = ["version"]`), so the existing
  `Cargo.toml`-vs-tag release check is unchanged.

## CI / release flow

- **`ci.yml`** — a `wheel` job runs `maturin build` on every PR, so a packaging
  regression fails fast.
- **`release.yml`** — on `release: published`, builds wheels (Linux + macOS,
  `amd64` + `arm64`) and an sdist, then publishes to PyPI. Runs alongside the
  existing Docker and GoReleaser jobs, which are unchanged.

### One-time PyPI setup (trusted publishing)

`publish-pypi` uses OIDC trusted publishing (no API token). Before the first
release that publishes a wheel, add a **trusted publisher** on PyPI: claim the
`infrahub-git-credential-helper` project, then under *Publishing* add a GitHub
Actions publisher with owner `opsmill`, repo `infrahub-git-credential-helper`,
workflow `release.yml`.

Until then `publish-pypi` fails (the rest of the release still succeeds). To use
an API token instead, drop the `id-token` permission and pass
`password: ${{ secrets.PYPI_API_TOKEN }}` to the publish action.

## How Infrahub consumes this (Option A — chosen)

Infrahub ships its Python helpers as console entry points; `uv sync` installs
them into `/.venv/bin` and the Docker stage points git at them. To adopt the
Rust binaries **without removing the Python implementation**:

1. **Add the dependency** `infrahub-git-credential-helper` to Infrahub's
   `[project.dependencies]`. `uv sync` then installs the wheel into the same
   `/.venv/bin` — no Dockerfile change needed.

2. **Resolve the name collision.** The wheel and the Python entry points both
   want `/.venv/bin/infrahub-git-credential` (last install wins). Keep Python as
   a revertible fallback by renaming only its console scripts — the code is
   untouched:

   ```toml
   [project.scripts]
   infrahub-git-credential-py = "infrahub.git_credential.helper:app"
   infrahub-git-askpass-py    = "infrahub.git_credential.askpass:app"
   ```

   The canonical names now resolve to the Rust binaries, git config keeps
   working, and reverting is a one-line rename.

## Option B — vendor the Rust crate into the Infrahub monorepo (not chosen)

Move the crate into Infrahub (e.g. `backend/git_credential_rs/`) as a
[uv workspace](https://docs.astral.sh/uv/concepts/projects/workspaces/) member
with the maturin backend, and have `uv sync` build it from source.

- ➕ Single source of truth; Rust and Python change atomically; no separate
  release to coordinate.
- ➖ `uv sync` compiles Rust, so the **server build image needs a Rust toolchain**
  and pays cargo/LTO build time on every build.
- ➖ You still need cross-platform wheels for non-developer installs, so you
  often end up doing Option A anyway.
- ➖ A Rust compile error blocks the whole Infrahub build.

**Why A:** it keeps the helper independently versioned, needs no Rust toolchain
in Infrahub's build, and reuses the exact mechanism Python already uses. Option
B's monorepo atomicity doesn't outweigh adding a Rust build to the server image.
