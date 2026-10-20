# Homebrew tap

The formula is rendered from the release manifest after the Apple Silicon
`aarch64-apple-darwin` artifact exists:

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
