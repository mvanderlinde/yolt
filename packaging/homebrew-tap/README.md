# homebrew-yolt

Homebrew tap for **[yolt](https://github.com/mvanderlinde/yolt)** — undo destructive LLM actions: backs up files before they change for quick reverts.

## Install

```sh
brew tap mvanderlinde/yolt
brew install yolt
```

If the formula does not yet ship a stable `url`/`sha256` for a tagged release, install from the default branch with:

```sh
brew tap mvanderlinde/yolt
brew install --HEAD yolt
```

## Maintaining this tap

### Stable release (after a new tag on `mvanderlinde/yolt`)

1. Compute the SHA-256 of the release tarball (replace the tag if needed):

   ```sh
   curl -sL https://github.com/mvanderlinde/yolt/archive/refs/tags/v0.1.0.tar.gz | shasum -a 256
   ```

2. In `Formula/yolt.rb`, set `url`, `sha256`, and `version` for that tag. You can keep `head` for optional `--HEAD` installs.

3. Commit and push this repo so `brew install yolt` uses the tarball without `--HEAD`.

### Check before pushing

```sh
brew install --build-from-source ./Formula/yolt.rb
brew test yolt
brew audit --strict Formula/yolt.rb
```
