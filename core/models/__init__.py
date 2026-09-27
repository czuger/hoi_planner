"""One mongoengine document per collection — the shape of what is stored.

The models describe the collections and nothing else: no module here opens a
connection or runs a query. `core.db` holds the connection and every read and
write, and hands dicts back to the rest of the app, so only this package and
`core.db` ever see a document object.

    focuses -> Focus (the German tree, imported from the saved wiki page)
    plans   -> Plan (one named ordering of focus ids)
"""

from .focus import Focus
from .plan import Plan

__all__ = ["Focus", "Plan"]
