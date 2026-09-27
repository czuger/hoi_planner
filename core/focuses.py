"""The focus catalog and the arithmetic of a plan. Pure: no database, no Flask.

Two jobs, and neither of them stores anything:

- **reading the catalog out of the saved wiki page** (`parse_wiki_page()`), which
  only `scripts/import_focuses.py` calls, and
- **turning an ordered list of focus ids into dates** (`schedule()`), which the
  web UI draws and the API returns.

Almost every focus takes the same 35 days, so a plan is mostly an order: a focus
completes when the ones before it are done plus its own term. The handful that
take half a term (14 days) or two (70) are marked by hand in the UI and travel
with the plan as `focus_days`, `{focus id: days}` — the wiki page carries no
duration to parse. That is the whole model, and it lives here so the views never
do date arithmetic.
"""

import html
import re
from datetime import date, timedelta
from html.parser import HTMLParser

# Where the icons stay. Nothing is downloaded: a focus keeps a link to the wiki's
# own copy, so the catalog is text and the page fetches the images.
WIKI_BASE = "https://hoi4.paradoxwikis.com"

# Germany starts the game here, and a national focus takes 35 days unless it is
# one of the few that take half a term or two of them. Those three are what the
# UI offers per focus; the plan's own `days` is what an unmarked focus costs.
DEFAULT_START = date(1936, 1, 1)
DEFAULT_DAYS_PER_FOCUS = 35
SHORT_DAYS = 14
LONG_DAYS = 70
FOCUS_DAY_CHOICES = (SHORT_DAYS, DEFAULT_DAYS_PER_FOCUS, LONG_DAYS)

# A focus row of the wiki page: `<tr id="Focus_Name">`, then a cell opening with a
# link to the icon's file page and the icon itself, then the name in bold.
_ROW = re.compile(r'^<tr id="([^"]+)">')
_ICON = re.compile(r'<a href="(/File:[^"]+)" class="image" title="([^"]*?) icon"><img[^>]*?src="([^"]+)"')
_NAME = re.compile(r'font-weight: bold; font-size:larger;">(.*?)<span id=')
_HEADLINE = re.compile(r'<h[23][^>]*>\s*<span class="mw-headline" id="([^"]+)"')
_TAGS = re.compile(r"<[^>]+>")

# A row is four cells: the focus itself, what it requires, and what it does. The
# requirements are the second, and nothing else on the page phrases them this way.
_PREREQUISITE_CELL = 2

# What that cell says, once the tags are out of the way. Only two forms of
# requirement exist on this page — there is no "all of the following" — and each
# is a `<li>` opening a `<ul>` of the focuses it names.
_REQUIRES_ANY = "requires one of the following"
_REQUIRES_ALL = "requires the following"
_EXCLUDES = "mutually exclusive with"


class _Lists(HTMLParser):
    """The `<ul>`/`<li>` nesting of one table cell, as a tree of items.

    A real parser rather than another regex because the requirement lists nest —
    "requires the following" can hold a "requires one of the following" — and a
    regex cannot tell which `</ul>` closes which list.

    An item is `{"text", "focuses", "children"}`: the words of the `<li>` itself,
    the anchors it links (`.../page#Focus_id`, which is how the wiki names a
    focus), and the `<li>`s of the `<ul>` inside it. Text and links land on the
    innermost open `<li>`, so a child's words never bleed into its parent's.
    """

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.root = _item()
        self._open = [self.root]

    @property
    def items(self):
        """The top-level `<li>`s of the cell."""
        return self.root["children"]

    def handle_starttag(self, tag, attrs):
        if tag == "li":
            item = _item()
            self._open[-1]["children"].append(item)
            self._open.append(item)
        elif tag == "a":
            # The wiki links a focus twice, as its icon and as its name.
            anchor = dict(attrs).get("href", "").partition("#")[2]
            if anchor and anchor not in self._open[-1]["focuses"]:
                self._open[-1]["focuses"].append(anchor)

    def handle_endtag(self, tag):
        if tag == "li" and len(self._open) > 1:
            self._open.pop()

    def handle_data(self, data):
        self._open[-1]["text"] += data


def _item(): return {"text": "", "focuses": [], "children": []}


def _phrase(text):
    """A `<li>`'s own words, flattened for comparing: lowercase, single-spaced."""
    return re.sub(r"\s+", " ", text).strip().lower()


def _groups(item):
    """One requirement list as groups — every group needs one of its focuses.

    Both forms collapse to the same shape. "Requires the following" is one group
    per entry (each must be taken); "requires one of the following" is a single
    group holding them all (any one will do). A list nesting the other kind is
    the recursion: the inner list is simply another group.
    """
    if _phrase(item["text"]).startswith(_REQUIRES_ANY):
        choice = [focus for child in item["children"] for focus in child["focuses"]]
        return [choice] if choice else []

    groups = []
    for child in item["children"]:
        if _phrase(child["text"]).startswith("requires"):
            groups.extend(_groups(child))
        elif child["focuses"]:
            groups.append(list(child["focuses"]))
    return groups


def _dependencies(block):
    """What one row's markup says the focus needs, and what it rules out.

    Anchors that are not focuses (a country, an ideology) and focuses from a
    branch this page does not carry come back too; `parse_wiki_page()` drops them
    once it knows the whole catalog.
    """
    cells = block.split("\n<td>")
    if len(cells) <= _PREREQUISITE_CELL:
        return [], []

    lists = _Lists()
    lists.feed(cells[_PREREQUISITE_CELL])

    requires, excludes = [], []
    for item in lists.items:
        phrase = _phrase(item["text"])
        if phrase.startswith(_REQUIRES_ANY) or phrase.startswith(_REQUIRES_ALL):
            requires.extend(_groups(item))
        elif phrase.startswith(_EXCLUDES):
            excludes.extend(focus for child in item["children"] for focus in child["focuses"])
    return requires, excludes


def _resolve(focuses):
    """Keep only the dependencies the catalog can name, and make exclusion mutual.

    A requirement list can point at an ideology, a country, or a focus from a
    branch of the tree this page does not carry (Legion Condor accepts two
    focuses the saved page never shows). None of those can be planned here, so
    they are dropped rather than left to block a focus forever — an emptied group
    is no requirement at all. Exclusion is symmetric by nature but the wiki only
    always writes one side of it, so the other is filled in here.
    """
    known = {focus["id"] for focus in focuses}
    by_id = {focus["id"]: focus for focus in focuses}

    for focus in focuses:
        groups = [[f for f in group if f in known] for group in focus["requires"]]
        focus["requires"] = [group for group in groups if group]
        focus["excludes"] = [f for f in focus["excludes"] if f in known and f != focus["id"]]

    for focus in focuses:
        for other in focus["excludes"]:
            if focus["id"] not in by_id[other]["excludes"]:
                by_id[other]["excludes"].append(focus["id"])

    return focuses


def parse_wiki_page(markup):
    """The focuses of a saved wiki page, in the order the page lists them.

    Ordered because that is how the branches read: the page groups focuses under
    their branch heading, so keeping document order keeps each branch's rows
    together and the UI can group them by simply walking the list.

    Only the full-size `/images/...` icons count. A focus also appears inside the
    prerequisite lists of the focuses that follow it, where the wiki links a
    `/thumb.php` resized copy instead — those rows are the same focus a second
    time, and skipping them is what keeps the catalog unique.

    Each focus also carries what it depends on: `requires`, groups of focus ids
    of which each group needs one taken, and `excludes`, the focuses it cannot
    share a plan with. See `_dependencies()`.
    """
    focuses, seen, section = [], set(), "Uncategorized"

    lines = markup.split("\n")
    # Where each row's markup ends — the next row's first line, or the page's.
    # The requirements are further down the row than the icon and the name, so a
    # row has to be read whole rather than through a window of its first lines.
    starts = [index for index, line in enumerate(lines) if _ROW.match(line)]
    ends = dict(zip(starts, starts[1:] + [len(lines)]))

    for index, line in enumerate(lines):
        headline = _HEADLINE.search(line)
        if headline:
            section = headline.group(1).replace("_", " ")

        row = _ROW.match(line)
        if not row:
            continue

        # The icon and the name sit on the line after the row's, but a row whose
        # first cell is one line long puts them on the row's own.
        head = "\n".join(lines[index : index + 3])
        icon = _ICON.search(head)
        if not icon or icon.group(3).startswith("/thumb.php"):
            continue

        name_match = _NAME.search(head)
        name = html.unescape(name_match.group(1) if name_match else icon.group(2))
        name = _TAGS.sub("", name).strip()

        key = (row.group(1), name)
        if key in seen:
            continue
        seen.add(key)

        requires, excludes = _dependencies("\n".join(lines[index : ends[index]]))
        focuses.append(
            {
                "id": row.group(1),
                "name": name,
                "icon": WIKI_BASE + icon.group(3),
                "icon_page": WIKI_BASE + icon.group(1),
                "section": section,
                "position": len(focuses),
                "requires": requires,
                "excludes": excludes,
            }
        )

    return _resolve(focuses)


def parse_start(value, default=DEFAULT_START):
    """A `YYYY-MM-DD` string as a date, falling back rather than raising.

    What a form or a query string hands over: the field is free text in the
    browser, and a plan with an unreadable start date should still draw.
    """
    try:
        year, month, day = (int(part) for part in str(value).split("-"))
        return date(year, month, day)
    except (AttributeError, TypeError, ValueError):
        return default


def unmet(focus, planned):
    """The prerequisite groups of `focus` that `planned` does not satisfy.

    `planned` is a set of focus ids. A group is met as soon as one of its focuses
    is in there — "requires the following" arrives here as one group per focus,
    "requires one of the following" as a single group of them all, so the same
    test covers both. Empty means every prerequisite is in place.
    """
    return [
        list(group)
        for group in (focus.get("requires") or [])
        if not any(focus_id in planned for focus_id in group)
    ]


def conflicts(focus, planned):
    """The planned focuses that rule `focus` out, mutually exclusive with it."""
    return [focus_id for focus_id in (focus.get("excludes") or []) if focus_id in planned]


def available(focus, planned):
    """Can `focus` be taken once `planned` is done? Nothing missing, nothing clashing."""
    return not unmet(focus, planned) and not conflicts(focus, planned)


def days_for(focus_id, focus_days=None, days=DEFAULT_DAYS_PER_FOCUS):
    """How long one focus takes: what the plan marked it, or the plan's default.

    `focus_days` holds only the focuses that differ (`{focus id: days}`), so an
    unmarked one — and one marked with anything that is not a whole number of
    days — costs `days`. Permissive rather than strict because a mark comes out
    of a stored plan, and one bad value should not stop the plan from drawing.
    """
    try:
        marked = int((focus_days or {})[focus_id])
    except (KeyError, TypeError, ValueError):
        return days
    return marked if marked > 0 else days


def completion_date(days_elapsed, start=DEFAULT_START):
    """The date reached after `days_elapsed` days of work.

    Work begins *on* the start date rather than after it, so the start date is
    not itself a day of work: 35 days from 1 Jan 1936 is 5 Feb 1936.
    """
    return start + timedelta(days=days_elapsed)


def schedule(focus_ids, catalog, start=DEFAULT_START, days=DEFAULT_DAYS_PER_FOCUS, focus_days=None):
    """One plan as rows to draw: each focus, its position, its term, and when it
    completes.

    `catalog` is `{focus id: focus}`. An id the catalog does not know is dropped
    rather than drawn blank — a plan saved before a re-import should lose the
    focus that went away, not break the page — and the focuses after it move up,
    so the dates close the gap rather than keep its slot.

    Each row also says whether the order works: `needs` is the prerequisite
    groups nothing before it satisfies and `clashes` the focuses before it that
    rule it out, both empty for a row that can actually be taken there.
    """
    rows, elapsed, taken = [], 0, set()
    for focus_id in focus_ids:
        focus = catalog.get(focus_id)
        if focus is None:
            continue
        term = days_for(focus_id, focus_days, days)
        elapsed += term
        rows.append(
            {
                **focus,
                "position": len(rows) + 1,
                "days": term,
                "completes_on": completion_date(elapsed, start).isoformat(),
                # Against what comes *before* it: a focus can only be worked on
                # once its prerequisites are done, so an order that puts one too
                # early is a plan that cannot be played, not merely an odd one.
                "needs": unmet(focus, taken),
                "clashes": conflicts(focus, taken),
            }
        )
        taken.add(focus_id)
    return rows


def total_days(focus_ids, catalog, days=DEFAULT_DAYS_PER_FOCUS, focus_days=None):
    """How long the whole plan takes, ignoring ids the catalog has lost."""
    return sum(
        days_for(focus_id, focus_days, days) for focus_id in focus_ids if focus_id in catalog
    )


def by_id(focuses):
    """The catalog keyed by focus id, which is how `schedule()` wants it."""
    return {focus["id"]: focus for focus in focuses}
