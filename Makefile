.PHONY: wasm build dev clean check docker-build docker-run seed-db fmt lint test check-all sqlx-prepare

wasm:
	wasm-pack build --target web --out-dir ../../static/wasm --out-name karta --no-typescript crates/frontend/

wasm-dev:
	wasm-pack build --target web --out-dir ../../static/wasm --out-name karta --dev --no-typescript crates/frontend/

build: wasm
	cargo build -p backend --bin backend

check:
	cargo check -p backend --bin backend

dev: wasm
	cargo watch -p backend --bin backend

seed-db:
	cargo run -p backend --bin seed

clean:
	cargo clean
	rm -rf static/wasm

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace -- -D warnings

test:
	cargo test --workspace

check-all: fmt lint test
	@echo "✅ All checks passed!"

sqlx-prepare:
	DATABASE_URL=sqlite:./data/rom.db cargo sqlx prepare --workspace

docker-build:
	docker build -t karta-yaxu .

docker-run:
	docker run \
		-p 3000:3000 \
		-v karta-yaxu-data:/app/data \
		-v karta-yaxu-static:/app/static \
		-e DATABASE_URL=sqlite:/app/data/rom.db \
		-e SEED_DATA_PATH=/app/template \
		-e RUST_LOG=info \
		-e PORT=3000 \
		--name karta-yaxu \
		karta-yaxu