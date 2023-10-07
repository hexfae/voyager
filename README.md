# Voyager

Voyager is the server back-end for [Endless Void](https://github.com/Skirlez/void-stranger-endless-void), a level editor for [Void Stranger](https://store.steampowered.com/app/2121980/Void_Stranger/), a "2D sokoban-style puzzle game where every step counts."

Currently, it handles POST requests to upload levels, and receives GET requests to send a list of levels. In the future, it will handle PUT requests to edit levels using a level-specific key.

You're unlikely to have a use for it, I'm open-sourcing it on principle and for practice.

## Building

Run `cargo build` to build in debug mode or `cargo build --release` to build in release mode. The compiled binary will be available in `target/debug/` or `target/release` respectively. See also the [Cargo build documentation](https://doc.rust-lang.org/cargo/commands/cargo-build.html).

## Running

### Prerequisites

- An open port on `3000`, which it will bind to.

### Usage

This might change in the future! Currently:

```bash
$ curl --data '{"name": "john java world", "data": "asdasd", "author": "john java", "author_brand": 1, "inputs": "wasd", "burdens": 2}' --header "Content-Type: application/json" localhost:3000/void_stranger
```
```json
{"key":"01HC4SDY2PQ956WBXJHERSD3NN"}
```

```bash
$ curl localhost:3000/void_stranger
```  
```json
[{"name":"john java world","data":"asdasd","author":"john java","author_brand":1,"burdens":2,"upload_date":"2023-10-07T10:02:50.838216987Z"},{"name":"john java world","data":"asdasd","author":"john java","author_brand":1,"burdens":2,"upload_date":"2023-10-07T10:13:28.587682489Z"}]
```

## To-do list

- [x] Use an [embedded SurrealDB database](https://surrealdb.com/docs/embedding/rust) instead for ease of starting, most likely [Speedb](https://www.speedb.io/).
- [x] Finalize data structures. (mostly finished)
- [ ] PUT router for editing levels.
- [ ] Use `serde::Value` for storing level data (JSON).
- [ ] Log to a file (DEBUG) as well as to stdout (INFO).
- [ ] A level rating system (like/dislike, star rating, play count, completion count?).
- [ ] Implement caching using one of [redis](https://lib.rs/crates/redis), [cached](https://lib.rs/crates/cached), etc.
- [ ] General code clean-up, especially in the `server/routers/` directory.
- [ ] Use [tower_sessions](https://lib.rs/crates/tower-sessions) for session management & [deadpool](https://lib.rs/crates/deadpool) for an async connection pool.
- [ ] Write unit tests.
- [ ] ~~Use `serde::Value` instead of `Level`, `CreateLevel`, `PublicLevel`, `Key`, & maybe also `Record`.~~
