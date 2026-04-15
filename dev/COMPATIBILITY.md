# Drop-in Replacement Constraints

This project replaces a Python credential helper that ships inside Infrahub. Behavioral fidelity with the Python original is a **hard requirement**.

## Do not "fix" these

The following quirks are intentional and must be preserved. They match the Python original exactly so that existing deployments keep working without reconfiguration.

### `credential.usehttppath` is required

The credential helper errors out if the git credential input does not include `path=`. Users must set `git config --global credential.usehttppath true`. This is by design — the Python helper has the same requirement.

### `split('=').nth(1)` for key=value parsing

When parsing git credential input, a line like `path=foo=bar` yields `path` → `foo` (value is truncated at the second `=`). This matches Python's `line.split("=")[1]`. Do **not** change it to `splitn(2, '=')` even though that would preserve the full value.

## Stdout vs stderr

- `infrahub-git-credential` writes errors to **stdout** and exits 1 (matches Python's `print()` + `typer.Exit(1)`).
- `infrahub-git-askpass` writes errors to **stderr** and exits 1 (matches Python's `typer.Exit("message")` which calls `sys.exit("message")`).

Do not unify these channels.

## Env var names

Env vars must match the Python SDK's names (`INFRAHUB_*` prefix, snake_case fields). The SDK-compatible `INFRAHUB_ADDRESS` fallback exists for parity even though the Python credential helper itself uses `INFRAHUB_INTERNAL_ADDRESS`.
