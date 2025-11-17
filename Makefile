.PHONY: help publish-core publish-stateful publish-contrib publish-cli publish-all check-all test-all clean tag tag-version bump-version check-login check-branch release

check-login:
	@echo "checking cargo login status..."
	@if ! cargo login --help > /dev/null 2>&1; then \
		echo "error: cargo is not installed or not in PATH"; \
		exit 1; \
	fi
	@if ! cargo owner --list ash-rpc-core 2>/dev/null | grep -q .; then \
		echo "error: not logged in to crates.io or no permission to query packages"; \
		echo "please run 'cargo login' first"; \
		exit 1; \
	fi
	@echo "✓ cargo login verified"

check-branch:
	@echo "checking git branch..."
	@CURRENT_BRANCH=$$(git rev-parse --abbrev-ref HEAD); \
	if [ "$$CURRENT_BRANCH" != "master" ]; then \
		echo "error: publishing must be done from master branch"; \
		echo "current branch: $$CURRENT_BRANCH"; \
		echo "please switch to master branch first: git checkout master"; \
		exit 1; \
	fi
	@echo "✓ on master branch"

help:
	@echo "available targets:"
	@echo "  release          - bump version, commit, tag, and publish all (VERSION=0.2.0)"
	@echo "  bump-version     - bump all crate versions with VERSION=0.2.0"
	@echo "  tag              - create a new git tag for the next version (interactive)"
	@echo "  tag-version      - create a new git tag with VERSION=v0.1.0"
	@echo "  publish-core     - publish the core package"
	@echo "  publish-stateful - publish the stateful package"
	@echo "  publish-contrib  - publish the contrib package"
	@echo "  publish-cli      - publish the cli package"
	@echo "  publish-all      - publish all packages in dependency order"
	@echo "  check-branch     - verify on master branch"
	@echo "  check-login      - verify cargo login status"
	@echo "  check-all        - run cargo check on all packages"
	@echo "  test-all         - run cargo test on all packages"
	@echo "  clean            - clean build artifacts"
	@echo "  help             - show this help message"

tag:
	@echo "enter version (e.g., v0.1.0):"
	@read -r version; \
	git tag $$version && \
	git push origin $$version && \
	echo "created and pushed tag: $$version"

tag-version:
	@if [ -z "$(VERSION)" ]; then \
		echo "usage: make tag-version VERSION=v0.1.0"; \
		exit 1; \
	fi
	@git tag $(VERSION) && \
	git push origin $(VERSION) && \
	echo "created and pushed tag: $(VERSION)"

bump-version:
	@if [ -z "$(VERSION)" ]; then \
		echo "usage: make bump-version VERSION=0.2.0"; \
		exit 1; \
	fi
	@echo "bumping all crate versions to $(VERSION)..."
	@sed -i.bak 's/^version = ".*"/version = "$(VERSION)"/' core/Cargo.toml
	@sed -i.bak 's/^version = ".*"/version = "$(VERSION)"/' stateful/Cargo.toml
	@sed -i.bak 's/^version = ".*"/version = "$(VERSION)"/' contrib/Cargo.toml
	@sed -i.bak 's/^version = ".*"/version = "$(VERSION)"/' cli/Cargo.toml
	@sed -i.bak 's/ash-rpc-core = { version = "[^"]*"/ash-rpc-core = { version = "$(VERSION)"/g' stateful/Cargo.toml
	@sed -i.bak 's/ash-rpc-core = { version = "[^"]*"/ash-rpc-core = { version = "$(VERSION)"/g' contrib/Cargo.toml
	@sed -i.bak 's/ash-rpc-core = { version = "[^"]*"/ash-rpc-core = { version = "$(VERSION)"/g' cli/Cargo.toml
	@rm -f core/Cargo.toml.bak stateful/Cargo.toml.bak contrib/Cargo.toml.bak cli/Cargo.toml.bak
	@echo "updated all crate versions to $(VERSION)"
	@echo "run 'git add . && git commit -m \"bump version to $(VERSION)\"' to commit changes"

check-all:
	@echo "checking all packages..."
	cargo check --workspace

test-all:
	@echo "testing all packages..."
	cargo test --workspace

publish-core: check-branch check-login check-all test-all
	@echo "publishing ash-rpc-core..."
	cd core && cargo publish

publish-stateful: check-branch check-login check-all test-all
	@echo "publishing ash-rpc-stateful..."
	cd stateful && cargo publish

publish-contrib: check-branch check-login check-all test-all
	@echo "publishing ash-rpc-contrib..."
	cd contrib && cargo publish

publish-cli: check-branch check-login check-all test-all
	@echo "publishing ash-rpc-cli..."
	cd cli && cargo publish

publish-all: check-branch check-login check-all test-all
	@echo "publishing ash-rpc-core..."
	cd core && cargo publish
	@echo "waiting 30 seconds for core to propagate..."
	sleep 30
	@echo "publishing ash-rpc-stateful..."
	cd stateful && cargo publish
	@echo "waiting 30 seconds for stateful to propagate..."
	sleep 30
	@echo "publishing ash-rpc-contrib..."
	cd contrib && cargo publish
	@echo "waiting 30 seconds for contrib to propagate..."
	sleep 30
	@echo "publishing ash-rpc-cli..."
	cd cli && cargo publish
	@echo "all packages published successfully!"

clean:
	@echo "cleaning build artifacts..."
	cargo clean

dry-run-core:
	@echo "dry run publishing ash-rpc-core..."
	cd core && cargo publish --dry-run

dry-run-stateful:
	@echo "dry run publishing ash-rpc-stateful..."
	cd stateful && cargo publish --dry-run

dry-run-contrib:
	@echo "dry run publishing ash-rpc-contrib..."
	cd contrib && cargo publish --dry-run

dry-run-cli:
	@echo "dry run publishing ash-rpc-cli..."
	cd cli && cargo publish --dry-run

dry-run-all: dry-run-core dry-run-stateful dry-run-contrib dry-run-cli
	@echo "all dry runs completed!"

release: check-branch
	@if [ -z "$(VERSION)" ]; then \
		echo "usage: make release VERSION=0.2.0"; \
		exit 1; \
	fi
	@echo "=== Starting release process for version $(VERSION) ==="
	@echo ""
	@echo "Step 1: Bumping version to $(VERSION)..."
	@$(MAKE) bump-version VERSION=$(VERSION)
	@echo ""
	@echo "Step 2: Committing version bump..."
	@git add . && git commit -m "bump version to $(VERSION)"
	@echo ""
	@echo "Step 3: Creating and pushing tag v$(VERSION)..."
	@$(MAKE) tag-version VERSION=v$(VERSION)
	@echo ""
	@echo "Step 4: Publishing all packages..."
	@$(MAKE) publish-all
	@echo ""
	@echo "=== Release $(VERSION) completed successfully! ==="
