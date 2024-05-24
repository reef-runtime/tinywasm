CLANG:=clang

.PHONY: build

build:
	$(CLANG) -Wl,--allow-undefined -Wl,--no-entry -Wl,--export-all --target=wasm32 --no-standard-libraries -v -o main.wasm main.c
