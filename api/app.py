"""The Flask app: its configuration, and the blueprints that hold every route.

Nothing here answers a request. The page lives in `views/main.py` and the JSON the
browser saves through in `views/plans.py`, so this file only ever grows when the
app gains a setting or a blueprint.
"""

import os
import sys
from pathlib import Path

# The repo root, so `core` imports the same way here as from the scripts. gunicorn
# runs with --chdir api/, which puts this directory on the path but not its parent.
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from flask import Flask

from core.config import current_env
from views import BLUEPRINTS


def secret_key():
    """Signs the session cookie — a fixed dev value is fine on localhost, but a
    guessable key in prod means anyone can forge a session."""
    key = os.environ.get("FLASK_SECRET_KEY")
    if key:
        return key
    if current_env() == "prod":
        raise EnvironmentError(
            "FLASK_SECRET_KEY must be set when APP_ENV=prod. Generate one with: "
            'python -c "import secrets; print(secrets.token_hex(32))"'
        )
    return "hoi4-planner-local"


def mounted_under(wsgi_app, prefix):
    """Tell the app it is served under `prefix` (e.g. `/hoi4`), in production.

    The proxy strips the prefix before forwarding, so routing already matches —
    but `url_for()`, redirects and static URLs would come out at the site root,
    outside the proxied location. SCRIPT_NAME is the WSGI way to say "this is the
    part of the URL that got you here", and Flask prepends it when building URLs.
    """
    prefix = "/" + prefix.strip("/")

    def wrapper(environ, start_response):
        environ["SCRIPT_NAME"] = prefix
        return wsgi_app(environ, start_response)

    return wrapper


def create_app():
    """The app, configured and with every route registered."""
    flask_app = Flask(__name__)
    flask_app.secret_key = secret_key()

    # Unset in dev and in tests, so the app stays at the root there.
    url_prefix = os.environ.get("URL_PREFIX", "").strip().strip("/")
    if url_prefix:
        flask_app.wsgi_app = mounted_under(flask_app.wsgi_app, url_prefix)

    for blueprint in BLUEPRINTS:
        flask_app.register_blueprint(blueprint)

    return flask_app


# What gunicorn serves (`app:app`) and what the tests drive.
app = create_app()


if __name__ == "__main__":
    app.run(debug=True, port=5000)
