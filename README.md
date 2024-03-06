# Voyager

Voyager is the server back-end for [Endless Void](https://github.com/Skirlez/void-stranger-endless-void), a level editor for [Void Stranger](https://store.steampowered.com/app/2121980/Void_Stranger/), a "2D sokoban-style puzzle game where every step counts."

## Building

Same as (mostly) any other Rust project, `cargo build [--release]`.

## Running

Voyager attempts to bind to port 3000. Voyager also looks for or creates a `voyager.db` file in the current directory.

## Usage

Voyager is a server/database for [Endless Void](https://github.com/Skirlez/void-stranger-endless-void). As such, little else is needed than to simply run it, and for users to send requests to it. Users can change which server to connect to in-game (although the official one is obviously recommended).

A Web UI is available at `/voyager/webui`. The Web UI may be used for administrative tasks, such as deleting naughty levels. Actual level uploading, editing, and browsing is done by clients through [Endless Void](https://github.com/Skirlez/void-stranger-endless-void).

## To-do list

- [ ] Level packs.
- [ ] Web UI (for administration).
- [ ] Comprehensive logging.
- [ ] Testing?
