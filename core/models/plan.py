"""A saved plan: an order of focuses, and what to date it from.

The plan is the order plus the few focuses that cost something other than the
usual term: `focus_days` holds only those, because the wiki page has no duration
to import and a focus is 35 days unless someone says otherwise. Everything the UI
draws (each focus's completion date, the total) is derived from `focus_ids`,
`focus_days`, `start_date` and `days_per_focus` by `core.focuses.schedule()`, so
nothing derived is ever stored.

Keyed by a slug of the name, so a plan can be fetched, replaced and deleted
through its own URL and saving the same name twice updates one document instead
of growing a second.
"""

from mongoengine import DateTimeField, Document, IntField, ListField, MapField, StringField

from .base import DictDocument
from ..focuses import DEFAULT_DAYS_PER_FOCUS


class Plan(Document, DictDocument):
    """One named ordering of focuses."""

    meta = {
        "collection": "plans",
        "indexes": ["updated_at"],
        "strict": False,
    }

    # A slug of the name — see core.db.slugify().
    id = StringField(primary_key=True)

    # What the person typed, kept as typed for display.
    name = StringField(required=True)

    # The plan itself: focus ids, in the order they are to be taken.
    focus_ids = ListField(StringField())

    # `YYYY-MM-DD`, stored as text because it is a game date, not a real one —
    # 1936 is inside what `datetime` handles, but nothing here needs it to be.
    start_date = StringField(default="1936-01-01")

    # What an unmarked focus costs.
    days_per_focus = IntField(default=DEFAULT_DAYS_PER_FOCUS)

    # `{focus id: days}`, and only for the focuses that differ from that — 14 for
    # a half term, 70 for two. A map rather than a list beside `focus_ids` so
    # reordering the plan cannot put a term against the wrong focus.
    focus_days = MapField(IntField())

    created_at = DateTimeField()

    # What the plan list is ordered by, most recently touched first.
    updated_at = DateTimeField()

    def __str__(self):
        return f"{self.name} ({len(self.focus_ids or [])} focuses)"
