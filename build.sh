#!/bin/bash

#cargo install cross --git https://github.com/cross-rs/cross

# linux
#RUSTFLAGS="-Zlocation-detail=none" cargo build --release
RUSTFLAGS="-Zlocation-detail=none" cross build --target=x86_64-unknown-linux-musl --release
# windows
#RUSTFLAGS="-Zlocation-detail=none" cross build --target x86_64-pc-windows-gnu --release --verbose
#upx --best --lzma target/release/archbot
upx --best --lzma target/x86_64-unknown-linux-musl/release/archbot
#strip target/x86_64-pc-windows-gnu/release/webify.exe
#upx --best --lzma target/x86_64-pc-windows-gnu/release/archbot.exe
