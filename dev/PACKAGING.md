# Packaging & Distribution

The helpers are distributed as a **Python wheel built by [maturin](https://www.maturin.rs/)**,
the same model `ruff` and `ty` use. This lets the binaries be installed with
`pip`/`uv`/`poetry` — they land on `PATH` (in `<env>/bin`) exactly like a Python
console script, with no runtime download or `docker COPY`.

## How the wheel is built

`pyproject.toml` declares maturin as the build backend in `bin` mode:

```toml
[build-system]
requires = ["maturin>=1.14,<2.0"]
build-backend = "maturin"

[tool.maturin]
bindings = "bin"
strip = true
```

Key points:

- **`bindings = "bin"`** — maturin compiles the crate's `[[bin]]` targets and
  packages each one into the wheel under `*.data/scripts/`. On install, those
  become executables in the environment's `bin/` directory. No Python module is
  shipped.
- **Both binaries ship in one wheel.** maturin gained multiple-binary support in
  v0.12.20 ([PR #948](https://github.com/PyO3/maturin/pull/948)), so a single
  wheel contains both `infrahub-git-credential` and `infrahub-git-askpass`.
- **Version is single-sourced from `Cargo.toml`.** `[project]` lists
  `dynamic = ["version"]`, so maturin reads the version from the crate. The
  release tag check stays the existing `Cargo.toml`-vs-tag comparison.
- **Wheels are Python-version independent** (`py3-none-<platform>` tags) but
  platform specific. CI builds Linux + macOS, `amd64` + `arm64`; `pip`/`uv`
  selects the matching wheel automatically.
- The GraphQL schema under `schema/` is a build-time input only and is **not**
  shipped in the wheel.

Build locally to inspect the output:

```sh
uvx maturin build --release --out dist
unzip -l dist/*.whl          # see the packaged scripts
```

## CI / release flow

- **`ci.yml`** has a `wheel` job that runs `maturin build` on every PR, so a
  broken `pyproject.toml` or packaging regression fails fast.
- **`release.yml`** (on `release: published`) builds wheels across a target
  matrix with `PyO3/maturin-action`, builds an sdist, and publishes everything
  to PyPI with `pypa/gh-action-pypi-publish`. This runs alongside the existing
  Docker-image and GoReleaser jobs — those distribution channels are unchanged.

### One-time PyPI setup (trusted publishing)

`publish-pypi` uses OIDC trusted publishing (`permissions: id-token: write`) —
no API token secret. Before the first release that publishes a wheel, configure
a **trusted publisher** on PyPI:

1. Create/claim the `infrahub-git-credential-helper` project on PyPI.
2. In the project's *Publishing* settings, add a GitHub Actions trusted
   publisher: owner `opsmill`, repo `infrahub-git-credential-helper`, workflow
   `release.yml`.

Until this is configured, the `publish-pypi` job will fail (the rest of the
release — Docker, binaries — still succeeds). Alternatively, swap to an API
token by removing the `id-token` permission and passing
`password: ${{ secrets.PYPI_API_TOKEN }}` to the publish action.

## How Infrahub consumes this (Option A — chosen)

The Infrahub server already ships its Python credential helpers as console
entry points in `pyproject.toml`:

```toml
[project.scripts]
infrahub-git-credential = "infrahub.git_credential.helper:app"
infrahub-git-askpass    = "infrahub.git_credential.askpass:app"
```

`uv sync` installs those into `/.venv/bin`, and the backend Docker stage points
git at them (`credential.helper "/usr/bin/env infrahub-git-credential"`).

To adopt the Rust binaries **without removing the Python implementation**:

1. **Add the dependency** to Infrahub's `pyproject.toml`:

   ```toml
   [project]
   dependencies = [
     # ...
     "infrahub-git-credential-helper",
   ]
   ```

   `uv sync` then installs the wheel; both binaries land in `/.venv/bin` — the
   same directory the Python scripts use, reached by the same `PATH`. No
   Dockerfile change is required.

2. **Resolve the name collision.** The Rust wheel and the Python entry points
   both want `/.venv/bin/infrahub-git-credential`; whichever installs last wins.
   Keep Python as a real, revertible fallback by renaming only its console
   scripts (the Python code is untouched and still importable):

   ```toml
   [project.scripts]
   infrahub-git-credential-py = "infrahub.git_credential.helper:app"
   infrahub-git-askpass-py    = "infrahub.git_credential.askpass:app"
   ```

   Now the canonical names resolve to the Rust binaries, the existing git config
   keeps working unchanged, and reverting is a one-line rename.

This keeps the Docker image, git config, and `usehttppath` requirement exactly
as they are today — the only change is a dependency line plus the entry-point
rename.

## Option B — vendor the Rust crate into the Infrahub monorepo (not chosen)

An alternative is to move this crate into the Infrahub repository (e.g.
`backend/git_credential_rs/`) and build it as part of Infrahub.

**Shape:** make the crate a [uv workspace](https://docs.astral.sh/uv/concepts/projects/workspaces/)
member with `build-backend = "maturin"`, and have Infrahub depend on it as a
path/workspace member. `uv sync` would build it from source.

**Trade-offs:**

- ➕ Single source of truth; Rust and Python change atomically in one PR; no
  separate release/version to coordinate.
- ➖ `uv sync` now compiles Rust, so the **server build image needs a Rust
  toolchain** and every build pays cargo compile time (the release profile uses
  LTO — minutes, not seconds).
- ➖ Cross-platform wheels still have to be built somewhere for non-developer
  installs, so you often end up publishing a wheel anyway — i.e. doing Option A
  in addition.
- ➖ Couples the Python package build to the Rust toolchain and to maturin
  workspace quirks; a Rust compile error blocks the whole Infrahub build.

**Why A was chosen:** Option A keeps the credential helper independently
versioned and released, requires **no Rust toolchain in Infrahub's build**, and
delivers the binaries through the exact mechanism Python already uses
(dependency → `uv sync` → `/.venv/bin`). Infrahub's only change is one
dependency line and the entry-point rename. Option B's main benefit
(monorepo atomicity) does not outweigh adding a Rust build to the Python server
image, especially since it would likely still need A's wheels for distribution.
