#!/bin/sh
set -eu

# Phase 1 user-local installer. It verifies the release manifest's SHA-256 and
# installs into a versioned directory, but it does not yet verify signatures.
# Do not present this as a signed production installer until Phase 2.

DEFAULT_MANIFEST_URL="https://github.com/JUANWANG-BUAA/mc-server-tui/releases/latest/download/release-manifest.json"
MANIFEST_URL="${MC_SERVER_TUI_MANIFEST_URL:-$DEFAULT_MANIFEST_URL}"
PREFIX="${HOME:-}/.local"
ROOT="${HOME:-}/.local/share/mc-server-tui"
CHANNEL="stable"
REQUESTED_VERSION=""
FORCE=0
DRY_RUN=0
TMP=""

usage() {
    cat <<'EOF'
Usage: install.sh [options]

Install mc-server-tui from a release manifest into a versioned directory.

Options:
  --manifest-url URL   Release manifest URL (env: MC_SERVER_TUI_MANIFEST_URL)
  --version VERSION    Require an exact version (default: manifest version)
  --channel CHANNEL    stable or beta (default: stable)
  --prefix DIR         Stable launcher directory (default: ~/.local)
  --root DIR           Versioned program directory (default: ~/.local/share/mc-server-tui)
  --force              Replace an existing version/launcher
  --dry-run            Resolve and print the plan without downloading
  -h, --help           Show this help

After installing, run: <prefix>/bin/mc setup
EOF
}

die() {
    printf 'install: %s\n' "$*" >&2
    exit 1
}

cleanup() {
    if [ -n "$TMP" ] && [ -d "$TMP" ]; then
        rm -rf "$TMP"
    fi
}
trap cleanup EXIT HUP INT TERM

while [ "$#" -gt 0 ]; do
    case "$1" in
        --manifest-url)
            [ "$#" -ge 2 ] || die "--manifest-url needs a value"
            MANIFEST_URL="$2"
            shift 2
            ;;
        --version)
            [ "$#" -ge 2 ] || die "--version needs a value"
            REQUESTED_VERSION="$2"
            shift 2
            ;;
        --channel)
            [ "$#" -ge 2 ] || die "--channel needs a value"
            CHANNEL="$2"
            shift 2
            ;;
        --prefix)
            [ "$#" -ge 2 ] || die "--prefix needs a value"
            PREFIX="$2"
            shift 2
            ;;
        --root)
            [ "$#" -ge 2 ] || die "--root needs a value"
            ROOT="$2"
            shift 2
            ;;
        --force)
            FORCE=1
            shift
            ;;
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            die "unknown option: $1 (try --help)"
            ;;
    esac
done

[ -n "${HOME:-}" ] || die "HOME is not set; pass --prefix and --root explicitly"
case "$PREFIX" in /*) ;; *) die "--prefix must be absolute" ;; esac
case "$ROOT" in /*) ;; *) die "--root must be absolute" ;; esac
case "$ROOT" in /|"$HOME"|"$HOME"/) die "refusing unsafe install root: $ROOT" ;; esac
case "$CHANNEL" in stable|beta) ;; *) die "--channel must be stable or beta" ;; esac

command -v curl >/dev/null 2>&1 || die "curl is required"
command -v tar >/dev/null 2>&1 || die "tar is required"
command -v python3 >/dev/null 2>&1 || die "python3 is required to read the release manifest"

OS=$(uname -s)
ARCH=$(uname -m)
case "$OS:$ARCH" in
    Darwin:arm64) CANDIDATES="aarch64-apple-darwin" ;;
    Darwin:x86_64) CANDIDATES="x86_64-apple-darwin" ;;
    Linux:x86_64|Linux:amd64) CANDIDATES="x86_64-unknown-linux-musl x86_64-unknown-linux-gnu" ;;
    Linux:aarch64|Linux:arm64) CANDIDATES="aarch64-unknown-linux-musl aarch64-unknown-linux-gnu" ;;
    *) die "unsupported platform: $OS $ARCH" ;;
esac

TMP=$(mktemp -d "${TMPDIR:-/tmp}/mc-server-tui-install.XXXXXX")
MANIFEST_FILE="$TMP/release-manifest.json"
curl -fsSL "$MANIFEST_URL" -o "$MANIFEST_FILE" || die "could not download release manifest: $MANIFEST_URL"

META=$(python3 - "$MANIFEST_FILE" "$MANIFEST_URL" "$CHANNEL" "$REQUESTED_VERSION" "$CANDIDATES" <<'PY'
import json
import sys
import urllib.parse

manifest_path, manifest_url, channel, requested, candidates = sys.argv[1:6]
try:
    with open(manifest_path, "r", encoding="utf-8") as handle:
        data = json.load(handle)
except Exception as error:
    raise SystemExit(f"invalid manifest: {error}")

if data.get("schema") != 1:
    raise SystemExit("unsupported release-manifest schema")
if data.get("channel") != channel:
    raise SystemExit(
        f"manifest channel is {data.get('channel')!r}, not {channel!r}"
    )
if requested and data.get("version") != requested:
    raise SystemExit(
        f"manifest version is {data.get('version')!r}, not requested {requested!r}"
    )

targets = data.get("targets") or {}
target = next((item for item in candidates.split() if item in targets), None)
if target is None:
    raise SystemExit(
        "manifest has no artifact for this platform; available: "
        + ", ".join(sorted(targets))
    )
item = targets[target]
for field in ("url", "sha256"):
    if not item.get(field):
        raise SystemExit(f"manifest target is missing {field}")
url = urllib.parse.urljoin(manifest_url, item["url"])
print(data["version"])
print(target)
print(url)
print(item["sha256"])
print(item.get("size", ""))
print(data.get("channel", ""))
PY
) || die "could not resolve a release target"

VERSION=$(printf '%s\n' "$META" | sed -n '1p')
TARGET=$(printf '%s\n' "$META" | sed -n '2p')
URL=$(printf '%s\n' "$META" | sed -n '3p')
SHA256=$(printf '%s\n' "$META" | sed -n '4p')
SIZE=$(printf '%s\n' "$META" | sed -n '5p')
CHANNEL_RESOLVED=$(printf '%s\n' "$META" | sed -n '6p')

case "$VERSION" in *[!A-Za-z0-9._+-]*) die "manifest version contains unsafe characters" ;; esac
[ -n "$TARGET" ] && [ -n "$URL" ] && [ -n "$SHA256" ] || die "manifest target is incomplete"

printf 'release: %s (%s, %s)\n' "$VERSION" "$TARGET" "$CHANNEL_RESOLVED"
printf 'url:     %s\n' "$URL"
printf 'prefix:  %s\n' "$PREFIX"
printf 'root:    %s\n' "$ROOT"

if [ "$DRY_RUN" -eq 1 ]; then
    printf 'dry-run: no files were downloaded or changed\n'
    exit 0
fi

mkdir -p "$PREFIX/bin" "$ROOT/versions"

if [ -e "$ROOT/versions/$VERSION" ] && [ "$FORCE" -ne 1 ]; then
    die "version $VERSION is already installed under $ROOT; use --force to replace it"
fi

ARCHIVE="$TMP/mc-server-tui.tar.gz"
curl -fsSL "$URL" -o "$ARCHIVE" || die "could not download release artifact: $URL"

if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL_SHA=$(sha256sum "$ARCHIVE" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
    ACTUAL_SHA=$(shasum -a 256 "$ARCHIVE" | awk '{print $1}')
else
    die "no SHA-256 tool found (need sha256sum or shasum)"
fi
[ "$ACTUAL_SHA" = "$SHA256" ] || die "SHA-256 mismatch: expected $SHA256, got $ACTUAL_SHA"

if [ -n "$SIZE" ]; then
    ACTUAL_SIZE=$(wc -c < "$ARCHIVE" | tr -d ' ')
    [ "$ACTUAL_SIZE" = "$SIZE" ] || die "size mismatch: expected $SIZE bytes, got $ACTUAL_SIZE"
fi

printf 'warning: release manifest has no signature; only SHA-256 was verified\n' >&2

mkdir -p "$TMP/extract"
tar -xzf "$ARCHIVE" -C "$TMP/extract"
SRC=$(find "$TMP/extract" -mindepth 1 -maxdepth 1 -type d | head -n 1)
[ -n "$SRC" ] || die "release archive is empty"
for binary in mc mc-server-tui mc-server-tui-daemon mc-server-tui-cli; do
    [ -x "$SRC/bin/$binary" ] || die "release archive is missing executable bin/$binary"
done

if [ -e "$ROOT/versions/$VERSION" ]; then
    rm -rf "$ROOT/versions/$VERSION"
fi
mv "$SRC" "$ROOT/versions/$VERSION"

if [ -L "$ROOT/current" ]; then
    OLD=$(readlink "$ROOT/current")
    ln -sfn "$OLD" "$ROOT/previous"
fi
ln -sfn "versions/$VERSION" "$ROOT/current"

if [ -e "$PREFIX/bin/mc" ] && [ ! -L "$PREFIX/bin/mc" ] && [ "$FORCE" -ne 1 ]; then
    die "$PREFIX/bin/mc exists and is not a symlink; use --force to replace it"
fi
ln -sfn "$ROOT/current/bin/mc" "$PREFIX/bin/mc"

printf 'installed mc-server-tui %s (%s)\n' "$VERSION" "$TARGET"
printf 'launcher: %s\n' "$PREFIX/bin/mc"
case ":${PATH:-}:" in
    *":$PREFIX/bin:"*) ;;
    *) printf 'add to PATH: export PATH="%s/bin:$PATH"\n' "$PREFIX" ;;
esac
printf 'next: %s/bin/mc setup\n' "$PREFIX"
