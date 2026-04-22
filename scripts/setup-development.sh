#!/usr/bin/env bash

set -eu -o pipefail

cd "$(dirname "$0")"

# Runs a command to check a tool's version against a minimum requirement.
# Exits with an error if the tool is missing or below the required version.
# Usage: require_version <name> <command> <req_major> [req_minor]
require_version() {
    local name="$1"
    local cmd="$2"
    local req_major="$3"
    local req_minor="${4:-0}"

    local raw
    if ! raw=$(eval "$cmd" 2>&1); then
        echo "Error: $name is not installed. Please install $name to continue."
        exit 1
    fi

    local full
    full=$(echo "$raw" | grep -oE '[0-9]+\.[0-9]+(\.[0-9]+)?' | head -1)
    local major="${full%%.*}"
    local minor
    minor=$(echo "$full" | cut -d. -f2)

    if [ "$major" -lt "$req_major" ] || { [ "$major" -eq "$req_major" ] && [ "$minor" -lt "$req_minor" ]; }; then
        echo "Error: $name version ${req_major}.${req_minor} or higher is required. Current version: $raw. Please update your $name installation."
        exit 1
    fi

    echo "$name version $full is OK"
}

# ┏━╸╻╺┳╸
# ┃╺┓┃ ┃
# ┗━┛╹ ╹
echo "... Checking git version"
require_version "Git" "git --version" 2 50

# ┏━┓╻ ╻┏━┓╺┳╸
# ┣┳┛┃ ┃┗━┓ ┃
# ╹┗╸┗━┛┗━┛ ╹
# Check rust version
echo "... Checking Rust version"
require_version "Rust" "rustc --version" 1 95

# ┏┓╻┏━┓╺┳┓┏━╸
# ┃┗┫┃ ┃ ┃┃┣╸
# ╹ ╹┗━┛╺┻┛┗━╸
echo "... Checking Node.js version"
require_version "node" "node -v" 24 15

# Install dependencies (specifically git-shit-done-cc)
echo "... Running npm install"
npm install

# ┏━┓┏━┓┏━╸╻┏
# ┣━┛┣┳┛┣╸ ┣┻┓
# ╹  ╹┗╸┗━╸╹ ╹
echo "... Checking prek version"
require_version "prek" "prek --version" 0 3

# ┏━╸╻╺┳╸   ╻ ╻┏━┓┏━┓╻┏ ┏━┓
# ┃╺┓┃ ┃    ┣━┫┃ ┃┃ ┃┣┻┓┗━┓
# ┗━┛╹ ╹    ╹ ╹┗━┛┗━┛╹ ╹┗━┛
# Install git hooks via prek. Hooks enforce linting and tests on commit/push,
# and validate conventional commit message formatting.
echo "... Installing prek hooks"
prek install \
    --prepare-hooks \
    -t pre-commit \
    -t pre-push \
    -t commit-msg
