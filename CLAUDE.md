# CLAUDE.md

Guidance for Claude Code (claude.ai/code) working in this repository.

## What this is

A planner for the **German national focus tree** of Hearts of Iron IV. The focus
catalog is parsed out of a saved copy of the wiki page into a local MongoDB, and a
one-screen Flask UI is where a plan is built: click focuses out of the catalog on
the left, drag them into order on the right, and every focus shows the date it
completes. Plans are saved in Mongo under a name.

The catalog only offers what the plan can actually take next: the wiki's
"Requires …" lists are parsed too, so a focus stays out of the list until its
prerequisites are in the plan.

**No icon is ever downloaded and nothing here touches the network.**
`data/foo.html` is the wiki page saved by hand, and a focus keeps a *link* to the
wiki's copy of its icon — the browser is what fetches the images.

## Setup and commands

```bash
pip install -e ".[dev]"              # runtime + test deps, all in pyproject.toml
cp .env.example .env                 # APP_ENV + MONGO_URI

python scripts/import_focuses.py              # data/focuses.html -> JSON -> Mongo
python scripts/import_focuses.py --from-json  # seed from the committed JSON (fast)
python scripts/import_focuses.py --json-only  # re-parse the page, touch no database
python scripts/import_focuses.py --from-json --env prod --mongo-uri mongodb://localhost:27018
                                              # ... into another database entirely
python api/app.py                             # the Flask UI on :5000
python -m pytest                              # tests + coverage (fails under 90%)
```

Needs a local mongod (`brew install mongodb-community`, `brew services start
mongodb-community`). `APP_ENV` (`dev`|`prod`, default `dev`) picks the database:
`hoi4_planner_<env>`. Collections are always `focuses` and `plans`. Dependencies,
pytest and coverage settings live in `pyproject.toml` (no `requirements.txt`). No
linter is configured.

The import script is the one thing that can be pointed at another database for a
single run — `--env` for the suffix, `--mongo-uri` for the server — which is how
the laptop seeds the server's catalog, through an SSH tunnel
(`ssh -N -L 27018:localhost:27017 ced@nuc150`) rather than by exposing its mongod.
Both flags only set what `core.config` and `core.db` read anyway, so nothing below
them learns of a second database, and the target is printed before it is written
to.

## The rules of a plan

A national focus costs **35 days** unless it is one of the few that take half a
term (**14**) or two (**70**), so a plan is an *order* plus the handful of focuses
that differ: each completes when everything before it is done plus its own term,
and the start date itself is not counted as a day of work — from the default
1 Jan 1936, an unmarked first focus lands on 5 Feb 1936.

The wiki page carries no duration to parse, so the odd term is picked by hand on
the focus's row in the plan (a `14 / 35 / 70` picker, the middle button being the
plan's own `days_per_focus` and clearing the mark) and kept in `focus_days`,
`{focus id: days}` holding **only** the focuses that differ. Keyed by id rather
than by position, so dragging cannot move a term onto the wrong focus; a mark for
a focus not in the order is dropped on save (`core.db.save_plan()`), and one that
is not a positive whole number falls back to the default rather than raising
(`core.focuses.days_for()`).

Nothing derived is stored: `focus_ids`, `focus_days`, `start_date` and
`days_per_focus` are the plan, and `core.focuses.schedule()` makes the dates.

The plan being built is kept too, without being named: every edit is written to
**the draft**, one plan document under an id no name can slug to
(`core.db.DRAFT_PLAN_ID`, `~draft`), so closing the tab loses nothing and the
page comes down with it already in hand. It is left out of `load_plans()` — it
belongs to the panel, not to the dropdown — and **Clear** is what throws it away
(`DELETE /api/draft`), leaving the saved plans alone.

## What a focus depends on

The page states a focus's prerequisites in the second cell of its row, in two
forms and no others — there is no "all of the following":

- **`Requires the following:`** — every focus in the list must be taken.
- **`Requires one of the following:`** — any one of them will do, and this form
  can be nested inside the first ("requires A, and one of B or C").

Both are stored as the same shape, `requires`: **a list of groups of focus ids,
each group needing one of its focuses taken**. "The following" is one group per
focus, "one of the following" is a single group of them all, and a nested list is
just another group — which makes checking a plan one test rather than two
(`core.focuses.unmet()`). A row's `Mutually exclusive with:` becomes `excludes`,
made symmetric at parse time because the page does not always write both sides.

Both are parsed with `html.parser` rather than a regex (`_Lists`): the lists nest,
and no regex can tell which `</ul>` closes which. What the catalog cannot name —
an ideology, a country, a focus from a branch this saved page does not carry — is
dropped once the whole page is read, and a group that empties out stops being a
requirement at all rather than stranding its focus forever.

A focus is **available** when every group is met and nothing it excludes is
planned. That is what the catalog filters on (`only available`, on by default;
turn it off and a gated focus is dimmed, locked, and still clickable), and what
`schedule()` re-checks *in order* — a prerequisite has to come **earlier**, so
dragging one below what needs it marks that row `needs`/`clashes` rather than
quietly dating a plan nobody could play.

## Architecture

Three roots, and the imports only ever go one way: `core/` knows nothing of Flask,
`api/` and `scripts/` both import from it, and neither imports from the other.

**`core/`** — everything the script and the web UI share:

- **`core/config.py`** — files and environment variables: `load_env()`,
  `current_env()` / `current_db_name()`, `ROOT`, `DATA_DIR`, `WIKI_PAGE`,
  `FOCUSES_JSON`. Loads `.env` at import.
- **`core/focuses.py`** — pure, and the only interesting logic here:
  `parse_wiki_page()` (the saved page -> focus dicts, dependencies included, via
  `_Lists` / `_dependencies()` / `_resolve()`), what a plan allows (`unmet()`,
  `conflicts()`, `available()`) and the arithmetic of one (`schedule()`,
  `days_for()`, `completion_date()`, `total_days()`, `parse_start()`). No
  database, no Flask.
- **`core/plan_file.py`** — pure too: the shape of an **exported plan file** and
  nothing about where a plan is stored. `to_file()` wraps the plan's own fields in
  an envelope (`format`, `version`) and adds a readable echo of the schedule;
  `from_file()` checks that envelope and hands the values back, leaving what a
  plan may *hold* to the one check a save already goes through.
- **`core/models/`** — one mongoengine document per collection and nothing else:
  `Focus` (keyed by the wiki anchor, and carrying `requires` / `excludes`) and
  `Plan` (keyed by a slug of its name).
  Each mixes in `DictDocument` (`core/models/base.py`), the boundary that keeps
  the ORM inside these modules: `from_dict()` in, `to_dict()` out, every declared
  field present with `None` where nothing was written, anything undeclared
  dropped — which is what lets a parsed wiki row be handed over as it came.
- **`core/db.py`** — **the only module that talks to MongoDB**: `connect_db()`
  (opened on first use, not at import), `replace_focuses()` / `load_focuses()` /
  `focus_count()`, `load_plans()` / `load_plan()` / `save_plan()` /
  `delete_plan()`, the draft's own `save_draft()` / `load_draft()` /
  `clear_draft()` (the same write as a save, against a fixed id), `slugify()` and
  `now()`. `DatabaseError` lives here too: the
  tuple every route catches (pymongo's errors *and* mongoengine's), so an
  unreachable database always reads the same on screen.

**`api/`** — the web UI, and nothing else imports it:

- **`api/app.py`** — configuration only: `secret_key()`, `mounted_under()` and
  `create_app()`. `app = create_app()` at the bottom is what gunicorn serves
  (`app:app`) and what the tests drive. No route lives here.
- **`api/views/`** — one module per page, one blueprint each, listed in
  `views.BLUEPRINTS`: `main` (`/`, `/health`) and `plans` (the JSON at
  `/api/plans` for a named plan, at `/api/draft` for the one the planner writes
  after every edit, and at `/api/plans/export` + `/api/plans/import` for the one
  that travels as a file — the same fields every time, checked by the same
  `plan_fields()`). Endpoints are therefore `<blueprint>.<function>` —
  `main.index`, `plans.save`, `plans.draft`, `plans.export_plan`.
- **`api/queries.py`** — the wording of a database failure, in one place.
- **`api/templates/`** + **`api/static/`** — `planner.html` renders all 215 rows
  server-side, each carrying its `data-requires` / `data-excludes`, and hands
  the draft over in `data-draft`; `app.js` filters them to what the plan allows,
  reorders the plan by drag, sets a focus's term, writes the draft back after
  every edit (on a short timer, and silent unless it fails), saves, and hands a
  plan out to a file or takes one back from it.

**`scripts/import_focuses.py`** — the seeder. The JSON in the middle
(`data/focuses.json`, committed) is deliberate: parsing 2 MB of wiki markup is the
slow and fragile half, so `--from-json` seeds a database in a moment and a
checkout with no saved page can still fill one.

### The one deliberate duplication

Completion dates — and the term of each focus that feeds them — are computed
**both** in `core/focuses.py` and in `api/static/app.js`. Dragging has to re-date
the whole list on every pointer move, and a round trip per frame would make the
drag feel dead. The server's copy stays
the authority for anything it answers itself — `/health`, and the dates written
into an exported file. Change the rule in one and change it in the other.

## A plan as a file

Export and import move one plan between databases — a laptop's `dev` and the
server's `prod`, or two people — as a single JSON file. The format is
`core/plan_file.py`'s and the browser writes none of it: **Export** posts the
panel and saves what comes back, **Import** posts what it read off disk and draws
what comes back.

```
POST /api/plans/export        the panel, as a download
GET  /api/plans/<id>/export   a saved plan, same file, for curl
POST /api/plans/import        a file, checked, handed back as a plan to draw
```

The file holds the plan and nothing else — `name`, `focus_ids`, `start_date`,
`days_per_focus`, `focus_days` — inside an envelope naming what it is, so a JSON
file that is not one of ours is refused with a sentence instead of half-read into
the panel. Beside them is `focuses`, a **readable echo**: each focus's name, term
and completion date, for whoever opens the file. An import ignores it, because the
order is the plan and `schedule()` makes those dates again. Nothing of the database
travels: no id, no stamps, no timestamp of the export itself — exporting one plan
twice gives the same bytes, which is what lets a plan be diffed or kept in git.

**Importing stores nothing.** A plan arriving is a plan being built, so it lands
in the panel — where it is drawn, reordered, renamed and then saved like any
other, and where the draft picks it up on the way. Ids this catalog does not have
come back in `unknown` and stay in the plan rather than being dropped quietly;
the panel says how many, and `schedule()` already leaves them out of the dates.

## Deploying

One command from the laptop, no registry: `make deploy` builds the image, `docker
save`s it, ships the tarball over SSH, loads it there and restarts the container,
then waits for `/health` — and `make rollback` puts the previous version back
without building or transferring anything (it is already loaded on the server).

```bash
make setup     # once: directory layout, docker check, nginx snippet
make deploy    # build -> ship -> restart -> health check -> prune
make seed      # fill the `focuses` collection from the image's JSON
make rollback  # back to the previous version
make status    # what runs there, and what /health says
make logs      # follow the container's output
```

`make seed` runs inside the deployed container, where the image's
`data/focuses.json` already is. The other direction — `import_focuses.py --env prod
--mongo-uri …` from the laptop, over a tunnel — seeds the same collection without a
deploy, and is what a re-parsed page goes out through when the image is not being
rebuilt.

`Makefile` is shortcuts only; the settings are in **`deploy/config.sh`**, every
one of them overridable from the environment (`SSH_HOST=ced@other-box make
deploy`). **`deploy/remote.sh`** is the server side and is uploaded before every
run, which is what keeps deploy and rollback launching the container the same way
— the `docker run` line exists once.

What the server owns and no deploy may write: **`$REMOTE_DIR/config/.env`** (the
secret key, `APP_ENV=prod`, `URL_PREFIX`, and a `MONGO_URI` pointing at the host's
mongod as `host.docker.internal`), mounted read-only into the container. What the
image carries: code, and `data/focuses.json` — the saved wiki page stays on the
laptop, so `make seed` is `--from-json` only. The catalog is not in the database
until it is seeded; a deploy onto an empty one comes up healthy and says so.

The container **publishes no port**. It joins the dockerized nginx's shared
network and answers at `http://$CONTAINER_NAME:$APP_PORT`, which the snippet
rendered from `deploy/nginx/*.template` proxies `/hoi4/` to — installed by
`make setup` / `make nginx`, never by a deploy, and validated with `nginx -t`
before the proxy restarts. The prefix lives in two places that must agree:
`URL_PREFIX` in `deploy/config.sh` (what nginx strips) and `URL_PREFIX` in the
server's `.env` (what `mounted_under()` builds links with).

## Tests

`pyproject.toml` sets `pythonpath = [".", "scripts", "api"]`: `core.*` imports
from the root, `import_focuses` from `scripts/`, and `app` / `queries` / `views.*`
from `api/` (only `core` is a package). `addopts` always runs coverage with
`fail_under = 90`.

**One test file per source module**, named after it: `test_core_<module>.py` for
`core/`, `test_api_<module>.py` for `api/`, `test_<script>.py` for `scripts/`.
Flat, with the `core_`/`api_` prefix, so two directories of tests cannot collide
on a name. `test_core_connect.py` is split out from `test_core_db.py` because it
is the one place `connect_db()` is *not* stubbed.

No test touches the network or a real MongoDB. The `fake_db` fixture binds the
models to an in-memory **mongomock** server and stubs out `core.db.connect_db()` —
the one function that would reach for a real one — and `clean_env` clears
`APP_ENV`/`MONGO_URI`, which `.env` would otherwise set at import. Since every
read and write goes through `core.db`, and every one of its functions calls
`connect_db()` first, replacing that one function is what fakes the database
everywhere (and what `mongo_is_down` fails on purpose).

`fake_db["focuses"].docs` is how a test seeds and reads a collection: assigning
replaces its contents, reading gives them back as the app sees them, with Mongo's
`_id` shown as `id`. It is a view, not the storage.

The autouse `clock` fixture makes `core.db.now()` tick a second per call: two
saves in one test otherwise land in the same millisecond, which mongoengine
rounds to a tie and leaves "most recently touched first" undefined. Stamps are
**naive UTC** throughout, because that is what Mongo hands back — see `now()`.

`data/foo.html` is never read by a test; the parser tests build the few rows of
markup they need (`tests/helpers.py`) — `wiki_row()` for the focus itself, and
`requirements()` wrapping `requires()` / `excludes()` for the cell that says what
it depends on. `requires()` nests, which is how the nested-list case is tested.
