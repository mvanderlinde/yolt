# Distribution specification

## Homebrew

- **Tap:** GitHub repo `mvanderlinde/homebrew-yolt` (tap name `mvanderlinde/yolt`). See [`packaging/homebrew-tap/README.md`](../packaging/homebrew-tap/README.md) for maintainer setup and stable-release steps.
- **Formula:** [`packaging/homebrew-tap/Formula/yolt.rb`](../packaging/homebrew-tap/Formula/yolt.rb) — builds from source with `cargo install` (see `std_cargo_args`); add `url` + `sha256` for stable installs from a tagged release tarball.
- **Platforms:** macOS **arm64** and **x86_64** (bottles optional in future).
- **Dependencies:** Rust toolchain at build time only.

## Release artifacts

- Source tarball from tag (GitHub or similar).
- Optional: prebuilt `yolt` binary attached to releases.

## Supported macOS

- **Minimum:** macOS 11+ (Big Sur) for development baseline; document if older versions break.

## Launchd (optional)

Not required for v1; users run `yolt watch` in a terminal or wrap with their own plist.
