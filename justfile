#!/usr/bin/env just --justfile

# Installs the pre-commit hook as soon as one runs `just`
_ := `ln -sf ${PWD}/pre-commit ${PWD}/.git/hooks/`

# Prints this message
help:
    @just --list

# Prints the TODOs found throughout the repository
todo:
	@ag --rust -Q 'expect("TODO'

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
fmt-examples: (_fmt "./examples/black-jack") (_fmt "./examples/ping-pong") (_fmt "./examples/metric-aggregator")

# Formats all the projects and examples
fmt-all: fmt fmt-macros fmt-examples

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
