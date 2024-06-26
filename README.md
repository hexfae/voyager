# Voyager

Voyager is the server back-end for [Endless Void](https://github.com/Skirlez/void-stranger-endless-void), a level builder for [Void Stranger](https://store.steampowered.com/app/2121980/Void_Stranger/), a "2D sokoban-style puzzle game where every step counts." Levels from [Endless Void](https://github.com/Skirlez/void-stranger-endless-void) may be uploaded to and downloaded from a Voyager instance.

## Features

Besides the standard features you'd expect from a server/database (CRUD), Voyager boasts:

- A sophisticated level parser/validator
- Extensive logging
- Comprehensive documentation (100% coverage)
- Tests (soon)
- Optional Discord webhook for notifying of level uploads
- A web UI
- 🔥🚀🦀

## Building

`cargo build [--release]`

## Running

Voyager binds to port 3000. Voyager also creates the following directories and files:

- `voyager/`
- `voyager/logs/`
- `voyager/backups/`
- `voyager/webui.db`
- `voyager/levels.db`

Logs are saved on a per-day basis as `voyager/logs/voyager.log.yyyy-mm-dd`.

Backups are made every 24 hours as `voyager/backups/yyyy-mm-dd.db`.

## Usage

Voyager is the server back-end for [Endless Void](https://github.com/Skirlez/void-stranger-endless-void). Once you have created the Web UI user (you will be prompted on first launch), Voyager will run on port 3000. Clients may then specify to use your instance of Voyager in-game (an official instance is available and selected by default, of course).

A Web UI is available at `/voyager/webui`. The Web UI may be used for administrative tasks, such as deleting naughty levels.

## Contributing

Please contribute

## To-do

- [ ] Level packs
- [ ] Tests
