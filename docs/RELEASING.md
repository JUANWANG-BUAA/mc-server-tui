# Releasing

The release path is tag-driven and creates a **draft** GitHub Release. It is
intentionally not published automatically because Phase 1 artifacts are
unsigned.

## Before tagging

1. Update workspace `Cargo.toml` and `Cargo.lock`; the tag must match exactly.
   Update both READMEs, guides, site metadata, changelog and versioned release notes.
   Regenerate documentation with `python3 scripts/build_docs_site.py`.
   Record actual acceptance and remaining limits in `docs/acceptance/<version>.md`.
2. Run locally:

   ```sh
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

3. Commit and push `main`.
4. Wait for the `CI` workflow to pass.

## Cut a release

```sh
git tag -a v0.2.1 -m "mc-server-tui 0.2.1"
git push origin v0.2.1
```

The `Release` workflow then:

- validates that `v<version>` matches `Cargo.toml`;
- builds `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, and
  `aarch64-unknown-linux-gnu`;
- creates tar.gz + SHA-256 + per-target manifests;
- merges the manifests and renders the Homebrew formula;
- creates `install.sh` with the release manifest URL baked in;
- creates or updates a **draft** GitHub Release with all assets.

The Linux arm64 job uses the public `ubuntu-24.04-arm` runner. If that runner is
not available for the repository, replace that matrix entry with a `cross`
build before tagging.

## Review and publish

```sh
gh release view v0.2.1
gh release download v0.2.1 --pattern 'release-manifest.json' --dir /tmp/mcst-release
```

Check:

- all three target artifacts are present;
- `release-manifest.json` has the expected version, channel, API protocol, and
  targets;
- `install.sh` points at the release manifest URL;
- `mc-server-tui.rb` has the Apple Silicon macOS URL/checksum;
- install/uninstall works from the downloaded artifacts.

Then publish the draft, optionally as a pre-release:

```sh
gh release edit v0.2.1 --notes-file docs/releases/0.2.1.md
gh release edit v0.2.1 --draft=false --prerelease
```

Do not describe unsigned artifacts as production-ready. Phase 2 adds signing,
notarization, and Linux package repositories.

## Reruns

Re-running the workflow for an existing tag uploads assets with `--clobber` to
the existing release. Do not delete a published release to “fix” an artifact;
cut a new version instead.
