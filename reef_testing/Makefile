all_test: c_test rust_test rust_test_debug


.PHONY: c_test

c_test:
	cd ./c_test/ && make build
	cp ./c_test/c_test.wasm ./c_test.wasm
	wasm2wat -o c_test.wat c_test.wasm
	cargo r -- run c_test.wasm


.PHONY: rust_test
.PHONY: rust_test_debug

rust_test:
	cargo build --package rust_test --target=wasm32-unknown-unknown --release
	cp ../target/wasm32-unknown-unknown/release/rust_test.wasm ./rust_test.wasm
	wasm-opt -o rust_test.wasm -O rust_test.wasm
	wasm2wat -o rust_test.wat rust_test.wasm
	cargo r -- run rust_test.wasm

rust_test_debug:
	cargo build --package rust_test --target=wasm32-unknown-unknown
	cp ../target/wasm32-unknown-unknown/debug/rust_test.wasm ./rust_test_debug.wasm
	wasm2wat -o rust_test_debug.wat rust_test_debug.wasm
	cargo r -- run rust_test_debug.wasm
