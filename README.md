The `steam-cli` is a tool that I built
1. to interact with the steam-api and answer questions such as, I'm playing games with a group of friends. What games do we all own?
2. to learn Rust, so I if I'm not following best practices, feel free to point them out or submit a PR

# Usage

The `steam-cli` primarily runs as a cli in the terminal, but it can also run as a discord bot.
The instructions below are for running it in the terminal.

## Environment Variables

### Steam API Key

For both development and you'll need a `STEAM_API_KEY`.
You can generate one using your steam account on [the steamstore](https://steamcommunity.com/dev/apikey).

### `USER_STEAM_ID`

If you want to use the `--by-name`/`-b` flag, then you'll need to set your `USER_STEAM_ID`.
This is used in order to grab your friends list to resolve persona names into steam ids.

You can also get a lot of the same functionality by just setting environment variables for your friends, but this way you only need to set up one envionment variable instead of several.

## Rust

Rust is required for development and installation. To install rust, use [rustup](https://rustup.rs/).

At some point, I will include a binary that be run directly, but I don't want to do the work to validate cross platform validation.

## Running it

If you want to run it without adding the binary to your path you can use `cargo run <args>`.

If you want to run it locally, you should use `./install-locally.sh`.

> I believe this will work on unix systems like MacOS and Linux. I'm not sure if this will work on Windows, but you should still be able to run it using `cargo run`.
