# ggober

`ggober` finds removable build and cache artifacts in code folders.

By default it performs a dry run and only reports what it would delete. Use
`--delete` to remove matches, or `--delete --auto-approve` to remove everything
without per-target confirmation.

## Usage

```sh
ggober [ROOT] [--profile all|python|rust|js] [--max-depth N] [--delete] [--auto-approve]
```

Examples:

```sh
# Scan the current directory
ggober

# Scan a project, but only Rust artifacts
ggober ~/Code/my-project --profile rust

# Delete matches after confirming each target
ggober --delete

# Delete all matches without prompts
ggober --delete --auto-approve
```

## Profiles

- `python`: `.venv`, `venv`, `__pycache__`, pytest/mypy/ruff caches, notebooks checkpoints, `.coverage`
- `rust`: `target`
- `js`: `node_modules`, `dist`, `build`, `.next`, `.svelte-kit`
- `all`: all of the above
