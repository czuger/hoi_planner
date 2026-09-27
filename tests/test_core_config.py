"""Which database a run uses, and what it takes to be told."""

import pytest

from core.config import current_db_name, current_env, load_env


def test_the_default_env_is_dev():
    assert current_env() == "dev"
    assert current_db_name() == "hoi4_planner_dev"


def test_prod_picks_its_own_database(monkeypatch):
    """The whole point of APP_ENV: a dev run must never write to prod data."""
    monkeypatch.setenv("APP_ENV", "prod")

    assert current_db_name() == "hoi4_planner_prod"


@pytest.mark.parametrize("value", ["PROD", " dev "])
def test_the_env_is_read_loosely(monkeypatch, value):
    monkeypatch.setenv("APP_ENV", value)

    assert current_env() == value.strip().lower()


def test_an_unknown_env_is_refused(monkeypatch):
    monkeypatch.setenv("APP_ENV", "staging")

    with pytest.raises(EnvironmentError, match="APP_ENV"):
        current_env()


def test_load_env_returns_what_is_set(monkeypatch):
    monkeypatch.setenv("MONGO_URI", "mongodb://localhost:27017")

    assert load_env(["MONGO_URI"]) == {"MONGO_URI": "mongodb://localhost:27017"}


def test_load_env_names_everything_missing(monkeypatch):
    monkeypatch.delenv("MONGO_URI", raising=False)

    with pytest.raises(EnvironmentError, match="MONGO_URI"):
        load_env(["MONGO_URI"])
