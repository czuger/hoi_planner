# CLAUDE.md

Guidance for Claude Code (claude.ai/code) working in this repository.

## What this is

A planner for the **German national focus tree** of Hearts of Iron IV, and the research to go with it. Both catalogs are parsed out of saved copies of wiki pages into a local SQLite database. The web UI is one screen (`/`). On the left, the catalog in three tabs: **Focus**, **Technologies**, **État-major**, each with its own search and filters. On the right, the plan: one line per focus, in four aligned columns. The first is the focus itself, dragged into order and showing the date it completes. The second holds the technologies attached to it, each taking the date of its focus. The third holds the staff and designers of the *German Reich* page hired with it, each taking its date and filling the post of its table (Heinz Guderian → Theorists, Krupp → Industrial concerns). The fourth holds free comments, written on the line itself. Clicking a focus in the catalog adds it and selects its line; clicking a technology or a member attaches it to the selected line, and clicking a line selects it. `/technologies` and `/staff`, the pages this used to be split over, redirect to `/`. Plans are saved in the database under a name.

The catalog only offers what the plan can actually take next: the wiki's "Requires …" lists are parsed too, so a focus stays out of the list until its prerequisites are in the plan.

**No icon is ever downloaded and nothing here touches the network.** `data/focuses.html` and `data/technologies/*.html` are wiki pages saved by hand, and a focus or a technology keeps a *link* to the wiki's copy of its icon: the browser is what fetches the images.

**Where the pages come from.** The saved pages are copied from a local scraper's mirror of the wiki, `~/python/perso/scraper/pages/hoi4.paradoxwikis.com/`, one file per page under its wiki title: `German_national_focus_tree.html` → `data/focuses.html`, `German_Reich.html` → `data/politics.html`, `<Name>_technology.html` → `data/technologies/` (the DLC versions, not the `(Basic)` ones). Copying is done by hand and nothing reads the mirror directly. The focus and politics copies in `data/` are older saves than the mirror's, so after copying a newer page in, re-run the matching import with `--json-only` (or `parse-politics`) and read `git diff data/` before trusting the parser with it.

## Commands

```sh
cargo run                                   # serve on http://127.0.0.1:5000 (HOI4_PLANNER_DATA_DIR, BIND_ADDR, HOI4_PLANNER_SOCKET, RUST_LOG, NO_COLOR)
cargo run -- seed                           # fill the three catalogs from the JSON compiled into the binary
cargo run -- import-focuses                 # data/focuses.html -> data/focuses.json -> database
cargo run -- import-focuses --from-json     # seed from the committed JSON (fast, no page needed)
cargo run -- import-focuses --json-only     # re-parse the page, touch no database
cargo run -- import-focuses --from-json --database-url other.db   # ... into another database
cargo run -- import-technologies            # data/technologies/*.html -> data/technologies.json -> database
cargo run -- import-technologies --from-json   # same flags as import-focuses
cargo run -- import-staff                   # data/politics.html -> data/staff.json -> database
cargo run -- import-staff --from-json       # same flags as import-focuses
cargo run -- parse-politics                 # data/politics.html -> politics.json + staff.json
cargo run -- seed --database-url other.db   # ... into another database
cargo run -- healthcheck                    # ask a running server for /health, print the answer
cargo test                                  # unit tests + tests/ (in-memory SQLite)
cargo test --test planner_api <name>        # one integration test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

No external service is needed: the database is a file, `hoi4_planner.db` in the data folder (`HOI4_PLANNER_DATA_DIR`, default `data`), created with its folder at the first connection, and pending migrations are applied every time the app connects. The catalogs are not in it until they are seeded; an empty one comes up healthy and says so on the page.

## The rules of a plan

A national focus costs **35 days** unless it is one of the few that take half a term (**14**) or two (**70**), so a plan is an *order* plus the handful of focuses that differ: each completes when everything before it is done plus its own term, and the start date itself is not counted as a day of work. From the default 1 Jan 1936, an unmarked first focus lands on 5 Feb 1936.

The wiki page carries no duration to parse, so the odd term is picked by hand on the focus's row in the plan (a `14 / 35 / 70` picker, the middle button being the plan's own `days_per_focus` and clearing the mark) and kept in `focus_days`, `{focus id: days}` holding **only** the focuses that differ. Keyed by id rather than by position, so dragging cannot move a term onto the wrong focus; a mark for a focus not in the order is dropped on save (`PlanRecord::store`), and one that is not positive falls back to the default rather than failing (`domain::days_for`).

Nothing derived is stored: `focus_ids`, `focus_days`, `focus_technologies`, `focus_staff`, `focus_comments`, `start_date` and `days_per_focus` are the plan, and `domain::schedule` makes the dates.

The plan being built is kept too, without being named: every edit is written to **the draft**, one row of `plans` under an id no name can slug to (`domain::DRAFT_PLAN_ID`, `~draft`), so closing the tab loses nothing and the page comes down with it already in hand. It is left out of `PlanRecord::list_saved`, as it belongs to the panel and not to the dropdown, and **Vider** is what throws it away (`DELETE /api/draft`), leaving the saved plans alone.

## What a focus depends on

The page states a focus's prerequisites in the second cell of its row, in two forms and no others (there is no "all of the following"):

- **`Requires the following:`**: every focus in the list must be taken.
- **`Requires one of the following:`**: any one of them will do, and this form can be nested inside the first ("requires A, and one of B or C").

Both are stored as the same shape, `requires`: **a list of groups of focus ids, each group needing one of its focuses taken**. "The following" is one group per focus, "one of the following" is a single group of them all, and a nested list is just another group, which makes checking a plan one test rather than two (`Focus::unmet`). A row's `Mutually exclusive with:` becomes `excludes`, made symmetric at parse time because the page does not always write both sides.

Both are parsed by walking the tags (`wiki::requirement_list`) rather than with a regex: the lists nest, and no regex can tell which `</ul>` closes which. What the catalog cannot name (an ideology, a country, a focus from a branch this saved page does not carry) is dropped once the whole page is read, and a group that empties out stops being a requirement at all rather than stranding its focus forever.

A focus is **available** when every group is met and nothing it excludes is planned. That is what the catalog filters on (« seulement les disponibles », on by default; turn it off and a gated focus is dimmed, locked, and still clickable), and what `schedule` re-checks *in order*: a prerequisite has to come **earlier**, so dragging one below what needs it marks that row `needs`/`clashes` rather than quietly dating a plan nobody could play.

## Technologies

The research tree is country-agnostic: the pages describe what every nation shares, and nothing here filters it for Germany. `data/technologies/` holds the DLC version of each page (the `(Basic)` pages are not used), and `wiki::parse_technology_pages` reads them together, as prerequisites cross pages. A section whose notice says "Available only with the … DLC disabled" is skipped, so **every DLC is taken to be on**. The secret and special-project techs are kept: they have no year and no research time, and the catalog hides them behind a checkbox.

A Prerequisites cell is `–`, links (each one a group), `One of:` and a list (one group), and links after a red `Excludes` (exclusions, made symmetric). "Can unlock this tech" and "Starts with this tech" cells are **not** prerequisites: their links go to other countries' focuses, some with the very anchor of a technology. A link is only followed within a page or to a `*_technology` page, and matched case-insensitively, as the wiki is inconsistent. What the catalog cannot name is dropped, as for focuses. Two links are dead on the saved pages: `#Engine_IV` (Centrifugal Jet) and `#Amphibious_Light_Tank_Chassis` (Amphibious Drive), so those two technologies come out with no prerequisite.

**For now a technology has no research schedule of its own.** A plan keeps `focus_technologies`, `{focus id: [technology ids]}`, and a technology completes when its focus does; one focus can carry several. `base_days` is parsed and shown, but drives no date. Keyed by focus id, so dragging a focus carries its technologies along. Removing a focus removes them too (`drop()` in `app.js` says how many, and `PlanRecord::store` trims the same way), as a technology has no date without its focus. The staff are attached the same way (see below).

A technology is **available** on a line when every group is met by what is on that line or an earlier one, and nothing it excludes is planned anywhere. On a chip the check is stricter on order: a prerequisite has to come earlier in the walk down the plan, chip by chip, or the chip shows a `⚠`. The pages do not say what a nation starts with, so a technology **dated before the plan's start year counts as researched**: it meets any prerequisite without being planned, and is hidden under « seulement les disponibles » (it can still be planned by hand with the filter off). This approximates, as no country really starts with all of them.

These rules live **only** in `static/technologies.js`: the server answers nothing about the order of technologies, so they are not part of the duplication below.

## Staff and designers

The staff catalog is the *Staff and designers* section of `data/politics.html`, its **Personnel** and **Companies** tables (advisors, theorists, chiefs, high command, field marshals, generals, admirals, industrial concerns, designers) and not the Organizations, which are the designers' trait trees. `wiki::parse_staff` reads it, `import-staff` stores it (`StaffMember`, `StaffRecord`); `staff.json` is the same file `parse-politics` writes, so either leaves `git diff data/` empty.

**A member's post is the table it is listed in** (`section`). Someone listed in several tables is one member per table, with the table in the id (`Heinz_Guderian__Theorists`, `Heinz_Guderian__Generals`), so picking the row is choosing the post. Like a technology, a member is hired **with a focus** and takes its date: a plan keeps `focus_staff`, `{focus id: [staff ids]}`, the plan draws them in the third column of the focus's line (a chip showing the member's post), and removing a focus removes its staff (`drop()` in `app.js`, `PlanRecord::store`). A member is hired once, on one line. Slots are not counted: the page does not say how many each post holds.

A member's requirements are focus ids, **not resolved against the catalog**: about half run through branches the saved focus page does not carry, and those are named from their id. On a line, a member is hirable when every group is met by a focus on that line or an earlier one, and nothing it excludes is planned; otherwise its chip warns (`⚠`), and its catalog row is locked against the selected line, but it can still be hired. The row also says from which line's date the plan allows it (« dès le … », the latest group's earliest focus). « seulement les disponibles » is off by default for that reason. Like the technology rules, these live **only** in `static/staff.js`.

## Comments

Free text, with no catalog and no rule: a plan keeps `focus_comments`, `{focus id: [text]}`, drawn as chips in the fourth column of the focus's line. The cell's « + » writes one, a double click rewrites it (Enter keeps it, Shift+Enter is a new line, Escape gives up, leaving the box keeps it), and emptying it removes it. Comments drag within and between lines like the other chips, and go with their focus when it is removed (`drop()` in `app.js`, `PlanRecord::store`). On the page each comment gets an id of its own (`state.comments` holds ids, `state.commentText` their texts), as two comments can say the same thing and `detach`/`dragChips` move chips by id; those ids are never stored. All of it lives in `static/comments.js`.

## Architecture

Stack: axum (HTTP), askama (server-rendered HTML in `templates/`), sqlx on SQLite. No front-end build: `static/app.css`, `static/common.js` (what the scripts share: requests, the draft timer, the two rules, and the chips attached to focus lines: `detach`, `dragChips`, each kept to its column), `static/app.js` (the plan's state, the focus tab, the lines and their focus column, the tabs, saving and files), `static/technologies.js` and `static/staff.js` (`Planner.technologyPane` / `Planner.staffPane`: one tab and one column each, called once by `app.js` with a `ctx` giving them the shared state, the selected line, the dates and the focus names), and `static/comments.js` (`Planner.commentPane`: a column and no tab, with `load`/`stored` turning the plan's texts into the page's ids and back). One binary (`src/main.rs`), whose subcommands are in `src/cli/`.

Layers, from the inside out:

- **`src/domain/`**: pure, no I/O, unit-tested. `Focus` (`unmet`, `conflicts`, `available`), `Catalog` (focuses by id), `schedule` (`schedule`, `days_for`, `completion_date`, `total_days`, `parse_start`, `ScheduleRow`), `Plan` and `DRAFT_PLAN_ID`, `PlanFields` (what a plan is made of, and the one check of a posted body: a save, the draft, an export and an imported file all go through `PlanFields::from_json`), `plan_file` (the exchange format: `PlanFile`, `read_plan_file`), `slugify`, `rules` (the constants of the game), `Technology` and `TechnologyCatalog` (technologies by id), `StaffMember` and `StaffCatalog` (staff by id).
- **`src/wiki/`**: pure too, the saved pages to catalogs. `html_token` is a tolerant tokenizer, on which `requirement_list` (the `<ul>`/`<li>` nesting of one cell) and `html_tree` (a whole tree, as the country page's tables nest and span rows) are built. `focus_page::parse_wiki_page` reads the focus page. `leaders_page::parse_leaders` and `staff_page::parse_staff` read the *German Reich* page (`data/politics.html`) into `PoliticsEntry`, a focus's shape with `section` being the leader's ideology or the staff's table: advisors, theorists, chiefs, high command, commanders, concerns, designers, but not the organizations, which are the designers' trait trees. A leader's "Via focus" routes are one group; a Requirements cell is a boolean tree (`Condition`: "One of"/"All of", "Has (not) completed focus", "Visible when" included) brought to groups, with whatever is not a focus dropped and an alternative left with no focus lifting the whole choice. These focus ids are **not** resolved against the catalog: most routes run through branches the saved focus page does not carry. `technology_page::parse_technology_pages` reads the research pages (see Technologies).
- **`src/db/`**: one record type per table (`FocusRecord`, `TechnologyRecord`, `StaffRecord`, `PlanRecord`) with async functions taking the pool. `db::connect` runs `migrations/` (embedded with `sqlx::migrate!`). Queries use runtime `sqlx::query`/`query_as` (no compile-time macros, so no database at build time). `requires`, `excludes`, `focus_ids`, `focus_days`, `focus_technologies`, `focus_staff` and `focus_comments` are JSON columns: they are only ever read whole, and a plan may hold ids the catalog has lost, so nothing joins the two tables. Stamps are UTC text to the microsecond, which sorts chronologically. Schema changes go in a new migration file, never by editing an applied one.
- **`src/service/`**: one use case per file. `save_plan` (slug + name checks), `draft`, `PlanDownload` (a plan as the file a browser saves), `import_focuses`, `import_technologies`, `import_staff` (each from the saved pages, the JSON of `data/`, or the JSON compiled into the binary: `BundledCatalog`, which `seed` uses), `parse_politics`. Services do not print; the CLI says what happened.
- **`src/web/`**: `router.rs` lists the routes. `main_handlers` serves the page (`PlannerPage` built from `PlannerData`, `templates/planner.html`, which draws the three catalogs' rows (`FocusRow`, `TechnologyRow`, `StaffRow`) with their `data-requires` / `data-excludes` and hands the draft over in `data-draft`; the lines are drawn and dated in the browser), the redirects of `/technologies` and `/staff`, and `/health`. `static_asset` serves `static/`, compiled into the binary (`include_str!`) with an `ETag` hashed at compile time; a file added to `static/` must be listed there, which a unit test checks. `plan_handlers` and `plan_file_handlers` speak JSON, as the planner is one screen that never reloads. Every API failure is an `ApiError`: `{"error": "..."}` and a status, 503 when the database failed, so that it reads as a message next to the save button. The page itself still draws when the database fails, saying so, and says how to fill each catalog still empty.
- **`src/cli/`**: `Command` (the command line), `run`, `serve`, `healthcheck`, `listener` (TCP, or a Unix socket when `HOI4_PLANNER_SOCKET` is set: a stale one removed first, `0666`, removed at SIGTERM or Ctrl-C).

**Every link is relative** (`static/app.js`, `api/plans`, the redirects' `./`): a proxy may serve the app under a sub-path it strips, and the app does not know which.

### One page, one plan

All three scripts edit one `state` (`app.js`), and every edit writes the whole plan to the draft. **Two tabs open at once still overwrite each other**: last write wins.

### The one deliberate duplication

What a focus allows, its term and its completion date are computed **both** in `src/domain/` and in `static/app.js`. Dragging has to re-date the whole list on every pointer move, and a round trip per frame would make the drag feel dead. The server's copy stays the authority for anything it answers itself: the dates written into an exported file. Change the rule in one and change it in the other.

## A plan as a file

Export and import move one plan between databases (a laptop's and the server's, or two people's) as a single JSON file. The format is `domain::plan_file`'s and the browser writes none of it: **Exporter** posts the panel and saves what comes back, **Importer** posts what it read off disk and draws what comes back.

```
POST /api/plans/export        the panel, as a download
GET  /api/plans/<id>/export   a saved plan, same file, for curl
POST /api/plans/import        a file, checked, handed back as a plan to draw
```

The file holds the plan and nothing else (`name`, `focus_ids`, `start_date`, `days_per_focus`, `focus_days`, `focus_technologies`, `focus_staff`, `focus_comments`) inside an envelope naming what it is (`format`, `version`), so a JSON file that is not one of ours is refused with a sentence instead of half-read into the panel. Beside them is `focuses`, a **readable echo**: each focus's name, term, completion date, technologies, staff with their posts, and comments, for whoever opens the file. An import ignores it, because the order is the plan and `schedule` makes those dates again. Nothing of the database travels: no id, no stamps, no timestamp of the export itself, and `focus_days` is sorted. Exporting one plan twice gives the same bytes, which is what lets a plan be diffed or kept in git. Version 2 added `focus_technologies`, version 3 `focus_staff`, version 4 `focus_comments`. Older files still import, version 1 (including those exported by the earlier Python version of this app) with no technologies, staff or comments, version 2 with no staff or comments, version 3 with no comments.

**Importing stores nothing.** A plan arriving is a plan being built, so it lands in the panel, where it is drawn, reordered, renamed and then saved like any other, and where the draft picks it up on the way. Ids this catalog does not have come back in `unknown`, `unknown_technologies` and `unknown_staff` and stay in the plan rather than being dropped quietly; the panel says how many, and neither is drawn or dated.

`POST /api/plans/export` and `/import` share their path with a plan whose name slugs to `export` or `import`, which the router handles explicitly (`GET` and `DELETE` on those paths mean the plan).

## Deploying

With `cargo deploy`, described in `[package.metadata.deploy]` of `Cargo.toml`. Nothing about the server lives in this repository: its values come from the `DEPLOY_*` environment variables.

```sh
cargo deploy setup          # once: prepare the server
cargo deploy                # build, ship, restart, health check (/health)
cargo deploy run -- seed    # fill the three catalogs from the deployed binary
cargo deploy rollback       # back to the previous version
cargo deploy status         # what runs, and what /health says
cargo deploy logs           # follow the service's journal
```

A release is the binary and nothing else, which is why `static/` and the catalogs' JSON are compiled in. The app gets its socket in `HOI4_PLANNER_SOCKET` and its data folder in `HOI4_PLANNER_DATA_DIR` (kept across releases, migrations applied at start), and runs sandboxed with no network: it opens no outgoing connection. `seed` only replaces the `focuses`, `technologies` and `staff` tables, so a re-parsed page goes out with a deploy followed by a seed: re-parsing is a laptop job, as the saved wiki pages are not shipped.

## Tests

Pure rules are unit-tested next to their code (`#[cfg(test)] mod tests`). `tests/` drives everything else, each file a topic: `focus_page`, `technology_page` and `politics_page` (the parsers), `plan_storage` (records and services), `planner_api` and `plan_file_api` (the JSON routes), `planner_page` (the page, its assets, the old pages' redirects, `/health`), `import_commands` (the commands reading the saved pages).

No test touches the network, and a database is either in memory (`sqlite::memory:`) or a file in a temporary folder. `tests/common/app.rs` holds `TestApp`, the real router on an in-memory database, driven one request at a time; `TestApp::lose_database` closes the pool, which is how every route is tested on a failing database.

The saved pages are never read by a test: `tests/common/wiki_markup.rs`, `technology_markup.rs` and `politics_markup.rs` build the few rows of markup each test needs. To check the parsers against the real pages, re-run `import-focuses --json-only`, `import-technologies --json-only` and `parse-politics` (or `import-staff --json-only`): `git diff data/` must be empty unless a parser was meant to change.

## Development guidelines

### Imports

- Prefer `crate::`-rooted paths for imports across modules; treat this as the Rust equivalent of "absolute imports."
- `super::` is acceptable within tightly nested module hierarchies (e.g. a tests submodule referencing its parent, or a submodule referencing its direct parent's types).
- Avoid deep `super::super::` chains — restructure the module tree instead.
- Avoid glob imports (`use foo::*;`) except for prelude modules explicitly designed for it, or in test modules (`use super::*;`).

### Type Annotations

- Annotate all `pub` function signatures explicitly (parameters and return type), even where inference would work.
- Let type inference handle local variable bindings inside function bodies.
- Prefer concrete types over generics unless genericity provides real, used value (called with more than one concrete type, or part of a public API meant to be generic).
- Avoid `Box<dyn Any>` and other type-erasure patterns unless strictly necessary; justify with a short comment.
- Avoid `Box<dyn Error>` for library code — prefer a concrete, well-typed error enum. `Box<dyn Error>` is acceptable at binary/application boundaries (e.g. `main`).

### Docstrings

- All `pub` items (functions, methods, structs, enums, traits) require a rustdoc comment (`///`), following standard rustdoc sections (`# Arguments`, `# Returns`, `# Errors`, `# Panics` as applicable).
- Private (non-`pub`) functions get a docstring only when the logic isn't obvious from the signature and name.
- Keep docstrings proportionate: short for simple functions, more detailed only when it adds real clarity.
- Do not restate information already conveyed by the signature (parameter types, obvious return values).
- Enforce at the crate level with `#![warn(missing_docs)]` (or `#![deny(missing_docs)]` for libraries).

### Comments

- Do not add comments to explain straightforward or self-evident code.
- Add comments only when the logic is genuinely complex, non-obvious, or could be misread.
- Keep comments short and synthetic — a brief note on intent or reasoning, not a narration of what the code does.
- Avoid verbose or redundant comments; never caption the obvious.

### Error Handling

- Use `Result<T, E>` for recoverable errors and `Option<T>` for absence. Do not use panics as control flow.
- Propagate errors with `?` rather than manual `match`/`if let` chains, unless the error needs to be transformed or contextualized at that point.
- Do not use `unwrap()` or `expect()` in library or production code paths. Reserve for tests, examples, and prototypes.
- If used outside those contexts because a failure is provably impossible, justify with a short comment (e.g. `// safe: regex is a compile-time constant`).
- Define proper error types (plain enums, or via `thiserror`) rather than stringly-typed errors (`Err("something failed".to_string())`) or blanket `Box<dyn Error>` in library code.
- Do not silently discard errors with `.ok()`, `let _ =`, or similar — handle, log, or propagate explicitly.
- `panic!` (and `unreachable!`, `todo!`, `unimplemented!` left in shipped code) is reserved for genuine invariant violations (bugs), not expected failure modes.

### Function Size & Decomposition

- Soft limit of 50 lines per function.
- If a function exceeds this limit, break it into smaller, well-named sub-functions, each with a single clear responsibility and its own concise docstring.
- Prefer composition of small functions over deeply nested logic or long procedural blocks.
- Prefer early returns / `?`-based flattening over deeply nested `match`/`if let` pyramids.

### Types & Modules

- One primary public type (struct, enum, or trait) per file, with its associated `impl` block(s).
- File name (snake_case) matches the type name (e.g. `struct UserAccount` → `user_account.rs`).
- Exceptions: small, tightly-coupled types may share the file with the primary type — e.g. an error enum returned by its methods, a config/builder struct feeding it, or a result type it produces.
- A closely-related family of small types with no independent identity (e.g. AST node variants, a state-machine's states) may live together in one file organized around the concept they jointly express, rather than one type per file. Split it out once it grows large enough to be its own topic.
- Traits implemented by multiple unrelated types get their own file; a trait implemented by only one type may be co-located with that type.

### File & Module Organization

- Avoid monolithic files. Split code by topic/responsibility into modules (`mod foo;`).
- Group related modules into a directory-based module tree (subpackages) when a topic grows large enough.
- Keep `mod.rs` / inline module-declaration files thin — `pub use` re-exports only, no implementation logic.
- Keep `lib.rs` / `main.rs` as a thin entry point: module declarations, top-level wiring, and re-exports — no business logic.

### General Principles

- Favor readability and small, testable units over cleverness.
- Prefer explicit over implicit in naming, typing, and structure.
- Split files as soon as they start doing "too much" rather than waiting for a refactor pass.
- Prefer composition over inheritance-like patterns; use traits for shared behavior, not deep hierarchies.
- Align with idiomatic Rust and clippy/rustfmt defaults rather than fighting them; deviate only when this style guide explicitly says so.

### Language

- Code is English, whatever language the prompt is in. Identifiers, function names, comments, docstrings, commit messages and log lines are written in English even when the request or the discussion is in French.
- User-facing text is French, and stays French. Error messages surfaced to users, CLI output, labels, templates — anything a reader sees is French, UTF-8, accents kept. Do not translate it, and write new user-facing text in French.
