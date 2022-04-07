.PHONY: .tools
.tools:
	cargo install flamegraph

.PHONY: loadserver
loadserver:
	cargo build --release
	valgrind --leak-check=full ./target/release/server 0.0.0.0:5000

.PHONY: loadall
loadall:
	cargo run --bin loadall

.PHONY: dropall
dropall:
	cargo run --bin dropall
