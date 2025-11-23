.PHONY: help publish-core publish-contrib publish-all check-all test-all clean tag tag-version \
        bump-version check-login check-branch release release-patch release-minor release-major \
        dry-run-core dry-run-contrib dry-run-all

CURRENT_VERSION := $(shell grep '^version = ' core/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

check-login:
	@echo "Checking cargo login status..."
	@if ! cargo login --help > /dev/null 2>&1; then \
		echo "Error: cargo is not installed or not in PATH"; \
		exit 1; \
	fi
	@if ! cargo owner --list ash-rpc-core 2>/dev/null | grep -q .; then \
		echo "Error: not logged in to crates.io or no permission to query packages"; \
		echo "Please run 'cargo login' first"; \
		exit 1; \
	fi
	@echo "✓ Cargo login verified"

check-branch:
	@echo "Checking git branch..."
	@CURRENT_BRANCH=$$(git rev-parse --abbrev-ref HEAD); \
	if [ "$$CURRENT_BRANCH" != "master" ]; then \
		echo "Error: publishing must be done from master branch"; \
		echo "Current branch: $$CURRENT_BRANCH"; \
		echo "Please switch to master branch first: git checkout master"; \
		exit 1; \
	fi
	@echo "✓ On master branch"

help:
	@echo "Available targets:"
	@echo ""
	@echo "Release Management:"
	@echo "  release-patch    - Bump patch version, commit, tag, and publish (e.g., 1.0.4 -> 1.0.5)"
	@echo "  release-minor    - Bump minor version, commit, tag, and publish (e.g., 1.0.4 -> 1.1.0)"
	@echo "  release-major    - Bump major version, commit, tag, and publish (e.g., 1.0.4 -> 2.0.0)"
	@echo "  release          - Full release with custom VERSION=x.y.z"
	@echo ""
	@echo "Version Management:"
	@echo "  bump-version     - Bump all crate versions (requires VERSION=x.y.z)"
	@echo "  tag              - Create a new git tag (interactive)"
	@echo "  tag-version      - Create a new git tag (requires VERSION=vx.y.z)"
	@echo ""
	@echo "Publishing:"
	@echo "  publish-core     - Publish ash-rpc-core package"
	@echo "  publish-contrib  - Publish ash-rpc-contrib package"
	@echo "  publish-all      - Publish all packages in dependency order"
	@echo ""
	@echo "Dry Run:"
	@echo "  dry-run-core     - Dry run publish for core"
	@echo "  dry-run-contrib  - Dry run publish for contrib"
	@echo "  dry-run-all      - Dry run publish for all packages"
	@echo ""
	@echo "Checks & Testing:"
	@echo "  check-all        - Run cargo check on all packages"
	@echo "  test-all         - Run cargo test on all packages"
	@echo "  check-branch     - Verify on master branch"
	@echo "  check-login      - Verify cargo login status"
	@echo ""
	@echo "Utilities:"
	@echo "  clean            - Clean build artifacts"
	@echo "  help             - Show this help message"
	@echo ""
	@echo "Current version: $(CURRENT_VERSION)"

tag:
	@echo "Enter version (e.g., v1.0.5):"
	@read -r version; \
	git tag $$version && \
	git push origin $$version && \
	echo "Created and pushed tag: $$version"

tag-version:
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make tag-version VERSION=v1.0.5"; \
		exit 1; \
	fi
	@git tag $(VERSION) && \
	git push origin $(VERSION) && \
	echo "Created and pushed tag: $(VERSION)"

bump-version:
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make bump-version VERSION=1.0.5"; \
		exit 1; \
	fi
	@echo "Bumping all crate versions to $(VERSION)..."
	@sed -i.bak 's/^version = ".*"/version = "$(VERSION)"/' core/Cargo.toml
	@sed -i.bak 's/^version = ".*"/version = "$(VERSION)"/' contrib/Cargo.toml
	@sed -i.bak 's/ash-rpc-core = { version = "[^"]*"/ash-rpc-core = { version = "$(VERSION)"/g' contrib/Cargo.toml
	@sed -i.bak 's/ash-rpc-core = { version = "[^"]*"/ash-rpc-core = { version = "$(VERSION)"/g' examples/Cargo.toml
	@rm -f core/Cargo.toml.bak contrib/Cargo.toml.bak examples/Cargo.toml.bak
	@echo "✓ Updated all crate versions to $(VERSION)"
	@echo "To commit: git add . && git commit -m \"Bump version to $(VERSION)\""

bump-patch:
	@echo "Current version: $(CURRENT_VERSION)"
	@MAJOR=$$(echo $(CURRENT_VERSION) | cut -d. -f1); \
	MINOR=$$(echo $(CURRENT_VERSION) | cut -d. -f2); \
	PATCH=$$(echo $(CURRENT_VERSION) | cut -d. -f3); \
	NEW_PATCH=$$((PATCH + 1)); \
	NEW_VERSION="$$MAJOR.$$MINOR.$$NEW_PATCH"; \
	echo "Bumping to: $$NEW_VERSION"; \
	$(MAKE) bump-version VERSION=$$NEW_VERSION

bump-minor:
	@echo "Current version: $(CURRENT_VERSION)"
	@MAJOR=$$(echo $(CURRENT_VERSION) | cut -d. -f1); \
	MINOR=$$(echo $(CURRENT_VERSION) | cut -d. -f2); \
	NEW_MINOR=$$((MINOR + 1)); \
	NEW_VERSION="$$MAJOR.$$NEW_MINOR.0"; \
	echo "Bumping to: $$NEW_VERSION"; \
	$(MAKE) bump-version VERSION=$$NEW_VERSION

bump-major:
	@echo "Current version: $(CURRENT_VERSION)"
	@MAJOR=$$(echo $(CURRENT_VERSION) | cut -d. -f1); \
	NEW_MAJOR=$$((MAJOR + 1)); \
	NEW_VERSION="$$NEW_MAJOR.0.0"; \
	echo "Bumping to: $$NEW_VERSION"; \
	$(MAKE) bump-version VERSION=$$NEW_VERSION

check-all:
	@echo "Checking all packages..."
	@cargo check --workspace --all-features

test-all:
	@echo "Testing all packages..."
	@cargo test --workspace --all-features

publish-core: check-branch check-login check-all test-all
	@echo "Publishing ash-rpc-core..."
	cd core && cargo publish

publish-contrib: check-branch check-login check-all test-all
	@echo "Publishing ash-rpc-contrib..."
	cd contrib && cargo publish

publish-all: check-branch check-login check-all test-all
	@echo "Publishing ash-rpc-core..."
	cd core && cargo publish
	@echo "Waiting 45 seconds for core to propagate on crates.io..."
	@sleep 45
	@echo "Publishing ash-rpc-contrib..."
	cd contrib && cargo publish
	@echo "✓ All packages published successfully!"

clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@rm -rf target/

dry-run-core:
	@echo "Dry run publishing ash-rpc-core..."
	cd core && cargo publish --dry-run

dry-run-contrib:
	@echo "Dry run publishing ash-rpc-contrib..."
	cd contrib && cargo publish --dry-run

dry-run-all: dry-run-core dry-run-contrib
	@echo "✓ All dry runs completed!"

release: check-branch
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make release VERSION=1.0.5"; \
		exit 1; \
	fi
	@echo "=== Starting release process for version $(VERSION) ==="
	@echo ""
	@echo "Step 1: Bumping version to $(VERSION)..."
	@$(MAKE) bump-version VERSION=$(VERSION)
	@echo ""
	@echo "Step 2: Committing version bump..."
	@git add . && git commit -m "Bump version to $(VERSION)"
	@echo ""
	@echo "Step 3: Creating and pushing tag v$(VERSION)..."
	@$(MAKE) tag-version VERSION=v$(VERSION)
	@echo ""
	@echo "Step 4: Publishing all packages..."
	@$(MAKE) publish-all
	@echo ""
	@echo "=== Release $(VERSION) completed successfully! ==="

release-patch: check-branch
	@echo "=== Starting PATCH release ==="
	@echo "Current version: $(CURRENT_VERSION)"
	@MAJOR=$$(echo $(CURRENT_VERSION) | cut -d. -f1); \
	MINOR=$$(echo $(CURRENT_VERSION) | cut -d. -f2); \
	PATCH=$$(echo $(CURRENT_VERSION) | cut -d. -f3); \
	NEW_PATCH=$$((PATCH + 1)); \
	NEW_VERSION="$$MAJOR.$$MINOR.$$NEW_PATCH"; \
	echo "New version: $$NEW_VERSION"; \
	echo ""; \
	$(MAKE) release VERSION=$$NEW_VERSION

release-minor: check-branch
	@echo "=== Starting MINOR release ==="
	@echo "Current version: $(CURRENT_VERSION)"
	@MAJOR=$$(echo $(CURRENT_VERSION) | cut -d. -f1); \
	MINOR=$$(echo $(CURRENT_VERSION) | cut -d. -f2); \
	NEW_MINOR=$$((MINOR + 1)); \
	NEW_VERSION="$$MAJOR.$$NEW_MINOR.0"; \
	echo "New version: $$NEW_VERSION"; \
	echo ""; \
	$(MAKE) release VERSION=$$NEW_VERSION

release-major: check-branch
	@echo "=== Starting MAJOR release ==="
	@echo "Current version: $(CURRENT_VERSION)"
	@MAJOR=$$(echo $(CURRENT_VERSION) | cut -d. -f1); \
	NEW_MAJOR=$$((MAJOR + 1)); \
	NEW_VERSION="$$NEW_MAJOR.0.0"; \
	echo "New version: $$NEW_VERSION"; \
	echo ""; \
	echo "⚠️  WARNING: This is a MAJOR version bump!"; \
	echo "Press Ctrl+C to cancel, or Enter to continue..."; \
	@read -r confirm; \
	MAJOR=$$(echo $(CURRENT_VERSION) | cut -d. -f1); \
	NEW_MAJOR=$$((MAJOR + 1)); \
	NEW_VERSION="$$NEW_MAJOR.0.0"; \
	$(MAKE) release VERSION=$$NEW_VERSION
