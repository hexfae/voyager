# Voyager

Voyager is the server back-end for [Endless Void](https://github.com/Skirlez/void-stranger-endless-void), a level editor for [Void Stranger](https://store.steampowered.com/app/2121980/Void_Stranger/), a "2D sokoban-style puzzle game where every step counts."

## Building

`cargo build [--release]`

## Running

Voyager uses port 3000. Voyager also creates the following directories and files:

- `voyager/`
- `voyager/logs/`
- `voyager/backups/`
- `voyager/webui.db`
- `voyager/levels.db`

Logs are saved on a per-day basis as `voyager/logs/voyager.log.yyyy-mm-dd`.

Backups are made every 24-hours as `voyager/backups/yyyy-mm-dd.db`


## Usage

Voyager is a server/database for [Endless Void](https://github.com/Skirlez/void-stranger-endless-void). As such, little else is needed than to simply run it, and for users to send requests to it. Users can change which server to connect to in-game (although the official one is obviously recommended).

A Web UI is available at `/voyager/webui`. The Web UI may be used for administrative tasks, such as deleting naughty levels. Actual level uploading, editing, and browsing is done by clients through [Endless Void](https://github.com/Skirlez/void-stranger-endless-void).

## To-do list

- [ ] Level packs.
- [ ] Comprehensive logging.
- [ ] Testing?
