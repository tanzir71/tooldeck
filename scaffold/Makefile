# Tooldeck — dev + verification Makefile
# `make verify` is the single gate the build loop (handoff §11.2) runs each milestone.
# It MUST exit non-zero on any failure.

SHELL := /bin/bash
.DEFAULT_GOAL := help

# ---- Paths ----
TAURI_DIR := src-tauri
FRONT_DIR := .

.PHONY: help
help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
	  awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-16s\033[0m %s\n",$$1,$$2}'

# ---- Setup ----
.PHONY: install
install: ## Install JS + Rust toolchain deps
	npm ci
	rustup component add clippy rustfmt
	@echo "Ensure sidecars exist in $(TAURI_DIR)/binaries (ffmpeg, ffprobe, tesseract). See make sidecars."

.PHONY: sidecars
sidecars: ## Print sidecar setup reminder (§9.4)
	@echo "Place per-triple binaries in $(TAURI_DIR)/binaries/ e.g.:"
	@echo "  ffmpeg-aarch64-apple-darwin  ffprobe-aarch64-apple-darwin  tesseract-aarch64-apple-darwin"
	@echo "chmod +x them. For dist: sign each sidecar with hardened runtime BEFORE notarizing."

# ---- Dev ----
.PHONY: dev
dev: ## Run the app in dev mode
	npm run tauri dev

# ---- Lint / format / typecheck ----
.PHONY: fmt fmt-check clippy ts-check eslint
fmt: ## Auto-format Rust + JS
	cd $(TAURI_DIR) && cargo fmt
	npx prettier --write "src/**/*.{ts,tsx,css}"

fmt-check: ## Check formatting (CI-safe)
	cd $(TAURI_DIR) && cargo fmt --check
	npx prettier --check "src/**/*.{ts,tsx,css}"

clippy: ## Rust lints, warnings are errors
	cd $(TAURI_DIR) && cargo clippy --all-targets -- -D warnings

ts-check: ## TypeScript typecheck (no emit)
	npx tsc --noEmit

eslint: ## Lint the frontend
	npx eslint "src/**/*.{ts,tsx}"

# ---- Tests ----
.PHONY: test-rust test-e2e
test-rust: ## Rust unit + integration tests (uses tests/fixtures)
	cd $(TAURI_DIR) && cargo test --all

test-e2e: ## WebDriver smoke flow (tauri-driver). Skips if driver missing.
	@if command -v tauri-driver >/dev/null 2>&1; then \
	  npm run test:e2e; \
	else \
	  echo "⚠️  tauri-driver not found — install to enable e2e smoke tests"; exit 0; \
	fi

# ---- The gate ----
.PHONY: verify
verify: fmt-check clippy ts-check eslint test-rust test-e2e ## THE LOOP GATE — all checks; exits non-zero on any failure
	@echo "✅ verify passed"

# ---- Package ----
.PHONY: build notarize
build: ## Build a DMG (unsigned unless env creds present)
	npm run tauri build

notarize: ## Sign + notarize + staple (needs Apple Developer creds in env). See scripts/notarize.sh
	./scripts/notarize.sh
