# HOI4 planner

A planner for the German national focus tree of Hearts of Iron IV: pick focuses out of the catalog, drag them into order, and every focus shows the date it completes. A second page attaches technologies to those focuses, in two aligned columns. See `CLAUDE.md` for how it works.

## Run

```sh
cargo run -- seed      # once, to fill the three catalogs (focuses, technologies, staff)
cargo run
```

Then open <http://127.0.0.1:5000>.

Settings (environment variables):

| Variable | Default | Meaning |
|---|---|---|
| `HOI4_PLANNER_DATA_DIR` | `data` | Folder of the SQLite database (`hoi4_planner.db`), created if missing |
| `BIND_ADDR` | `127.0.0.1:5000` | TCP address the server listens on |
| `HOI4_PLANNER_SOCKET` | | Unix socket to listen on instead of TCP |
| `RUST_LOG` | `info` | Log filter |
| `NO_COLOR` | | Plain logs, even on a terminal |

The stylesheet, the scripts and the catalogs' JSON are compiled into the binary, which is all a deployed copy needs. Every link is relative, so the app can be served under any sub-path.

## Deploy

With `cargo deploy`, configured in `[package.metadata.deploy]` of `Cargo.toml`; the server's own settings come from the `DEPLOY_*` environment variables, never from this repository.

```sh
cargo deploy setup          # once: prepare the server
cargo deploy                # build, ship, restart, health check
cargo deploy run -- seed    # fill the catalogs from the deployed binary
cargo deploy rollback       # back to the previous version
cargo deploy status         # what runs, and what /health says
cargo deploy logs           # follow the service's journal
```
