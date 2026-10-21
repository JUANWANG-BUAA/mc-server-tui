#!/bin/sh
set -eu

# Enhanced Phase 1 installer with pre-flight checks, progress feedback, and better error messages.
# This verifies the release manifest's SHA-256 and installs into a versioned directory.
# It does not yet verify signatures (Phase 2 work).

DEFAULT_MANIFEST_URL="https://github.com/JUANWANG-BUAA/mc-server-tui/releases/latest/download/release-manifest.json"
MANIFEST_URL="${MC_SERVER_TUI_MANIFEST_URL:-$DEFAULT_MANIFEST_URL}"
PREFIX="${HOME:-}/.local"
ROOT="${HOME:-}/.local/share/mc-server-tui"
CHANNEL="stable"
REQUESTED_VERSION=""
FORCE=0
DRY_RUN=0
TMP=""
VERBOSE=0
MIN_DISK_SPACE_MB=500  # Minimum disk space required in MB

# Color codes for better UX (disabled if NO_COLOR is set)
if [ -z "${NO_COLOR:-}" ] && [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    RESET='\033[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' BOLD='' RESET=''
fi

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
  --verbose            Show detailed progress information
  -h, --help           Show this help

After installing, run: <prefix>/bin/mc setup

Environment:
  MC_SERVER_TUI_MANIFEST_URL    Override manifest URL
  NO_COLOR                      Disable colored output
  TMPDIR                        Temporary directory (default: /tmp)

Examples:
  # Install latest stable release
  ./install.sh

  # Install specific version
  ./install.sh --version v0.2.1

  # Preview installation without executing
  ./install.sh --dry-run

  # Install to custom location
  ./install.sh --prefix /opt/local --root /opt/mc-server-tui
EOF
}

info() {
    printf "${BLUE}==>${RESET} ${BOLD}%s${RESET}\n" "$*"
}

success() {
    printf "${GREEN}✓${RESET} %s\n" "$*"
}

warn() {
    printf "${YELLOW}warning:${RESET} %s\n" "$*" >&2
}

error() {
    printf "${RED}error:${RESET} %s\n" "$*" >&2
}

die() {
    error "$*"
    exit 1
}

verbose() {
    if [ "$VERBOSE" -eq 1 ]; then
        printf "  %s\n" "$*"
    fi
}

cleanup() {
    if [ -n "$TMP" ] && [ -d "$TMP" ]; then
        verbose "Cleaning up temporary directory: $TMP"
        rm -rf "$TMP"
    fi
}
trap cleanup EXIT HUP INT TERM

check_command() {
    local cmd="$1"
    local purpose="$2"
    if ! command -v "$cmd" >/dev/null 2>&1; then
        die "$cmd is required $purpose"
    fi
    verbose "Found $cmd: $(command -v "$cmd")"
}

check_disk_space() {
    local target_dir="$1"
    local required_mb="$2"

    # Create parent directory if it doesn't exist for df check
    local check_dir="$target_dir"
    while [ ! -d "$check_dir" ] && [ "$check_dir" != "/" ]; do
        check_dir=$(dirname "$check_dir")
    done

    local available_kb
    if available_kb=$(df -k "$check_dir" 2>/dev/null | tail -1 | awk '{print $4}'); then
        local available_mb=$((available_kb / 1024))
        verbose "Available disk space: ${available_mb}MB (required: ${required_mb}MB)"
        if [ "$available_mb" -lt "$required_mb" ]; then
            die "Insufficient disk space: ${available_mb}MB available, ${required_mb}MB required"
        fi
    else
        warn "Could not check disk space for $check_dir; proceeding anyway"
    fi
}

check_java() {
    if command -v java >/dev/null 2>&1; then
        local java_version
        java_version=$(java -version 2>&1 | head -1 | cut -d'"' -f2 | cut -d'.' -f1)
        if [ -n "$java_version" ] && [ "$java_version" -ge 21 ] 2>/dev/null; then
            success "Java $java_version detected"
            return 0
        else
            warn "Java detected but version could not be determined or is < 21"
            warn "Paper 1.21.x requires Java 21+; install compatible Java before creating servers"
            return 1
        fi
    else
        warn "Java not found on PATH"
        warn "Install Java 21+ before creating Minecraft servers"
        info "  macOS: brew install openjdk@21"
        info "  Ubuntu/Debian: sudo apt install openjdk-21-jdk"
        info "  Fedora/RHEL: sudo dnf install java-21-openjdk"
        return 1
    fi
}

parse_args() {
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
            --verbose)
                VERBOSE=1
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
}

validate_environment() {
    info "Validating environment..."

    [ -n "${HOME:-}" ] || die "HOME is not set; pass --prefix and --root explicitly"
    case "$PREFIX" in /*) ;; *) die "--prefix must be absolute" ;; esac
    case "$ROOT" in /*) ;; *) die "--root must be absolute" ;; esac
    case "$ROOT" in /|"$HOME"|"$HOME"/) die "refusing unsafe install root: $ROOT" ;; esac
    case "$CHANNEL" in stable|beta) ;; *) die "--channel must be stable or beta" ;; esac

    success "Environment validated"
}

check_dependencies() {
    info "Checking dependencies..."

    check_command curl "to download release artifacts"
    check_command tar "to extract archives"
    check_command python3 "to parse the release manifest"

    # Check for SHA-256 tool
    if command -v sha256sum >/dev/null 2>&1; then
        verbose "SHA-256 verification: sha256sum"
    elif command -v shasum >/dev/null 2>&1; then
        verbose "SHA-256 verification: shasum"
    else
        die "no SHA-256 tool found (need sha256sum or shasum)"
    fi

    success "All required dependencies present"
}

detect_platform() {
    info "Detecting platform..."

    OS=$(uname -s)
    ARCH=$(uname -m)

    verbose "OS: $OS"
    verbose "Architecture: $ARCH"

    case "$OS:$ARCH" in
        Darwin:arm64)
            CANDIDATES="aarch64-apple-darwin"
            PLATFORM_NAME="macOS Apple Silicon"
            ;;
        Darwin:x86_64)
            CANDIDATES="x86_64-apple-darwin"
            PLATFORM_NAME="macOS Intel"
            warn "Intel macOS builds may not be available in all releases"
            ;;
        Linux:x86_64|Linux:amd64)
            CANDIDATES="x86_64-unknown-linux-musl x86_64-unknown-linux-gnu"
            PLATFORM_NAME="Linux x86_64"
            ;;
        Linux:aarch64|Linux:arm64)
            CANDIDATES="aarch64-unknown-linux-musl aarch64-unknown-linux-gnu"
            PLATFORM_NAME="Linux ARM64"
            ;;
        *)
            die "unsupported platform: $OS $ARCH (supported: macOS arm64/x86_64, Linux x86_64/arm64)"
            ;;
    esac

    success "Detected: $PLATFORM_NAME"
}

download_manifest() {
    info "Downloading release manifest..."
    verbose "URL: $MANIFEST_URL"

    TMP=$(mktemp -d "${TMPDIR:-/tmp}/mc-server-tui-install.XXXXXX")
    MANIFEST_FILE="$TMP/release-manifest.json"

    if ! curl -fsSL "$MANIFEST_URL" -o "$MANIFEST_FILE"; then
        error "Could not download release manifest"
        info "Troubleshooting:"
        info "  1. Check your internet connection"
        info "  2. Verify the URL is correct: $MANIFEST_URL"
        info "  3. Check GitHub status: https://www.githubstatus.com/"
        info "  4. Try again in a few moments"
        exit 1
    fi

    success "Manifest downloaded"
}

parse_manifest() {
    info "Parsing release manifest..."

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
    available = ", ".join(sorted(targets))
    raise SystemExit(
        f"manifest has no artifact for this platform; available: {available}"
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

    success "Release resolved: $VERSION ($TARGET)"
}

show_install_plan() {
    info "Installation plan:"
    printf "  ${BOLD}Version:${RESET}  %s (%s)\n" "$VERSION" "$CHANNEL_RESOLVED"
    printf "  ${BOLD}Platform:${RESET} %s\n" "$PLATFORM_NAME"
    printf "  ${BOLD}Target:${RESET}   %s\n" "$TARGET"
    if [ -n "$SIZE" ]; then
        SIZE_MB=$((SIZE / 1024 / 1024))
        printf "  ${BOLD}Size:${RESET}     %s MB\n" "$SIZE_MB"
    fi
    printf "  ${BOLD}Source:${RESET}   %s\n" "$URL"
    printf "  ${BOLD}Prefix:${RESET}   %s\n" "$PREFIX"
    printf "  ${BOLD}Root:${RESET}     %s\n" "$ROOT"
    printf "  ${BOLD}Launcher:${RESET} %s/bin/mc\n" "$PREFIX"
    echo
}

prepare_directories() {
    info "Preparing directories..."

    check_disk_space "$ROOT" "$MIN_DISK_SPACE_MB"

    mkdir -p "$PREFIX/bin" "$ROOT/versions" || die "could not create directories"

    verbose "Created: $PREFIX/bin"
    verbose "Created: $ROOT/versions"
    success "Directories ready"
}

check_existing_version() {
    if [ -e "$ROOT/versions/$VERSION" ]; then
        if [ "$FORCE" -ne 1 ]; then
            error "Version $VERSION is already installed"
            info "Installation directory: $ROOT/versions/$VERSION"
            info "Options:"
            info "  1. Use the existing installation: skip this installer"
            info "  2. Reinstall: run with --force to replace it"
            info "  3. Uninstall first: run 'mc uninstall'"
            exit 1
        else
            warn "Replacing existing version $VERSION (--force)"
        fi
    fi
}

download_release() {
    info "Downloading release artifact..."

    ARCHIVE="$TMP/mc-server-tui.tar.gz"

    if [ -n "$SIZE" ]; then
        SIZE_MB=$((SIZE / 1024 / 1024))
        verbose "Downloading ${SIZE_MB}MB..."
    fi

    if ! curl -fsSL "$URL" -o "$ARCHIVE"; then
        error "Could not download release artifact"
        info "Troubleshooting:"
        info "  1. Check your internet connection"
        info "  2. Verify you have sufficient disk space (${MIN_DISK_SPACE_MB}MB required)"
        info "  3. Try again: partial downloads are not resumed"
        info "  4. Check GitHub status: https://www.githubstatus.com/"
        exit 1
    fi

    success "Download complete"
}

verify_archive() {
    info "Verifying SHA-256 checksum..."

    if command -v sha256sum >/dev/null 2>&1; then
        ACTUAL_SHA=$(sha256sum "$ARCHIVE" | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        ACTUAL_SHA=$(shasum -a 256 "$ARCHIVE" | awk '{print $1}')
    else
        die "no SHA-256 tool found (this should not happen after dependency check)"
    fi

    verbose "Expected: $SHA256"
    verbose "Actual:   $ACTUAL_SHA"

    if [ "$ACTUAL_SHA" != "$SHA256" ]; then
        error "SHA-256 mismatch detected"
        info "Expected: $SHA256"
        info "Actual:   $ACTUAL_SHA"
        info "This indicates:"
        info "  • The download was corrupted"
        info "  • A network error occurred"
        info "  • A security issue (man-in-the-middle attack)"
        info "Recovery:"
        info "  1. Delete the cached file if any"
        info "  2. Run the installer again"
        info "  3. If this persists, report it as a security issue"
        exit 1
    fi

    if [ -n "$SIZE" ]; then
        ACTUAL_SIZE=$(wc -c < "$ARCHIVE" | tr -d ' ')
        if [ "$ACTUAL_SIZE" != "$SIZE" ]; then
            warn "Size mismatch: expected $SIZE bytes, got $ACTUAL_SIZE"
            warn "SHA-256 matches, so this may be acceptable (gzip compression variance)"
        fi
    fi

    success "Checksum verified"
    warn "This release is unsigned; only SHA-256 was verified (not a code signature)"
}

extract_archive() {
    info "Extracting archive..."

    mkdir -p "$TMP/extract"
    tar -xzf "$ARCHIVE" -C "$TMP/extract" || die "could not extract archive"

    SRC=$(find "$TMP/extract" -mindepth 1 -maxdepth 1 -type d | head -n 1)
    [ -n "$SRC" ] || die "release archive is empty"

    verbose "Extracted to: $SRC"

    # Verify required binaries
    for binary in mc mc-server-tui mc-server-tui-daemon mc-server-tui-cli; do
        if [ ! -x "$SRC/bin/$binary" ]; then
            die "release archive is missing executable bin/$binary"
        fi
        verbose "Found: bin/$binary"
    done

    success "Archive extracted and verified"
}

install_version() {
    info "Installing version $VERSION..."

    if [ -e "$ROOT/versions/$VERSION" ]; then
        verbose "Removing existing version directory"
        rm -rf "$ROOT/versions/$VERSION"
    fi

    mv "$SRC" "$ROOT/versions/$VERSION" || die "could not move extracted files"

    verbose "Installed to: $ROOT/versions/$VERSION"

    # Update version symlinks
    if [ -L "$ROOT/current" ]; then
        OLD=$(readlink "$ROOT/current" || echo "")
        if [ -n "$OLD" ]; then
            verbose "Updating previous -> $OLD"
            ln -sfn "$OLD" "$ROOT/previous"
        fi
    fi

    verbose "Updating current -> versions/$VERSION"
    ln -sfn "versions/$VERSION" "$ROOT/current"

    success "Version $VERSION installed"
}

install_launcher() {
    info "Installing launcher..."

    if [ -e "$PREFIX/bin/mc" ] && [ ! -L "$PREFIX/bin/mc" ] && [ "$FORCE" -ne 1 ]; then
        error "$PREFIX/bin/mc exists and is not a symlink"
        info "Options:"
        info "  1. Remove it manually if safe"
        info "  2. Run with --force to replace it"
        info "  3. Use a different --prefix"
        exit 1
    fi

    ln -sfn "$ROOT/current/bin/mc" "$PREFIX/bin/mc"

    verbose "Launcher: $PREFIX/bin/mc -> $ROOT/current/bin/mc"
    success "Launcher installed"
}

check_path() {
    case ":${PATH:-}:" in
        *":$PREFIX/bin:"*)
            success "$PREFIX/bin is in PATH"
            return 0
            ;;
        *)
            warn "$PREFIX/bin is not in PATH"
            info "Add it to your shell profile:"
            info "  ${BOLD}export PATH=\"$PREFIX/bin:\$PATH\"${RESET}"
            echo
            info "Or run the full path: ${BOLD}$PREFIX/bin/mc setup${RESET}"
            return 1
            ;;
    esac
}

show_next_steps() {
    echo
    info "${BOLD}Installation complete!${RESET}"
    echo
    info "Next steps:"
    printf "  ${BOLD}1.${RESET} Run setup (creates config, installs service, checks Java):\n"
    printf "     ${BOLD}mc setup${RESET}\n"
    echo
    printf "  ${BOLD}2.${RESET} Open the TUI:\n"
    printf "     ${BOLD}mc${RESET}\n"
    echo
    info "Useful commands:"
    printf "  ${BOLD}mc version${RESET}         Show installed version\n"
    printf "  ${BOLD}mc doctor${RESET}          Check system requirements\n"
    printf "  ${BOLD}mc setup --dry-run${RESET} Preview setup without changes\n"
    printf "  ${BOLD}mc --help${RESET}          Show all available commands\n"
    echo
    info "Documentation: https://juanwang-buaa.github.io/mc-server-tui/"
}

main() {
    parse_args "$@"

    echo "${BOLD}mc-server-tui installer${RESET}"
    echo

    validate_environment
    check_dependencies
    detect_platform
    download_manifest
    parse_manifest

    show_install_plan

    if [ "$DRY_RUN" -eq 1 ]; then
        warn "Dry run: no files were downloaded or changed"
        exit 0
    fi

    prepare_directories
    check_existing_version
    download_release
    verify_archive
    extract_archive
    install_version
    install_launcher

    echo
    check_java || true
    check_path || true

    show_next_steps
}

main "$@"
