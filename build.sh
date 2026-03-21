#!/bin/bash

#cargo install cross --git https://github.com/cross-rs/cross

# linux
#RUSTFLAGS="-Zlocation-detail=none" cargo build --release
RUSTFLAGS="-Zlocation-detail=none" cross build --target=x86_64-unknown-linux-musl --release
# windows
#RUSTFLAGS="-Zlocation-detail=none" cross build --target x86_64-pc-windows-gnu --release --verbose
#cargo xwin build --release --target x86_64-pc-windows-msvc

#upx --best --lzma target/release/archbot
upx --best --lzma target/x86_64-unknown-linux-musl/release/archbot
#strip target/x86_64-pc-windows-gnu/release/archbot.exe
#upx --best --lzma target/x86_64-pc-windows-gnu/release/archbot.exe
#strip target/x86_64-pc-windows-msvc/release/archbot.exe
#upx --best --lzma target/x86_64-pc-windows-msvc/release/archbot.exe
