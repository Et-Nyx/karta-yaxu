.PHONY: wasm build dev clean check

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

clean:
	cargo clean
	rm -rf static/wasm