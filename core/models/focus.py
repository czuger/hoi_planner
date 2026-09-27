"""One national focus of the German tree, as the wiki lists it.

Keyed by the wiki's own anchor (`Fortify_the_Vaterland`), which is stable across
re-imports and is what a plan stores — so a plan survives a re-import as long as
the focus is still on the page.

**No icon is ever downloaded.** `icon` is a link to the wiki's copy and the
browser fetches it; `icon_page` is the file's page there, for a human who wants
the original.
"""

from mongoengine import Document, IntField, ListField, StringField

from .base import DictDocument


class Focus(Document, DictDocument):
    """A focus: what it is called, its icon, and where it sits on the page."""

    meta = {
        "collection": "focuses",
        "indexes": ["position", "section"],
        "strict": False,
    }

    # The wiki anchor, e.g. "Fortify_the_Vaterland".
    id = StringField(primary_key=True)

    name = StringField(required=True)

    # Links to the wiki, never a local file. See the module docstring.
    icon = StringField(required=True)
    icon_page = StringField()

    # The branch heading the focus sits under, e.g. "Anschluss Sub-branch".
    section = StringField()

    # Document order on the page, which is how the UI lists them: it keeps a
    # branch's focuses together and roughly in the order they can be taken.
    position = IntField()

    # What the page's "Requires …" lists say, as groups of focus ids: every group
    # needs one of its focuses taken first. A group of one is a plain "requires
    # this"; a longer one is a "requires one of these". See
    # `core.focuses.unmet()`.
    requires = ListField(ListField(StringField()))

    # The focuses this one cannot share a plan with — the page's "Mutually
    # exclusive with". Symmetric: both sides carry the other.
    excludes = ListField(StringField())

    def __str__(self):
        return f"{self.name} ({self.section})"
