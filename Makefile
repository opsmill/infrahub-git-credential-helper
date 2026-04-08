.PHONY: build release check fmt lint test clean docker update-schema

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
	docker build -t registry.opsmill.io/opsmill/infrahub-git-credential-helper .

update-schema:
	curl -sL https://raw.githubusercontent.com/opsmill/infrahub/stable/schema/schema.graphql -o schema/schema.graphql
