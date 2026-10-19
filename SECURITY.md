# Security policy

## Supported versions

mc-server-tui is currently an **alpha**. Security fixes target the latest
`main` and the newest published pre-release.

| Version | Supported |
| --- | --- |
| `0.1.x` alpha | best effort |
| older pre-releases | no |

## Reporting a vulnerability

Please use GitHub's private vulnerability reporting:

https://github.com/JUANWANG-BUAA/mc-server-tui/security/advisories/new

Do not open a public issue for a vulnerability. Include:

- affected version (`mc version --json`);
- platform and installation method;
- a minimal reproduction;
- impact and any known mitigation.

## Security boundaries

The daemon listens on a per-user Unix socket and relies on filesystem
permissions. The project treats these as security-relevant:

- daemon socket ownership/mode and state-directory permissions;
- path traversal in instance, backup, restore, or modpack handling;
- command construction and console-input validation;
- download URL allow-lists, redirect/DNS checks, and artifact hash checks;
- install/update manifest and artifact integrity;
- accidental disclosure of User-Agent or local paths in exported diagnostics.

## Signed releases

The current `v0.1.0` release is **unsigned**. The installer verifies the
release manifest's SHA-256 and file size, but it does not verify a code
signature. Do not describe these artifacts as signed or notarized. Phase 2 adds
signing, notarization, and package-repository verification.

## Secrets

Never commit tokens, private keys, world data, or personal server addresses.
If a secret is exposed, revoke it first; removing the commit is not enough.
