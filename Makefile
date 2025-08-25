# Makefile for ash-rpc workspace publishing
#
# This Makefile provides easy commands to publish the main packages:
# - ash-rpc-core
# - ash-rpc-stateful  
# - ash-rpc-cli

.PHONY: help publish-core publish-stateful publish-cli publish-all check-all test-all clean

# Default target
help:
	@echo "Available targets:"
	@echo "  publish-core     - Publish the core package"
	@echo "  publish-stateful - Publish the stateful package"
	@echo "  publish-cli      - Publish the cli package"
	@echo "  publish-all      - Publish all packages in dependency order"
	@echo "  check-all        - Run cargo check on all packages"
	@echo "  test-all         - Run cargo test on all packages"
	@echo "  clean            - Clean build artifacts"
	@echo "  help             - Show this help message"

# Check all packages before publishing
check-all:
	@echo "Checking all packages..."
	cargo check --workspace

# Test all packages before publishing
test-all:
	@echo "Testing all packages..."
	cargo test --workspace

# Publish core package (dependency for others)
publish-core: check-all test-all
	@echo "Publishing ash-rpc-core..."
	cd core && cargo publish

# Publish stateful package (depends on core)
publish-stateful: check-all test-all
	@echo "Publishing ash-rpc-stateful..."
	cd stateful && cargo publish

# Publish cli package (depends on core and stateful)
publish-cli: check-all test-all
	@echo "Publishing ash-rpc-cli..."
	cd cli && cargo publish

# Publish all packages in dependency order
publish-all: publish-core
	@echo "Waiting 30 seconds for core to propagate..."
	sleep 30
	$(MAKE) publish-stateful
	@echo "Waiting 30 seconds for stateful to propagate..."
	sleep 30
	$(MAKE) publish-cli
	@echo "All packages published successfully!"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean

# Dry run publishing (useful for testing)
dry-run-core:
	@echo "Dry run publishing ash-rpc-core..."
	cd core && cargo publish --dry-run

dry-run-stateful:
	@echo "Dry run publishing ash-rpc-stateful..."
	cd stateful && cargo publish --dry-run

dry-run-cli:
	@echo "Dry run publishing ash-rpc-cli..."
	cd cli && cargo publish --dry-run

dry-run-all: dry-run-core dry-run-stateful dry-run-cli
	@echo "All dry runs completed!"
