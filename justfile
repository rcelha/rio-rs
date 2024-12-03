#!/usr/bin/env just --justfile

# Installs the pre-commit hook as soon as one runs `just`
_ := `ln -sf ${PWD}/pre-commit ${PWD}/.git/hooks/`

# Prints this message
help:
	@just --list

# Prints the TODOs found throughout the repository
todo:
	@ag --rust -Q 'expect("TODO'

# Prints missing docs
todo-docs:
	@ag --rust '/// TODO'


fmt:
	#!/usr/bin/env bash
	set -euxo pipefail
	cargo fix --allow-dirty --allow-staged
	cargo fmt
	cargo c

# Runs tests for the main project
test:
	cargo nextest run --no-fail-fast

# Install development tools
install-tools:
	cargo install cargo-watch
	cargo install flamegraph
	cargo install cargo-nextest
	cargo install cargo-readme
	cargo install simple-http-server

# Generates the README.md file
generate-readme:
	cargo readme -t ../README.tpl.md -r rio-rs > README.md

# Generate docs and run a server for preview
YELLOW:='\033[0;33m'
NC:='\033[0m'
serve-docs:
	cargo doc -p rio-rs -p rio-macros
	@echo -e "you can open your docs at {{YELLOW}}http://$(hostname):8000/rio_rs/{{NC}}"
	simple-http-server --nocache -i ./target/doc

# Run serve-docs, and refresh it when the code changes
serve-docs-watch:
	cargo watch -s 'just serve-docs'
