
.PHONY: .tools
.tools:
	cargo install flamegraph

.PHONY: loadserver
loadserver:
	cargo build --release
	valgrind --leak-check=full ../../target/release/metric_aggregator_server 0.0.0.0:5000

.PHONY: flamegraph-server
flamegraph-server:
	CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph --bin metric_aggregator_server --  0.0.0.0:5000

.PHONY: loadall
loadall:
	cargo run --bin metric_aggregator_loadall

.PHONY: dropall
dropall:
	cargo run --bin metric_aggregator_dropall
