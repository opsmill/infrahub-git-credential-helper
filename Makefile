.PHONY: build release check fmt lint test clean docker

build:
	cargo build

release:
	cargo build --release

check:
	cargo check

fmt:
	cargo fmt

lint:
	cargo fmt --check
	cargo clippy -- -D warnings

test:
	cargo test -- --test-threads=1

clean:
	cargo clean

docker:
	docker build -t infrahub-git-credential .
