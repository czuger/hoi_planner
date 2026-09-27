"""Turning a document into a plain dict, and back.

The app is written against dicts — `core.focuses` reads them, the template renders
them, the API returns them as JSON — so the models are a boundary, not a type that
travels. Every model mixes this in, and `core.db` converts on the way in and on
the way out.

Two rules the whole app leans on:

- **The dict is keyed as the document is stored.** The primary key is the
  exception: a model whose id is its own (a focus id off the wiki, a plan slug)
  hands it over as `id`, which is what the app calls it and what Mongo keeps in
  `_id`; a generated ObjectId is Mongo's own bookkeeping and never leaves here.
- **`to_dict()` always returns every declared field**, `None` where nothing is
  stored, so a caller can read a key without knowing whether it was ever written;
  **`from_dict()` ignores anything undeclared**, which is what lets a parsed wiki
  row be handed over as it came.
"""

from mongoengine import ObjectIdField
from mongoengine.base import BaseDocument


def plain(value):
    """A document's value as plain Python: no BaseList, BaseDict or embedded doc."""
    if isinstance(value, BaseDocument):
        return value.to_dict()
    if isinstance(value, (list, tuple)):
        return [plain(item) for item in value]
    if isinstance(value, dict):
        return {key: plain(item) for key, item in value.items()}
    return value


class DictDocument:
    """A document that goes in and comes out as a dict keyed like the stored one."""

    @classmethod
    def stored_fields(cls):
        """`{dict key: attribute name}`, in declaration order.

        A declared primary key is the document's own id: it is kept, under its
        attribute name (`id`) rather than under `_id`. The ObjectId one mongoengine
        adds when a model declares no key of its own is Mongo's bookkeeping, and
        stays out of the dict.
        """
        primary_key = cls._meta.get("id_field")
        fields = {}
        for name in cls._fields_ordered:
            field = cls._fields[name]
            if name != primary_key:
                fields[field.db_field] = name
            elif not isinstance(field, ObjectIdField):
                fields[name] = name
        return fields

    @classmethod
    def collection_name(cls):
        """The collection this model is stored in — for a message about a run."""
        return cls._get_collection_name()

    @classmethod
    def from_dict(cls, data):
        """Build a document from a stored-shaped dict, ignoring undeclared keys."""
        fields = cls.stored_fields()
        return cls(**{fields[key]: value for key, value in data.items() if key in fields})

    def to_dict(self):
        """Every declared field, `None` where unset, keyed as it is stored."""
        return {key: plain(getattr(self, name)) for key, name in self.stored_fields().items()}
