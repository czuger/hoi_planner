"""The local MongoDB: the connection, and every read and write.

The only module here that talks to the database — `core.focuses` is pure and
`core.models` only describes the collections. Callers name nothing but plain
values and get plain dicts back: nothing outside this module and `core/models/`
ever holds a document object, which is what keeps the Flask views and the import
script free of the ORM.

The connection is opened on the first call rather than at import time, so
importing `core.db` never needs a reachable server — and a test can stand a fake
one in by replacing `connect_db()`.
"""

import os
import re
import unicodedata
from datetime import datetime, timezone

from mongoengine import connect, disconnect
from mongoengine import get_db as mongoengine_db
from mongoengine.errors import MongoEngineException
from pymongo.errors import PyMongoError

from .config import current_db_name
from .focuses import DEFAULT_DAYS_PER_FOCUS
from .models import Focus, Plan

# What every caller catches around a database call. Two families rather than one:
# pymongo raises when the server cannot be reached or refuses a command,
# mongoengine when a document does not fit its model. Both mean "the database did
# not do what was asked", and every route reports them the same way.
DatabaseError = (PyMongoError, MongoEngineException)

# Long enough for a local mongod, short enough that a dead server shows up as an
# error on the page rather than as a hung request.
SERVER_SELECTION_TIMEOUT_MS = 3000

# The database the current connection was opened on, so a run that switches
# APP_ENV mid-flight (the tests do) reconnects instead of writing to the wrong one.
_connected_to = None

# The plan being built right now, saved on every edit so a reload does not lose
# it. One document, in the same collection as the saved plans, under an id no
# name could ever take: `slugify()` only ever yields `[a-z0-9-]`, so the tilde is
# what keeps a plan someone calls "Draft" from landing on top of this one.
DRAFT_PLAN_ID = "~draft"


def connect_db():
    """Register the connection the models use, once per database name.

    Every function below calls this first, which is also the single seam the tests
    replace: with it stubbed out, nothing here ever reaches for a real server.
    """
    global _connected_to
    db_name = current_db_name()
    if _connected_to == db_name:
        return

    mongo_uri = os.environ.get("MONGO_URI")
    if not mongo_uri:
        raise EnvironmentError(
            "Missing MONGO_URI environment variable. Copy .env.example to .env and fill it in."
        )

    if _connected_to is not None:
        disconnect()
    connect(
        db=db_name,
        host=mongo_uri,
        serverSelectionTimeoutMS=SERVER_SELECTION_TIMEOUT_MS,
        # Explicit only to keep pymongo from warning about its legacy default; no
        # UUID is ever stored here.
        uuidRepresentation="standard",
    )

    # A database named in MONGO_URI ("…:27017/somewhere") wins over `db` above, and
    # would quietly take a dev run onto another database. APP_ENV is what picks it.
    connected = mongoengine_db().name
    if connected != db_name:
        disconnect()
        raise EnvironmentError(
            f"MONGO_URI points at the database '{connected}', but APP_ENV asks for "
            f"'{db_name}'. Leave the database out of MONGO_URI."
        )

    _connected_to = db_name


def now():
    """The clock, in one place: `updated_at` is what the plan list is ordered by,
    and a test needs to advance it rather than race a millisecond.

    **Naive UTC**, because that is what comes back out: Mongo stores a datetime
    without a zone, so a stamp read from an existing document is naive while a
    fresh `datetime.now(timezone.utc)` is aware — and an overwrite, which keeps
    the stored `created_at`, would hand back a differently-shaped one than the
    save that created it. Dropping the zone here keeps every stamp comparable.
    """
    return datetime.now(timezone.utc).replace(tzinfo=None)


def slugify(name):
    """A plan's name as its id: lowercase, ascii, dashes.

    The id travels through a URL (the save and delete routes post to it), so it
    has to survive one. A name that slugs to nothing — punctuation only — has no
    usable id, and `save_plan()` refuses it rather than storing a document nothing
    can address.
    """
    ascii_name = unicodedata.normalize("NFKD", name).encode("ascii", "ignore").decode()
    return re.sub(r"[^a-z0-9]+", "-", ascii_name.lower()).strip("-")


# --- the catalog ---------------------------------------------------------------


def replace_focuses(focuses):
    """Rewrite the catalog wholesale, and say how many rows it now holds.

    Wholesale because the wiki page is the truth and the collection is only a copy
    of it: a re-import should lose the focuses the page no longer has rather than
    leave them behind. Plans are untouched — they store ids, and `schedule()`
    drops the ones the catalog no longer knows.
    """
    connect_db()
    Focus.objects.delete()
    if focuses:
        Focus.objects.insert([Focus.from_dict(focus) for focus in focuses], load_bulk=False)
    return Focus.objects.count()


def load_focuses():
    """The whole catalog, in the page's own order — which groups it by branch."""
    connect_db()
    return [doc.to_dict() for doc in Focus.objects.order_by("position")]


def focus_count():
    """How many focuses are stored, for the "seed the database first" check."""
    connect_db()
    return Focus.objects.count()


# --- plans ---------------------------------------------------------------------


def load_plans():
    """Every *saved* plan, most recently touched first.

    The draft is stored beside them and left out here: it is the panel as it
    stands, not something to offer in the dropdown or delete by name.
    """
    connect_db()
    return [
        doc.to_dict()
        for doc in Plan.objects(id__ne=DRAFT_PLAN_ID).order_by("-updated_at")
    ]


def load_plan(plan_id):
    """One plan, or `None` when there is no such id."""
    connect_db()
    doc = Plan.objects(id=plan_id).first()
    return doc.to_dict() if doc else None


def save_plan(
    name, focus_ids, start_date, days_per_focus=DEFAULT_DAYS_PER_FOCUS, focus_days=None
):
    """Create or overwrite the plan of that name, and hand back what was stored.

    Keyed by the slug of the name, so saving "Barbarossa rush" twice updates one
    document. `created_at` is kept from the existing one — an overwrite is the
    same plan revised, not a new one.

    Returns `None` when the name has no usable slug; the caller turns that into
    the message on the page.
    """
    plan_id = slugify(name)
    if not plan_id:
        return None
    return _store_plan(
        plan_id, name.strip(), focus_ids, start_date, days_per_focus, focus_days
    )


def save_draft(
    name, focus_ids, start_date, days_per_focus=DEFAULT_DAYS_PER_FOCUS, focus_days=None
):
    """Write the plan on screen over the draft, and hand back what was stored.

    The same write as `save_plan()` against a fixed id, so the panel survives a
    reload without being named first: the browser posts here on every edit, and
    the name is only what is typed in the box so far — nothing is slugged from
    it, and it may well be empty.
    """
    return _store_plan(
        DRAFT_PLAN_ID, str(name or "").strip(), focus_ids, start_date, days_per_focus, focus_days
    )


def load_draft():
    """The plan being built, or `None` when there is no draft to pick back up."""
    return load_plan(DRAFT_PLAN_ID)


def clear_draft():
    """Throw the draft away, and say whether there was one — the Clear button."""
    return delete_plan(DRAFT_PLAN_ID)


def _store_plan(plan_id, name, focus_ids, start_date, days_per_focus, focus_days):
    """Write one plan document, saved or draft, and give it back as a dict.

    `focus_days` is trimmed to the order being stored: a focus dragged out of the
    plan takes its term with it, rather than leaving a mark behind that nothing
    on the page can see or clear.
    """
    focus_ids = list(focus_ids)
    planned = set(focus_ids)
    marks = {
        focus_id: int(days)
        for focus_id, days in (focus_days or {}).items()
        if focus_id in planned
    }

    connect_db()
    stamp = now()
    existing = Plan.objects(id=plan_id).first()

    plan = Plan.from_dict(
        {
            "id": plan_id,
            "name": name,
            "focus_ids": focus_ids,
            "start_date": start_date,
            "days_per_focus": int(days_per_focus),
            "focus_days": marks,
            "created_at": existing.created_at if existing else stamp,
            "updated_at": stamp,
        }
    )
    plan.save()
    return plan.to_dict()


def delete_plan(plan_id):
    """Throw one plan away, and say whether there was one to throw."""
    connect_db()
    return Plan.objects(id=plan_id).delete() > 0
