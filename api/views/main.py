"""The planner itself: the catalog on the left, the plan being built on the right.

One page, and it is the whole UI. The catalog is rendered server-side (215 rows,
grouped by branch); the plan is edited in the browser — dragging is a client-side
affair — and posted to `views/plans.py` as JSON.
"""

from flask import Blueprint, render_template

from core.config import current_db_name
from core.focuses import DEFAULT_DAYS_PER_FOCUS, DEFAULT_START, LONG_DAYS, SHORT_DAYS
from core.db import DatabaseError, load_draft, load_focuses, load_plans
from queries import catalog_empty, db_error

bp = Blueprint("main", __name__)


@bp.get("/")
def index():
    """The catalog, the saved plans, and the plan that was being built.

    The draft comes down with the page rather than being fetched after it: it is
    what the panel draws on the first render, and a second round trip would show
    an empty plan for a moment before filling it in.
    """
    error, focuses, plans, draft = None, [], [], None
    try:
        focuses = load_focuses()
        plans = load_plans()
        draft = load_draft()
    except DatabaseError:
        error = db_error()

    if not error and not focuses:
        error = catalog_empty()

    return render_template(
        "planner.html",
        focuses=focuses,
        plans=plans,
        draft=draft,
        error=error,
        default_start=DEFAULT_START.isoformat(),
        default_days=DEFAULT_DAYS_PER_FOCUS,
        short_days=SHORT_DAYS,
        long_days=LONG_DAYS,
        db_name=current_db_name(),
    )


@bp.get("/health")
def health():
    """Is the app up, and can it read the database? For the deploy check."""
    try:
        count = len(load_focuses())
    except DatabaseError:
        return {"status": "degraded", "database": current_db_name(), "error": db_error()}, 503
    return {"status": "ok", "database": current_db_name(), "focuses": count}
