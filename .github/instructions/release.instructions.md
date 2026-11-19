---
applyTo: "**/Cargo.toml,release-plz.toml,.github/workflows/release-plz.yml,**/CHANGELOG.md"
---

# Crates and releases

- Versions are SemVer 2.0 (Cargo's interpretation). Pre-releases (`-rc.1`) are fine, but
  build metadata (`+...`) is not.
- Versions and `CHANGELOG.md` are managed by release-plz release PRs. Do not bump versions
  or edit generated changelog entries by hand in feature PRs.
- Every publishable crate has `description`, `license`, `repository`, `readme`, `keywords`
  (at most 5) and `categories` (valid crates.io slugs), and declares `rust-version`. Crates that
  are not published have `publish = false`.
- `width-search` is a single crate, tagged `vX.Y.Z`.
- `tests/fixtures/` and the GPL license text stay in `exclude`: the fixtures are test input
  under their own licenses (`REUSE.toml`) and must not ship in the package.
- New dependencies must pass `cargo deny check`: allowed licenses, no advisories, no git or
  unknown-registry sources. Path dependencies of publishable crates carry a `version`.
- `rust-version` is a signal, not a contract. Raising it is allowed when a dependency or a
  feature benefits. It needs a minor release and a changelog note, never a patch release.
- release-plz runs in git-only mode: it tags and creates GitHub Releases. `publish-crates-io.yml`
  publishes from the tag in the `crates-io` environment, via trusted publishing
  (`id-token: write`, no `CARGO_REGISTRY_TOKEN`). Keep it that way.
- Changelog groups and minor bumps come from imperative subject verbs (`release-plz.toml`).
  A subject that hides a feature behind "Update ..." lands under Changed. Flag it.
