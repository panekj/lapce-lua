release:
  cargo build --target wasm32-wasi --release
  cp ./target/wasm32-wasi/release/lapce-lua.wasm .

dev:
  cargo build --target wasm32-wasi
  cp ./target/wasm32-wasi/release/lapce-lua.wasm .
