"""What a page reads, and the one message an unreachable database gets.

Thin on purpose: the app has two collections and no joins. What lives here is the
wording of a database failure, so every route reports it the same way instead of
each inventing its own sentence — and a 500 never reaches the browser.
"""

from core.config import current_db_name


def db_error():
    """The sentence a route shows when MongoDB did not answer."""
    return (
        f"Could not reach the database ({current_db_name()}). "
        f"Is mongod running, and is MONGO_URI right?"
    )


def catalog_empty():
    """What to say when the database is up but holds no focus yet."""
    return "No focus stored yet. Seed the catalog with: python scripts/import_focuses.py"
