.PHONY: help publish-core publish-stateful publish-cli publish-all check-all test-all clean tag tag-version bump-version

help:
	@echo "available targets:"
	@echo "  bump-version     - bump all crate versions with VERSION=0.2.0"
	@echo "  tag              - create a new git tag for the next version (interactive)"
	@echo "  tag-version      - create a new git tag with VERSION=v0.1.0"
	@echo "  publish-core     - publish the core package"
	@echo "  publish-stateful - publish the stateful package"
	@echo "  publish-cli      - publish the cli package"
	@echo "  publish-all      - publish all packages in dependency order"
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
	@sed -i.bak 's/^version = ".*"/version = "$(VERSION)"/' cli/Cargo.toml
	@sed -i.bak 's/ash-rpc-core = { version = "[^"]*"/ash-rpc-core = { version = "$(VERSION)"/g' stateful/Cargo.toml
	@sed -i.bak 's/ash-rpc-core = { version = "[^"]*"/ash-rpc-core = { version = "$(VERSION)"/g' cli/Cargo.toml
	@rm -f core/Cargo.toml.bak stateful/Cargo.toml.bak cli/Cargo.toml.bak
	@echo "updated all crate versions to $(VERSION)"
	@echo "run 'git add . && git commit -m \"bump version to $(VERSION)\"' to commit changes"

check-all:
	@echo "checking all packages..."
	cargo check --workspace

test-all:
	@echo "testing all packages..."
	cargo test --workspace

publish-core: check-all test-all
	@echo "publishing ash-rpc-core..."
	cd core && cargo publish

publish-stateful: check-all test-all
	@echo "publishing ash-rpc-stateful..."
	cd stateful && cargo publish

publish-cli: check-all test-all
	@echo "publishing ash-rpc-cli..."
	cd cli && cargo publish

publish-all: publish-core
	@echo "waiting 30 seconds for core to propagate..."
	sleep 30
	$(MAKE) publish-stateful
	@echo "waiting 30 seconds for stateful to propagate..."
	sleep 30
	$(MAKE) publish-cli
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

dry-run-cli:
	@echo "dry run publishing ash-rpc-cli..."
	cd cli && cargo publish --dry-run

dry-run-all: dry-run-core dry-run-stateful dry-run-cli
	@echo "all dry runs completed!"
