.PHONY: dev build test lint fmt check release install setup-hooks clean

# Run frontend + backend in parallel for development
dev:
	@echo "Starting dev servers..."
	@trap 'kill 0' EXIT; \
		cd frontend && npm run dev & \
		cargo run -- server & \
		wait

# Build frontend then cargo build --release
build:
	cd frontend && npm ci && npm run build
	cargo build --release

# Run all tests
test:
	cargo test
	cd frontend && npm run test

# Lint everything
lint:
	cargo clippy -- -D warnings
	cd frontend && npm run lint

# Format everything
fmt:
	cargo fmt
	cd frontend && npx prettier --write 'src/**/*.{ts,tsx,css}'

# Check formatting (CI-friendly, no writes)
check:
	cargo fmt -- --check
	cd frontend && npx prettier --check 'src/**/*.{ts,tsx,css}'

# Tag and push to trigger release workflow
release:
	@version=$$(cargo metadata --format-version 1 --no-deps | grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4); \
	echo "Tagging v$$version..."; \
	git tag "v$$version" && git push origin "v$$version"

# Build and install to /usr/local/bin
install: build
	cp target/release/conductor /usr/local/bin/conductor
	@echo "Installed conductor to /usr/local/bin/conductor"

# Install git pre-commit hook
setup-hooks:
	@echo '#!/bin/sh' > .git/hooks/pre-commit
	@echo 'set -e' >> .git/hooks/pre-commit
	@echo '' >> .git/hooks/pre-commit
	@echo '# Rust formatting' >> .git/hooks/pre-commit
	@echo 'cargo fmt -- --check' >> .git/hooks/pre-commit
	@echo '' >> .git/hooks/pre-commit
	@echo '# Rust linting' >> .git/hooks/pre-commit
	@echo 'cargo clippy -- -D warnings' >> .git/hooks/pre-commit
	@echo '' >> .git/hooks/pre-commit
	@echo '# Frontend linting and formatting' >> .git/hooks/pre-commit
	@echo 'cd frontend && npx prettier --check "src/**/*.{ts,tsx,css}" && npm run lint' >> .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "Pre-commit hook installed."

clean:
	cargo clean
	rm -rf frontend/dist
