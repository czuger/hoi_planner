"""How the app is configured: its session key, and being served under a prefix."""

import pytest

import app as flask_app


def test_dev_gets_a_fixed_key():
    assert flask_app.secret_key() == "hoi4-planner-local"


def test_a_set_key_wins(monkeypatch):
    monkeypatch.setenv("FLASK_SECRET_KEY", "from-the-env")

    assert flask_app.secret_key() == "from-the-env"


def test_prod_refuses_to_run_on_the_dev_key(monkeypatch):
    """A guessable key in prod means anyone can forge a session."""
    monkeypatch.setenv("APP_ENV", "prod")
    monkeypatch.delenv("FLASK_SECRET_KEY", raising=False)

    with pytest.raises(EnvironmentError, match="FLASK_SECRET_KEY"):
        flask_app.secret_key()


def test_mounted_under_tells_the_app_where_it_lives():
    """The proxy already stripped the prefix, so only URL building needs telling —
    PATH_INFO is left alone."""
    seen = {}

    def wsgi_app(environ, start_response):
        seen.update(environ)
        return []

    wrapped = flask_app.mounted_under(wsgi_app, "hoi4")
    wrapped({"PATH_INFO": "/api/plans"}, None)

    assert seen["SCRIPT_NAME"] == "/hoi4"
    assert seen["PATH_INFO"] == "/api/plans"


@pytest.mark.parametrize("prefix", ["hoi4", "/hoi4", "/hoi4/"])
def test_the_prefix_is_normalized(prefix):
    seen = {}
    wrapped = flask_app.mounted_under(lambda environ, sr: seen.update(environ) or [], prefix)
    wrapped({}, None)

    assert seen["SCRIPT_NAME"] == "/hoi4"


def test_the_app_serves_at_the_root_without_a_prefix(client):
    assert client.get("/health").status_code in (200, 503)
