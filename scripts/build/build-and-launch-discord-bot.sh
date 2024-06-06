#!/bin/sh

# TODO: I couldn't get this `source .env` bit to work, so I'm just commenting it out.
# Some env vars are expected to be in the environment though before running this.
# source .env
cargo run -r --bin discord-steam-cli
