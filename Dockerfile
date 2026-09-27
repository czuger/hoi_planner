# Image for the Flask UI (the import script is copied in too, so `make seed` can
# run it with `docker exec`, but the container's job is to serve the web app).
#
# It carries code only: `.env` is mounted from the server's config/ directory at
# run time — see deploy/remote.sh. Nothing here ever bakes in a credential, which
# is why .dockerignore drops that file.
#
# **Nothing in here reaches the network at run time either**: the catalog comes
# from the committed JSON, and a focus's icon is a link to the wiki's copy that
# the browser fetches.
#
# No `--platform` is pinned here on purpose: deploy.sh passes the *server's*
# architecture on the build command line (TARGET_PLATFORM, default linux/amd64),
# so building on an Apple Silicon Mac still yields an x86_64 image. Building this
# file by hand on a Mac gives you an arm64 image the Linux server cannot run.
FROM python:3.13-slim

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1

WORKDIR /app

# Dependencies first: this layer is rebuilt only when what it copies changes, so
# editing the app is a fast rebuild. `pip install .` also needs what
# [tool.setuptools] declares — the `core` package — hence copying it here rather
# than with the rest.
COPY pyproject.toml ./
COPY core/ ./core/
RUN pip install --no-cache-dir ".[prod]"

# The application itself. It runs from /app (not from site-packages), so the copies
# below are what actually serve requests.
COPY api/ ./api/
COPY scripts/ ./scripts/

# The parsed catalog, and it alone: `scripts/import_focuses.py --from-json` seeds
# the database from it. The 2 MB wiki page it was parsed out of stays on the
# laptop (.dockerignore).
COPY data/focuses.json ./data/focuses.json

# Non-root, and nothing under /app needs to be writable: the plans and the catalog
# live in the server's MongoDB, and the logs go to stdout for `docker logs`.
RUN useradd --system --uid 10001 --shell /usr/sbin/nologin appuser
USER appuser

# No EXPOSE and no published port anywhere: the container runs on its own docker
# network (deploy/remote.sh) and is reached by container name over it. Nothing is
# bound on the host.

# No curl in the slim image — urllib does the same job with what is already here.
# /health also reads the database, so this goes unhealthy if mongod does.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD python -c "import urllib.request; urllib.request.urlopen('http://127.0.0.1:8000/health', timeout=3)"

# --chdir puts api/ on sys.path, so `app:app` resolves and so do the app's own
# modules (queries, views); app.py itself prepends /app, which is what makes
# `import core` work from there.
CMD ["gunicorn", "--chdir", "/app/api", "--bind", "0.0.0.0:8000", \
     "--workers", "2", "--timeout", "60", \
     "--access-logfile", "-", "--error-logfile", "-", "app:app"]
