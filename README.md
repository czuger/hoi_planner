# HOI4 planner

A planner for the German national focus tree of Hearts of Iron IV: pick focuses out of the catalog, drag them into order, and every focus shows the date it completes. A second page attaches technologies to those focuses, in two aligned columns. See `CLAUDE.md` for how it works.

## Run

```sh
cargo run -- import-focuses --from-json       # once, to fill the focus catalog
cargo run -- import-technologies --from-json  # once, to fill the technology catalog
cargo run
```

Then open <http://127.0.0.1:5000>.

Settings (environment variables):

| Variable | Default | Meaning |
|---|---|---|
| `DATABASE_URL` | `sqlite://data/hoi4_planner.db` | SQLite database, created with its folder if missing |
| `BIND_ADDR` | `127.0.0.1:5000` | TCP address the server listens on |
| `SOCKET_PATH` | | Unix socket to listen on instead of TCP |
| `ASSET_ROOT` | `.` | Folder holding `static/` and `data/` |
| `URL_PREFIX` | | Sub-path a reverse proxy serves the app under |
| `RUST_LOG` | `info` | Log filter |

## Deploy

`make setup` once, then `make deploy` and `make seed`. See `deploy/config.sh` for the settings, and `make help`.
