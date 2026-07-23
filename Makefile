SHELL := /bin/bash
.DEFAULT_GOAL := help

.PHONY: help
help: ## Show available commands
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-16s\033[0m %s\n",$$1,$$2}'

.PHONY: install
install: ## Install JavaScript dependencies
	npm ci

.PHONY: dev
dev: ## Run Tooldeck in Tauri development mode
	npm run tauri dev

.PHONY: fmt
fmt: ## Format Rust and frontend source
	cd src-tauri && cargo fmt
	npm run format

.PHONY: fmt-check
fmt-check: ## Check Rust and frontend formatting
	cd src-tauri && cargo fmt --check
	npm run format:check

.PHONY: clippy
clippy: ## Run Rust lints with warnings denied
	cd src-tauri && cargo clippy --all-targets -- -D warnings

.PHONY: ts-check
ts-check: ## Typecheck the frontend
	npm run typecheck

.PHONY: eslint
eslint: ## Lint the frontend
	npm run lint

.PHONY: test-rust
test-rust: ## Run Rust tests
	cd src-tauri && cargo test --all

.PHONY: test-web
test-web: ## Run frontend unit and smoke tests
	npm test

.PHONY: test-e2e
test-e2e: ## Run the headless registry-to-workspace smoke flow
	npm run test:e2e

.PHONY: release-check
release-check: ## Validate release config, scripts, and credential blocker reporting
	./scripts/test-release-config.sh

.PHONY: verify
verify: fmt-check clippy ts-check eslint test-rust test-web test-e2e release-check ## Run the complete headless gate
	npm run build
	@echo "verify passed"

.PHONY: sidecars
sidecars: ## Validate and stage reviewed Apple Silicon sidecars
	./scripts/prepare-sidecars.sh "$(SIDECAR_SOURCE)"

.PHONY: release-audit
release-audit: ## Report signing, notarization, sidecar, and license readiness
	./scripts/release-audit.sh

.PHONY: package-dry-run
package-dry-run: ## Build an ad-hoc DMG rehearsal after sidecars are staged
	./scripts/release.sh --dry-run

.PHONY: build
build: ## Build, sign, notarize, staple, and verify the release DMG
	./scripts/release.sh

.PHONY: notarize
notarize: ## Sign, notarize, and staple a built DMG
	./scripts/notarize.sh "$(DMG)"
