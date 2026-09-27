"""Where a run gets its settings: `.env`, and which database.

The bottom of the package: this module knows about files and environment
variables, and about nothing else here.
"""

import os
from pathlib import Path

from dotenv import load_dotenv

ROOT = Path(__file__).resolve().parent.parent
DATA_DIR = ROOT / "data"

# The wiki page saved by hand, and the catalog `scripts/import_focuses.py`
# extracts from it. The JSON is the intermediate the import writes and the
# database is seeded from — kept in the repo so a fresh checkout can seed itself
# without re-parsing the 2 MB page.
WIKI_PAGE = DATA_DIR / "foo.html"
FOCUSES_JSON = DATA_DIR / "focuses.json"

load_dotenv(ROOT / ".env")

# Base database name; the database actually used is this name + the current env
# suffix (see current_db_name below). APP_ENV picks which local Mongo database is
# used, so a dev run never touches prod data (and vice versa).
DB_NAME = "hoi4_planner"
ENV_NAMES = ("dev", "prod")


def load_env(required_vars):
    env = {name: os.environ.get(name) for name in required_vars}
    missing = [name for name, value in env.items() if not value]
    if missing:
        raise EnvironmentError(
            f"Missing required environment variables: {', '.join(missing)}. "
            f"Copy .env.example to .env and fill it in."
        )
    return env


def current_env():
    env_name = os.environ.get("APP_ENV", "dev").strip().lower()
    if env_name not in ENV_NAMES:
        raise EnvironmentError(f"APP_ENV must be one of {sorted(ENV_NAMES)}, got '{env_name}'")
    return env_name


def current_db_name():
    return f"{DB_NAME}_{current_env()}"
