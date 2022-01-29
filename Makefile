.PHONY: help
help:
	@echo make help|fmt|todo

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
	cd ./examples/metric-aggregator && cargo fmt
	cd ./examples/metric-aggregator && cargo fix --allow-dirty --allow-staged
	cd ./examples/metric-aggregator && cargo c

.PHONY: fmt-macros
fmt-macros:
	cd ./rio-macros && cargo fmt
	cd ./rio-macros && cargo fix --allow-dirty --allow-staged
	cd ./rio-macros && cargo c

.PHONY: docker-images
docker-images:
	DOCKER_BUILDKIT=1 docker build -t rio-rs:latest .