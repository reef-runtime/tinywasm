CLANG:=clang

.PHONY: build

build:
	$(CLANG) -Wl,--allow-undefined -Wl,--no-entry -Wl,--export-all --target=wasm32 --no-standard-libraries -v -o c_test.wasm main.c
