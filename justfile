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


_fmt WORKDIR:
	#!/usr/bin/env bash
	set -euxo pipefail
	cd {{WORKDIR}}
	cargo fmt
	cargo fix --allow-dirty --allow-staged
	cargo c

# Formats main project
fmt:
	@just _fmt .

# Formats the macros project
fmt-macros: (_fmt "./rio-macros")

# Formats all the examples
fmt-examples: (_fmt "./examples/black-jack") (_fmt "./examples/ping-pong") (_fmt "./examples/metric-aggregator") (_fmt "./examples/presence")

# Formats all the projects and examples
fmt-all: fmt fmt-macros fmt-examples generate-readme

# Runs tests for the main project
test:
	cargo nextest run

# Runs tests for the macros project
test-macros:
	cargo nextest run  --manifest-path ./rio-macros/Cargo.toml

# Runs tests for all the example projects
test-examples:
	cargo nextest run  --manifest-path ./examples/black-jack/Cargo.toml
	cargo nextest run  --manifest-path ./examples/ping-pong/Cargo.toml
	cargo nextest run  --manifest-path ./examples/metric-aggregator/Cargo.toml

# Tests all the projects and examples
test-all: test test-macros test-examples

# Install development tools
install-tools:
	cargo install cargo-watch
	cargo install flamegraph
	cargo install cargo-nextest
	cargo install cargo-readme
	cargo install simple-http-server

# Generates the README.md file
generate-readme:
	cargo readme -t README.tpl.md > README.md

# Generate docs and run a server for preview
YELLOW:='\033[0;33m'
NC:='\033[0m'
serve-docs:
	cargo doc
	@echo -e "you can open your docs at {{YELLOW}}http://$(hostname):8000/rio_rs/{{NC}}"
	simple-http-server --nocache -i ./target/doc

# Run serve-docs, and refresh it when the code changes
serve-docs-watch:
	cargo watch -s 'just serve-docs'
