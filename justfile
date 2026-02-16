# Release automation for siteprobe

# List available commands
default:
    @just --list

# Extract version from Cargo.toml
version := `grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'`
repo := "bartTC/siteprobe"

# Build in debug mode
build:
    cargo build

# Build in release mode
build-release:
    cargo build --release

# Run clippy lints
lint:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Check formatting without modifying files
fmt-check:
    cargo fmt --check

# Run all checks (format, lint, test)
check: fmt-check lint test

# Clean build artifacts
clean:
    cargo clean

# Run tests. Use --cov for HTML coverage report.
test *args:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ "{{args}}" == *"--cov"* ]]; then
        cargo tarpaulin --out html
        open tarpaulin-report.html
    else
        cargo test
    fi

# Perform a full release: test, update changelog, commit, tag, push
release:
    #!/usr/bin/env bash
    set -euo pipefail

    echo "🚀 Preparing release of siteprobe v{{version}}"
    echo ""

    # 1. Ask for confirmation
    # Fail if tag already exists
    if git rev-parse "v{{version}}" >/dev/null 2>&1; then
        echo "Error: tag v{{version}} already exists."
        exit 1
    fi

    read -p "Release v{{version}}? [y/N] " confirm
    if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
        echo "Aborted."
        exit 1
    fi

    # 2. Run tests
    echo ""
    echo "Running tests..."
    cargo test
    echo "Tests passed."

    # 3. Update changelog: replace WIP with today's date
    today=$(date +%Y-%m-%d)
    if grep -q "WIP" CHANGELOG.md; then
        sed -i '' "s/WIP/$today/g" CHANGELOG.md
        echo "Updated CHANGELOG.md: WIP → $today"
    else
        echo "No WIP found in CHANGELOG.md, skipping."
    fi

    # 4. Create commit
    git add -A
    git commit -m "Release v{{version}}"

    # 5. Create tag
    git tag "v{{version}}"

    # 6. Push to GitHub
    git push
    git push --tags

    # 7. Show release workflow URL
    echo ""
    echo "Release v{{version}} pushed!"
    echo "Watch the release workflow:"
    echo "  https://github.com/{{repo}}/actions/workflows/release.yml"
