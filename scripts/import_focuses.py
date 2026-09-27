"""Seed the `focuses` collection from the saved wiki page.

    python scripts/import_focuses.py              # data/focuses.html -> JSON -> Mongo
    python scripts/import_focuses.py --json-only  # stop at data/focuses.json
    python scripts/import_focuses.py --from-json  # skip the page, load the JSON

The database seeded is the one `.env` names, unless this run is pointed
elsewhere: `--env` picks the suffix (`hoi4_planner_<env>`) and `--mongo-uri` the
server, both by setting what `core.config` and `core.db` read anyway, so nothing
below has to know which database it is writing to.

    python scripts/import_focuses.py --from-json --env prod \
        --mongo-uri mongodb://localhost:27018

That is how the laptop fills the server's catalog: the server's mongod listens on
its own host only, so reach it through an SSH tunnel opened beside this
(`ssh -N -L 27018:localhost:27017 ced@nuc150`) rather than by exposing the port.
`make seed` stays the other way round — the same JSON, seeded from inside the
deployed container.

**Nothing here fetches anything**, the database aside. `data/focuses.html` is a copy
of the German national focus tree page saved by hand, and the icons stay where
they are: the catalog keeps a link to the wiki's copy, and the browser is what
fetches them.

The JSON in the middle is deliberate. Parsing 2 MB of wiki markup is the slow and
fragile half, so its result is written to `data/focuses.json` and committed:
`--from-json` then seeds a database in a moment, and a checkout with no saved page
can still fill one.
"""

import argparse
import json
import os
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.config import ENV_NAMES, FOCUSES_JSON, WIKI_PAGE, current_db_name
from core.db import DatabaseError, replace_focuses
from core.focuses import parse_wiki_page


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--json-only",
        action="store_true",
        help="parse the page and write the JSON, without touching the database",
    )
    parser.add_argument(
        "--from-json",
        action="store_true",
        help="seed from data/focuses.json, without re-parsing the saved page",
    )
    parser.add_argument(
        "--env",
        choices=sorted(ENV_NAMES),
        help="which database to seed (hoi4_planner_<env>), instead of APP_ENV's",
    )
    parser.add_argument(
        "--mongo-uri",
        metavar="URI",
        help="which server to seed, instead of MONGO_URI's (e.g. a tunnelled prod)",
    )
    return parser.parse_args(argv)


def target(env_name=None, mongo_uri=None):
    """Point this run at another database, before anything opens a connection.

    Set as environment variables rather than passed down: `current_db_name()` and
    `connect_db()` read them at call time, so this is the whole of it — no
    function below takes a database as an argument, and a run left to `.env`
    behaves exactly as it did.
    """
    if env_name:
        os.environ["APP_ENV"] = env_name
    if mongo_uri:
        os.environ["MONGO_URI"] = mongo_uri


def server():
    """The server about to be written to, as a line worth printing.

    Credentials are stripped: seeding prod is the case this exists for, and the
    URI is likely to carry a password on the way to a remote mongod.
    """
    uri = os.environ.get("MONGO_URI", "")
    return re.sub(r"(?<=//)[^@/]*@", "", uri) or "the server MONGO_URI names"


def read_focuses(from_json):
    """The catalog, either re-parsed from the page or read back from the JSON."""
    if from_json:
        if not FOCUSES_JSON.exists():
            raise FileNotFoundError(f"{FOCUSES_JSON} not found. Run without --from-json first.")
        return json.loads(FOCUSES_JSON.read_text(encoding="utf-8"))

    if not WIKI_PAGE.exists():
        raise FileNotFoundError(
            f"{WIKI_PAGE} not found. Save the German national focus tree page there, "
            f"or seed from the committed JSON with --from-json."
        )
    return parse_wiki_page(WIKI_PAGE.read_text(encoding="utf-8"))


def main(argv=None):
    args = parse_args(argv)
    target(args.env, args.mongo_uri)

    focuses = read_focuses(args.from_json)
    if not focuses:
        print("No focus found in the saved page — has its markup changed?", file=sys.stderr)
        return 1
    print(f"{len(focuses)} focuses parsed")

    if not args.from_json:
        FOCUSES_JSON.write_text(
            json.dumps(focuses, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
        print(f"wrote {FOCUSES_JSON}")

    if args.json_only:
        return 0

    print(f"seeding {current_db_name()} on {server()}")
    try:
        stored = replace_focuses(focuses)
    except (*DatabaseError, EnvironmentError) as error:
        print(f"Could not write to the database: {error}", file=sys.stderr)
        return 1

    print(f"{stored} focuses stored in {current_db_name()}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
