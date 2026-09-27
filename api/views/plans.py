"""Saving, loading and deleting a plan — JSON, not pages.

The planner is one screen that never reloads: dragging a focus into place is a
client-side edit, and only the result is worth storing. So these routes speak JSON
and the browser calls them, rather than posting forms that would redraw the page
and lose the scroll position mid-plan.

Two ways in, over the same fields: `/api/plans` is the named plan someone chose to
keep, `/api/draft` is the panel as it stands, written on every edit so a reload
picks the plan back up where it was left. The draft has no name to slug and no
list to appear in — see `core.db.DRAFT_PLAN_ID`.

A plan also travels as a file, `/api/plans/export` and `/api/plans/import`: the
same fields again, in the envelope `core.plan_file` describes. Export answers with
a download and import answers with a plan to draw — it stores nothing, because an
imported plan lands in the panel where it can be looked at and then saved under a
name like any other.

Every route answers with a body the caller can show — `{"error": "..."}` and a
status — instead of raising, so a dead database reads as a message next to the
save button rather than a 500 in the console.
"""

import json

from flask import Blueprint, Response, jsonify, request

from core.db import (
    DatabaseError,
    clear_draft,
    delete_plan,
    load_focuses,
    load_plan,
    load_plans,
    save_draft,
    save_plan,
    slugify,
)
from core.focuses import DEFAULT_DAYS_PER_FOCUS, by_id
from core.plan_file import from_file, to_file
from queries import db_error

bp = Blueprint("plans", __name__)

# What an exported plan is called on the way down. Two extensions on purpose: the
# file is JSON and opens as JSON, and the middle one says whose.
FILE_SUFFIX = ".hoi4plan.json"


def plan_fields(payload):
    """The plan out of a posted body, or the sentence saying what is wrong with it.

    `(fields, None)` or `(None, message)`. Shared by the save and the draft: the
    two differ in what they are keyed by, not in what a plan is made of — so a
    body the draft accepts is one that could be saved under a name later.
    """
    focus_ids = payload.get("focus_ids") or []
    if not isinstance(focus_ids, list):
        return None, "focus_ids must be a list."
    # Ids, not just anything in a list: they are stored as strings and an import
    # reads them out of a file nobody here wrote.
    if not all(isinstance(focus_id, str) for focus_id in focus_ids):
        return None, "focus_ids must be focus ids."

    # `or` would turn a posted 0 into the default and slip past the check below.
    given = payload.get("days_per_focus")
    try:
        days = DEFAULT_DAYS_PER_FOCUS if given in (None, "") else int(given)
    except (TypeError, ValueError):
        return None, "days_per_focus must be a whole number."
    if days < 1:
        return None, "days_per_focus must be at least 1."

    # `{focus id: days}` for the focuses that cost something other than `days`.
    # Only shape and sign are checked, not the three values the UI offers: what
    # is worth refusing is a term no schedule could be drawn from.
    marks = payload.get("focus_days") or {}
    if not isinstance(marks, dict):
        return None, "focus_days must be an object."
    try:
        focus_days = {str(focus_id): int(term) for focus_id, term in marks.items()}
    except (TypeError, ValueError):
        return None, "focus_days must be whole numbers."
    if any(term < 1 for term in focus_days.values()):
        return None, "focus_days must be at least 1."

    return (
        {
            "focus_ids": focus_ids,
            "start_date": payload.get("start_date"),
            "days_per_focus": days,
            "focus_days": focus_days,
        },
        None,
    )


@bp.get("/api/plans")
def index():
    """Every saved plan, most recently touched first."""
    try:
        return jsonify(plans=load_plans())
    except DatabaseError:
        return jsonify(error=db_error()), 503


@bp.get("/api/plans/<plan_id>")
def show(plan_id):
    """One plan, so the page can load it into the panel."""
    try:
        plan = load_plan(plan_id)
    except DatabaseError:
        return jsonify(error=db_error()), 503

    if plan is None:
        return jsonify(error="No such plan."), 404
    return jsonify(plan=plan)


@bp.post("/api/plans")
def save():
    """Create the plan of that name, or overwrite the one already called that.

    The name is the identity (`slugify()` makes the id), so this is one route
    rather than a create and an update: saving twice under one name revises it.
    """
    payload = request.get_json(silent=True) or {}
    name = str(payload.get("name", "")).strip()
    if not name:
        return jsonify(error="A plan needs a name."), 400

    fields, problem = plan_fields(payload)
    if problem:
        return jsonify(error=problem), 400

    try:
        plan = save_plan(name, **fields)
    except DatabaseError:
        return jsonify(error=db_error()), 503

    # A name of nothing but punctuation slugs to an empty id, which no URL could
    # address — see core.db.save_plan().
    if plan is None:
        return jsonify(error="That name has no letters or digits in it."), 400
    return jsonify(plan=plan), 201


# --- a plan as a file ---------------------------------------------------------


def _catalog():
    """The catalog keyed by id — what `to_file()` writes its names and dates from,
    and what says which of an imported file's ids this catalog still knows."""
    return by_id(load_focuses())


def _download(plan, catalog):
    """One plan as the file the browser saves.

    `indent=2` because the point of a file is that it can be read and diffed;
    `ensure_ascii=False` so a plan named in French is still named in French once
    it is on disk. The name is slugged the way a plan id is — it has to survive a
    filename as well as a URL — and a plan with nothing to slug still downloads,
    under `plan`.
    """
    body = json.dumps(to_file(plan, catalog), indent=2, ensure_ascii=False) + "\n"
    filename = (slugify(plan.get("name") or "") or "plan") + FILE_SUFFIX
    return Response(
        body,
        mimetype="application/json",
        headers={"Content-Disposition": f'attachment; filename="{filename}"'},
    )


@bp.post("/api/plans/export")
def export_plan():
    """The plan on screen, as a file — the Export button.

    A POST because what is exported is the panel, which is not the server's to
    read: the draft is written on a timer, so asking for the stored one could
    hand back the plan as it was an edit ago. The body is the same one a save
    posts, and it is checked the same way.
    """
    payload = request.get_json(silent=True) or {}
    fields, problem = plan_fields(payload)
    if problem:
        return jsonify(error=problem), 400

    try:
        catalog = _catalog()
    except DatabaseError:
        return jsonify(error=db_error()), 503

    return _download({"name": payload.get("name"), **fields}, catalog)


@bp.get("/api/plans/<plan_id>/export")
def export_saved(plan_id):
    """A saved plan as a file, by id — the same file, without a browser.

        curl -OJ http://localhost:5000/api/plans/barbarossa-rush/export
    """
    try:
        plan = load_plan(plan_id)
        catalog = _catalog() if plan else {}
    except DatabaseError:
        return jsonify(error=db_error()), 503

    if plan is None:
        return jsonify(error="No such plan."), 404
    return _download(plan, catalog)


@bp.post("/api/plans/import")
def import_plan():
    """A file's contents, checked and handed back for the panel to draw.

    It stores nothing. An import is a plan arriving, not a plan kept: it lands in
    the panel, where it can be looked at, reordered, renamed and saved like any
    other — and the browser writes it to the draft on the way, so it survives a
    reload before it is ever named.

    `unknown` is the ids this catalog does not have. They are handed back with the
    rest rather than dropped: a plan exported from a fuller catalog should say
    what is missing, not quietly come up short — `schedule()` already leaves such
    an id out of the dates.
    """
    plan, problem = from_file(request.get_json(silent=True))
    if problem:
        return jsonify(error=problem), 400

    fields, problem = plan_fields(plan)
    if problem:
        return jsonify(error=problem), 400

    try:
        catalog = _catalog()
    except DatabaseError:
        return jsonify(error=db_error()), 503

    return jsonify(
        plan={"name": str(plan.get("name") or "").strip(), **fields},
        unknown=[focus_id for focus_id in fields["focus_ids"] if focus_id not in catalog],
    )


# --- the draft ----------------------------------------------------------------


@bp.put("/api/draft")
def draft():
    """Keep the plan on screen, whatever state it is in.

    The browser calls this after every edit, so it answers about the body and
    nothing else: no name is required, because the draft is what is being built
    rather than something someone has decided to keep.
    """
    payload = request.get_json(silent=True) or {}
    fields, problem = plan_fields(payload)
    if problem:
        return jsonify(error=problem), 400

    try:
        return jsonify(plan=save_draft(payload.get("name"), **fields))
    except DatabaseError:
        return jsonify(error=db_error()), 503


@bp.delete("/api/draft")
def discard():
    """Throw the draft away — the Clear button, with nothing left to pick up.

    Clearing a panel that was never saved is not an error, so this answers the
    same either way rather than 404ing on an empty draft.
    """
    try:
        return jsonify(cleared=clear_draft())
    except DatabaseError:
        return jsonify(error=db_error()), 503


@bp.delete("/api/plans/<plan_id>")
def destroy(plan_id):
    """Throw one plan away. There is no undo."""
    try:
        found = delete_plan(plan_id)
    except DatabaseError:
        return jsonify(error=db_error()), 503

    if not found:
        return jsonify(error="No such plan."), 404
    return jsonify(deleted=plan_id)
