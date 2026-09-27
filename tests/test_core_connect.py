"""Opening the connection — the one part of `core.db` the fake database replaces.

Every other test stubs `connect_db()` out, so these drive the real one with
mongoengine's own calls faked: what matters is which database it settles on, not
that a server answered.
"""

import pytest

from core import db as core_db


@pytest.fixture
def fake_mongoengine(monkeypatch):
    """`connect()`/`disconnect()` recorded, and a database name to hand back."""
    calls = {"connect": [], "disconnect": 0, "db_name": "hoi4_planner_dev"}

    class FakeDatabase:
        name = property(lambda self: calls["db_name"])

    monkeypatch.setattr(core_db, "_connected_to", None)
    monkeypatch.setattr(core_db, "connect", lambda **kw: calls["connect"].append(kw))
    monkeypatch.setattr(
        core_db, "disconnect", lambda: calls.update(disconnect=calls["disconnect"] + 1)
    )
    monkeypatch.setattr(core_db, "mongoengine_db", lambda: FakeDatabase())
    return calls


def test_connects_to_the_database_the_env_picks(monkeypatch, fake_mongoengine):
    monkeypatch.setenv("MONGO_URI", "mongodb://localhost:27017")

    core_db.connect_db()

    assert fake_mongoengine["connect"][0]["db"] == "hoi4_planner_dev"
    assert fake_mongoengine["connect"][0]["host"] == "mongodb://localhost:27017"


def test_connecting_twice_opens_one_connection(monkeypatch, fake_mongoengine):
    monkeypatch.setenv("MONGO_URI", "mongodb://localhost:27017")

    core_db.connect_db()
    core_db.connect_db()

    assert len(fake_mongoengine["connect"]) == 1


def test_switching_env_reconnects(monkeypatch, fake_mongoengine):
    """A run that changes APP_ENV mid-flight must not keep writing to the old one."""
    monkeypatch.setenv("MONGO_URI", "mongodb://localhost:27017")
    core_db.connect_db()

    monkeypatch.setenv("APP_ENV", "prod")
    fake_mongoengine["db_name"] = "hoi4_planner_prod"
    core_db.connect_db()

    assert fake_mongoengine["disconnect"] == 1
    assert [call["db"] for call in fake_mongoengine["connect"]] == [
        "hoi4_planner_dev", "hoi4_planner_prod"
    ]


def test_a_missing_mongo_uri_says_what_to_do(monkeypatch, fake_mongoengine):
    monkeypatch.delenv("MONGO_URI", raising=False)

    with pytest.raises(EnvironmentError, match="MONGO_URI"):
        core_db.connect_db()


def test_a_database_named_in_the_uri_is_refused(monkeypatch, fake_mongoengine):
    """A database in MONGO_URI wins over the one APP_ENV asks for, which would
    quietly take a dev run onto another database."""
    monkeypatch.setenv("MONGO_URI", "mongodb://localhost:27017/somewhere")
    fake_mongoengine["db_name"] = "somewhere"

    with pytest.raises(EnvironmentError, match="Leave the database out of MONGO_URI"):
        core_db.connect_db()

    # And it lets go rather than leaving the wrong connection registered.
    assert fake_mongoengine["disconnect"] == 1
