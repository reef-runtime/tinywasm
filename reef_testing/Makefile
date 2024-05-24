.PHONY: c-test

c-test:
	cd ./c/ && make build
	cp ./c/main.wasm c.wasm
	cargo r -- run c.wasm
