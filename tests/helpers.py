"""The few documents the tests build, as plain functions.

Functions rather than constants because a test usually wants two or three with
one field changed, and a shared dict would carry a mutation into the next test.
"""

FOCUS_ID = "Fortify_the_Vaterland"
WIKI = "https://hoi4.paradoxwikis.com"


def focus(focus_id=FOCUS_ID, name="Fortify the Vaterland", section="Rhineland Branch",
          position=0, requires=(), excludes=()):
    """One catalog row, shaped as `core.focuses.parse_wiki_page()` hands it over."""
    return {
        "id": focus_id,
        "name": name,
        "icon": f"{WIKI}/images/f/f5/Goal_generic_construct_military.png",
        "icon_page": f"{WIKI}/File:Goal_generic_construct_military.png",
        "section": section,
        "position": position,
        "requires": [list(group) for group in requires],
        "excludes": list(excludes),
    }


def catalog():
    """Three focuses across two branches — enough to test grouping and filtering."""
    return [
        focus(position=0),
        focus("Rhineland", "Remilitarize the Rhineland", position=1),
        focus("Anschluss", "Anschluss", section="Anschluss Sub-branch", position=2),
    ]


# A row of the saved wiki page, in the markup `parse_wiki_page()` reads.
def wiki_row(focus_id=FOCUS_ID, name="Fortify the Vaterland",
             image="/images/f/f5/Goal_generic_construct_military.png", cells=""):
    return (
        f'<tr id="{focus_id}">\n'
        f'<td><div style="display:flex; align-items:center;">'
        f'<a href="/File:Goal_generic_construct_military.png" class="image" '
        f'title="{name} icon"><img alt="{name} icon" src="{image}" width="86"></a>\n'
        f'<div style="margin-left:5px;">'
        f'<div style="font-weight: bold; font-size:larger;">{name}'
        f'<span id="{focus_id}"></span></div></div></div>\n'
        f'{cells}'
    )


def wiki_heading(section="Rhineland Branch"):
    anchor = section.replace(" ", "_")
    return f'<h3><span class="mw-headline" id="{anchor}">{section}</span></h3>\n'


# --- the prerequisite cell of a row -------------------------------------------
#
# The wiki writes a row as three cells — the focus, what it requires, what it
# does — and links a focus twice inside a requirement list, as its icon and as
# its name. `wiki_row()` alone builds only the first cell, which is a row with no
# prerequisites at all; pass `cells=requirements(...)` for the rest.


def wiki_link(focus_id, name=None):
    """A focus as a requirement list links it: the icon, then the name."""
    name = name or focus_id.replace("_", " ")
    page = f"/German_national_focus_tree#{focus_id}"
    return (
        f'<a href="{page}" title="{name}">'
        f'<img alt="{name}" src="/thumb.php?f=Focus.png&amp;width=36"></a>'
        f'<a href="{page}" title="German national focus tree">{name}</a>'
    )


def requires(*entries, any_of=False):
    """A `Requires …` list, as the `<li>` that opens it.

    An entry is a focus id, a raw `<li>` (a nested list, or something that is not
    a focus at all), or a `(id, name)` pair for a link the catalog does not hold.
    """
    label = "Requires <i>one</i> of the following:" if any_of else "Requires the following:"
    items = "".join(
        entry if str(entry).startswith("<li>")
        else f"<li>{wiki_link(*entry) if isinstance(entry, tuple) else wiki_link(entry)}</li>"
        for entry in entries
    )
    return f"<li>{label}\n<ul>{items}</ul></li>"


def excludes(*focus_ids):
    """A `Mutually exclusive with` list, as the `<li>` that opens it."""
    items = "".join(f"<li>{wiki_link(focus_id)}</li>" for focus_id in focus_ids)
    return f"<li>Mutually exclusive with:\n<ul>{items}</ul></li>"


def requirements(*items, effects=""):
    """The two cells that follow a row's first one: what it needs, what it does.

    `effects` is the third cell, which a real row fills with wording that reads
    like a requirement ("One of the following must be true") without being one.
    """
    return (
        "</td>\n<td>Germany:\n<ul>" + "".join(items) + "</ul>\n"
        "</td>\n<td>Germany:\n" + effects + "\n</td>\n"
    )
