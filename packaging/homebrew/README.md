# Homebrew tap

The formula is rendered from a signed release manifest after the macOS
`aarch64-apple-darwin` and `x86_64-apple-darwin` artifacts exist:

```sh
python3 scripts/render_homebrew.py dist/0.2.0/release-manifest.json
```

Publish the generated `dist/mc-server-tui.rb` in a tap repository (for example
`ORG/homebrew-tap`) and install with:

```sh
brew install ORG/tap/mc-server-tui
mc setup
```

Phase 1 does not publish a tap or notarize the artifacts. Do not advertise the
formula as production-ready until Phase 2 signing/notarization is complete.
