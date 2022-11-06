$(shell ln -sf ${PWD}/pre-commit ${PWD}/.git/hooks/)

.PHONY: help
help:
	@echo "make help|fmt|todo"

.PHONY: todo
todo:
	@ag --rust -Q 'expect("TODO'

.PHONY: fmt
fmt: fmt-src fmt-examples fmt-macros

.PHONY: fmt-src
fmt-src:
	cargo fmt
	cargo fix --allow-dirty --allow-staged
	cargo c

.PHONY: fmt-examples
fmt-examples:
	@echo "Formatting metric-aggregator"
	cd ./examples/metric-aggregator && cargo fmt
	cd ./examples/metric-aggregator && cargo fix --allow-dirty --allow-staged
	cd ./examples/metric-aggregator && cargo c
	@echo "Formatting black-jack"
	cd ./examples/black-jack && cargo fmt
	cd ./examples/black-jack && cargo fix --allow-dirty --allow-staged
	cd ./examples/black-jack && cargo c
	@echo "Formatting ping-pong"
	cd ./examples/ping-pong && cargo fmt
	cd ./examples/ping-pong && cargo fix --allow-dirty --allow-staged
	cd ./examples/ping-pong && cargo c

.PHONY: fmt-macros
fmt-macros:
	cd ./rio-macros && cargo fmt
	cd ./rio-macros && cargo fix --allow-dirty --allow-staged
	cd ./rio-macros && cargo c

.PHONY: docker-images
docker-images:
	DOCKER_BUILDKIT=1 docker build -t rio-rs:latest .

# Tooling
.PHONY: tools
tools: cargo-watch cargo-flamegraph cargo-nextest

.PHONY: cargo-watch
cargo-watch: $(HOME)/.cargo/bin/cargo-watch

$(HOME)/.cargo/bin/cargo-watch:
	cargo install cargo-watch

.PHONY: cargo-flamegraph
cargo-flamegraph: $(HOME)/.cargo/bin/cargo-flamegraph

$(HOME)/.cargo/bin/cargo-flamegraph:
	cargo install flamegraph

.PHONY: cargo-nextest
cargo-nextest: $(HOME)/.cargo/bin/cargo-nextest

$(HOME)/.cargo/bin/cargo-nextest:
	cargo install cargo-nextest
