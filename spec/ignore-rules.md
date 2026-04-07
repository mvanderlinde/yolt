# Ignore rules specification

## Syntax

Gitignore-style patterns as implemented by the `ignore` crate / `gitignore` rules:

- `*` and `**` globs
- Trailing `/` for directories
- `!` negation (if supported by matcher)
- Lines starting with `#` are comments in `.yoltignore`

## Layers

1. **Built-in defaults** (see below) unless `--no-default-ignores`.
2. **`.yoltignore`** in the **watch root** (optional file).
3. **`--ignore` / config** extra patterns.

Patterns are matched against paths **relative to watch root** with POSIX separators.

## Built-in default patterns

```
# JS / TS
node_modules/
.next/
.nuxt/
dist/
build/
.turbo/
.parcel-cache/

# Python
__pycache__/
*.pyc
*.pyo
.venv/
venv/
.tox/
.mypy_cache/

# Rust
target/

# Ruby
vendor/bundle/

# Go
vendor/

# OS / misc
.DS_Store
Thumbs.db

# VCS (see product rationale)
.git/
```

## `.git/` decision

**Ignored by default.** Users may include `.git/` by using `--no-default-ignores` and supplying their own pattern set, or by adding `!.git/` negation in `.yoltignore` **if** negation is enabled in the matcher for overrides.

## Acceptance criteria

- `node_modules/foo` is ignored.
- `src/main.rs` is not ignored by defaults.
- `.git/config` is ignored by default.
