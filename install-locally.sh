#!/bin/sh

# build with release flag for optimized version
cargo build -r

# put binary on PATH as steam-cli
sudo cp target/release/steam-cli /usr/bin/steam-cli
