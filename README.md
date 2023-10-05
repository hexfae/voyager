# Voyager

Voyager is the server back-end for [Endless Void](https://github.com/Skirlez/void-stranger-endless-void), a level editor for [Void Stranger](https://store.steampowered.com/app/2121980/Void_Stranger/), a "2D sokoban-style puzzle game where every step counts."

Currently, it handles POST requests to upload levels, and receives GET requests to send a list of levels. In the future, it will handle PUT requests to edit levels using a level-specific key.

You're unlikely to have a use for it, I'm open-sourcing it on principle and for practice.

## Building

Run `cargo build` to build in debug mode or `cargo build --release` to build in release mode. The compiled binary will be available in `target/debug/` or `target/release` respectively. See also the [Cargo build documentation](https://doc.rust-lang.org/cargo/commands/cargo-build.html).

## Running

### Prerequisites

- A [SurrealDB](https://surrealdb.com/) database running on `localhost:8000` with a `root` user & a `root` password (see ["Getting started"](https://surrealdb.com/docs/introduction/start)).
- An open port on `3000`, which it will bind to.

### Usage

TBD. This **will** change in the future! Currently:

```bash
$ curl --data '{"level": {"name": "john java world", "data": "asdasd", "author": "john java"}, "inputs": "wasd"}' --header "Content-Type: application/json" localhost:3000/void_stranger
"01HBYGJ3WM8D5347JYWKJN2C85" # the level's key, for editing
```

```bash
$ curl localhost:3000/void_stranger
[{"name":"john java world","data":"asdasd","author":"john java"}]
```

## To-do list

- Use [tower_sessions](https://lib.rs/crates/tower-sessions) for session management & [deadpool](https://lib.rs/crates/deadpool) for an async connection pool.
- Use an [embedded SurrealDB database](https://surrealdb.com/docs/embedding/rust) instead for ease of starting, most likely [Speedb](https://www.speedb.io/).
- Use `serde::Value` instead of `Level`, `CreateLevel`, `PublicLevel`, `Key`, & maybe also `Record`.
- Definitely use `serde::Value` for storing level data (JSON).
- Log to a file (DEBUG) as well as to stdout (INFO).
- PUT router for editing levels.
- A level rating system (like/dislike, star rating, play count, completion count?).
- General code clean-up, especially in the `server/routers/` directory.
