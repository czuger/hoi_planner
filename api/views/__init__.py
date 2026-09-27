"""One module per page of the UI, one blueprint each.

`app.py` registers whatever BLUEPRINTS holds and knows nothing else about the
routes; the modules import from `core` and `queries`, never from `app`, so there
is no import cycle to work around.

Endpoints are therefore named `<blueprint>.<function>` — `main.index`,
`plans.save` — which is what the template's `url_for()` calls use.
"""

from . import main, plans

BLUEPRINTS = (
    main.bp,
    # Not a page: JSON, which the planner posts to as you edit.
    plans.bp,
)
