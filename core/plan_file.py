"""A plan as a file: what an export writes, and what an import is allowed to read.

Pure, like `core.focuses`: this module knows the shape of the exchange format and
nothing about where a plan is stored or how the file got here.

What travels is the plan itself and nothing else — the order, the start date, the
default term and the handful of focuses that differ — inside an envelope naming
what the file is (`format`, `version`), so that some other JSON dropped on the
import button is refused with a sentence instead of being half-read into the
panel.

Everything else in the file is a **readable echo**: `focuses` spells the plan out
in names and dates for whoever opens it, and an import ignores it. Nothing is
stored from it and nothing is derived from it — the order is the plan, and
`schedule()` makes those dates again from it.

The file is deterministic: no timestamp, no id, nothing about the database it came
out of. Exporting the same plan twice gives the same bytes, so two exports diff
cleanly and a plan can be kept in git next to the rest.
"""

from .focuses import DEFAULT_DAYS_PER_FOCUS, parse_start, schedule

# What the envelope says. The format name is checked on import — it is the only
# thing that tells one of our files from any other JSON object — and the version
# is what a future change to these fields would raise, so an older planner can
# refuse a file it would misread rather than guess at it.
FORMAT = "hoi4-planner-plan"
VERSION = 1

# The plan, and all of it: the four fields a plan is made of, plus the name it was
# exported under. Everything the UI draws is derived from these — see
# `core.models.plan`, which stores exactly the same and nothing more.
PLAN_FIELDS = ("name", "focus_ids", "start_date", "days_per_focus", "focus_days")

# What each focus of the echo says. `id` is what an import would read if it read
# the echo at all; the rest is for the person looking at the file.
ECHO_FIELDS = ("position", "id", "name", "days", "completes_on")


def to_file(plan, catalog=None):
    """One plan as the dict an export writes out.

    `catalog` is `{focus id: focus}` — what `core.focuses.by_id()` makes — and is
    only there for the readable echo: without it the file still holds the whole
    plan, just no names or dates beside it.
    """
    fields = {key: plan.get(key) for key in PLAN_FIELDS}
    days = fields["days_per_focus"] or DEFAULT_DAYS_PER_FOCUS
    rows = schedule(
        fields["focus_ids"] or [],
        catalog or {},
        start=parse_start(fields["start_date"]),
        days=days,
        focus_days=fields["focus_days"] or {},
    )
    return {
        "format": FORMAT,
        "version": VERSION,
        **fields,
        "focuses": [{key: row[key] for key in ECHO_FIELDS} for row in rows],
    }


def from_file(data):
    """The plan inside an exported file, or the sentence saying there is none.

    `(plan, None)` or `(None, message)`, like every other check in this app. The
    plan handed back is the file's values as they came: this is the envelope's
    check, and what a plan may hold — a list of ids, whole numbers of days — is
    the same check a save goes through (`api.views.plans.plan_fields()`), so
    there is one definition of that and not a second one for files.
    """
    if not isinstance(data, dict):
        return None, "A plan file holds a JSON object."

    if data.get("format") != FORMAT:
        return None, f"That file is not a {FORMAT} export."

    version = data.get("version")
    if not isinstance(version, int) or isinstance(version, bool) or version < 1:
        return None, "That file does not say which version it is."
    if version > VERSION:
        return None, (
            f"That file is version {version}, and this planner reads up to "
            f"version {VERSION}. Export it again from an older planner, or update "
            f"this one."
        )

    return {key: data.get(key) for key in PLAN_FIELDS}, None
