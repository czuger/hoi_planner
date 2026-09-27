"""Fixtures every test file shares: the fake database, and a client on it.

No test here ever needs a running mongod or a network: the models are pointed at
an in-memory mongomock server, and `core.db.connect_db()` — the one place that
would reach for a server — is stubbed out. `data/foo.html` is never read either;
the parser tests build the few rows of markup they need (`tests/helpers.py`).
"""

import copy
import itertools
from datetime import datetime, timedelta

import mongoengine
import mongomock
import pytest
from pymongo.errors import ServerSelectionTimeoutError

import app as flask_app
from core import db as core_db
from core.models import Focus, Plan

# The database the models are bound to for the whole session. Its name matters
# only in what the page prints (`current_db_name()` gives the same one, since
# clean_env drops APP_ENV).
TEST_DB_NAME = "hoi4_planner_dev"

MODELS = (Focus, Plan)


class FakeCollection:
    """One mongomock collection, seen as the list of documents tests set up.

    `docs` is the whole interface: assigning replaces the collection's contents,
    reading gives them back with Mongo's `_id` shown as the app's own `id`. Both
    collections here are keyed by an id of their own — a wiki anchor, a plan slug
    — so the swap always applies. Documents are copied in, so a test that hands
    over a shared dict does not get it back stamped with an `_id`.
    """

    def __init__(self, collection):
        self._collection = collection

    @property
    def docs(self):
        return [
            {("id" if key == "_id" else key): value for key, value in doc.items()}
            for doc in self._collection.find()
        ]

    @docs.setter
    def docs(self, docs):
        self._collection.delete_many({})
        if docs:
            self._collection.insert_many(
                [
                    {("_id" if key == "id" else key): value for key, value in copy.deepcopy(doc).items()}
                    for doc in docs
                ]
            )


class FakeDb:
    """`fake_db["focuses"].docs` — how a test seeds and reads a collection."""

    def __init__(self, connection):
        self._connection = connection

    def __getitem__(self, name):
        return FakeCollection(self._connection[TEST_DB_NAME][name])


@pytest.fixture(autouse=True)
def clean_env(monkeypatch):
    """`.env` is loaded at import; a test must not inherit its database."""
    monkeypatch.delenv("APP_ENV", raising=False)
    monkeypatch.delenv("MONGO_URI", raising=False)
    monkeypatch.delenv("URL_PREFIX", raising=False)


@pytest.fixture
def fake_db(monkeypatch):
    """The models bound to an in-memory server, and `connect_db()` stubbed out.

    Every read and write in `core.db` calls `connect_db()` first, so replacing
    that one function is what fakes the database everywhere — and what
    `mongo_is_down` fails on purpose.
    """
    mongoengine.disconnect_all()
    connection = mongoengine.connect(
        db=TEST_DB_NAME,
        mongo_client_class=mongomock.MongoClient,
        uuidRepresentation="standard",
    )
    monkeypatch.setattr(core_db, "connect_db", lambda: None)
    monkeypatch.setattr(core_db, "_connected_to", TEST_DB_NAME, raising=False)

    for model in MODELS:
        model.objects.delete()

    yield FakeDb(connection)

    mongoengine.disconnect_all()


@pytest.fixture(autouse=True)
def clock(monkeypatch):
    """A clock that ticks a second per call.

    `updated_at` is what the plan list is ordered by, and two saves in one test
    land in the same millisecond on a real one — which mongoengine rounds to a
    tie, leaving the order undefined. This makes each save strictly later.

    Naive, like `core.db.now()` — see the reasoning there.
    """
    ticks = itertools.count()

    def tick():
        return datetime(1936, 1, 1) + timedelta(seconds=next(ticks))

    monkeypatch.setattr(core_db, "now", tick)
    return tick


@pytest.fixture
def mongo_is_down(monkeypatch):
    """Make every database call fail the way an unreachable server does."""

    def boom():
        raise ServerSelectionTimeoutError("no mongod here")

    monkeypatch.setattr(core_db, "connect_db", boom)


@pytest.fixture
def client(fake_db):
    """The Flask test client, on the fake database."""
    flask_app.app.config["TESTING"] = True
    return flask_app.app.test_client()
