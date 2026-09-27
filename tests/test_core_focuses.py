"""The parser and the date arithmetic — the two pure halves of the app."""

from datetime import date

import pytest

from core.focuses import (
    DEFAULT_DAYS_PER_FOCUS,
    DEFAULT_START,
    LONG_DAYS,
    SHORT_DAYS,
    available,
    by_id,
    completion_date,
    conflicts,
    days_for,
    parse_start,
    parse_wiki_page,
    schedule,
    total_days,
    unmet,
)
from helpers import (
    catalog,
    excludes,
    focus,
    requirements,
    requires,
    wiki_heading,
    wiki_row,
)


def test_parses_a_row_into_a_focus():
    [focus] = parse_wiki_page(wiki_heading() + wiki_row())

    assert focus["id"] == "Fortify_the_Vaterland"
    assert focus["name"] == "Fortify the Vaterland"
    assert focus["section"] == "Rhineland Branch"
    assert focus["position"] == 0
    # The icon is a link to the wiki, never a downloaded file.
    assert focus["icon"].startswith("https://hoi4.paradoxwikis.com/images/")
    assert focus["icon_page"].startswith("https://hoi4.paradoxwikis.com/File:")


def test_focuses_keep_the_page_order_and_their_branch():
    markup = (
        wiki_heading("Rhineland Branch")
        + wiki_row("A", "Alpha")
        + wiki_heading("Anschluss Sub-branch")
        + wiki_row("B", "Beta")
    )
    focuses = parse_wiki_page(markup)

    assert [f["name"] for f in focuses] == ["Alpha", "Beta"]
    assert [f["position"] for f in focuses] == [0, 1]
    assert [f["section"] for f in focuses] == ["Rhineland Branch", "Anschluss Sub-branch"]


def test_a_focus_outside_any_heading_is_uncategorized():
    [focus] = parse_wiki_page(wiki_row())
    assert focus["section"] == "Uncategorized"


def test_thumbnail_rows_are_not_a_second_copy_of_a_focus():
    """A focus reappears in the prerequisite list of every focus that follows it,
    where the wiki links a resized copy — those rows are the same focus again."""
    markup = wiki_row() + wiki_row(image="/thumb.php?f=Goal_generic.png&width=36")

    assert len(parse_wiki_page(markup)) == 1


def test_the_same_row_twice_is_stored_once():
    assert len(parse_wiki_page(wiki_row() + wiki_row())) == 1


def test_a_page_with_no_focus_row_gives_nothing():
    assert parse_wiki_page("<p>nothing here</p>") == []


def three_rows(cells):
    """Alpha and Beta to depend on, and Gamma carrying the markup under test."""
    return wiki_row("Alpha", "Alpha") + wiki_row("Beta", "Beta") + wiki_row("Gamma", "Gamma", cells=cells)


def dependencies(cells):
    """`{id: (requires, excludes)}` for a page of those three rows."""
    return {f["id"]: (f["requires"], f["excludes"]) for f in parse_wiki_page(three_rows(cells))}


def test_a_focus_with_no_requirement_cell_depends_on_nothing():
    [alpha] = parse_wiki_page(wiki_row("Alpha", "Alpha"))

    assert alpha["requires"] == []
    assert alpha["excludes"] == []


def test_requires_the_following_is_one_group_for_each_focus():
    """Every one of them has to be taken, so each stands alone in its own group."""
    assert dependencies(requirements(requires("Alpha", "Beta")))["Gamma"][0] == [["Alpha"], ["Beta"]]


def test_requires_one_of_the_following_is_a_single_group():
    """Any one will do, so they share a group — met as soon as one is planned."""
    cells = requirements(requires("Alpha", "Beta", any_of=True))

    assert dependencies(cells)["Gamma"][0] == [["Alpha", "Beta"]]


def test_a_choice_nested_inside_a_list_is_a_group_of_its_own():
    """"Requires the following: Alpha, and one of Beta or Gamma" — the page nests
    the second list inside the first."""
    cells = requirements(requires("Alpha", requires("Beta", "Gamma", any_of=True)))

    assert dependencies(cells)["Gamma"][0] == [["Alpha"], ["Beta", "Gamma"]]


def test_what_the_catalog_cannot_name_is_dropped():
    """A requirement list also links ideologies, countries, and focuses from a
    branch the saved page does not carry. None can be planned here."""
    cells = requirements(requires(("Elsewhere", "Off-page focus"), "Alpha"))

    assert dependencies(cells)["Gamma"][0] == [["Alpha"]]


def test_a_requirement_of_nothing_the_catalog_knows_is_no_requirement():
    """Rather than a group nothing could ever satisfy, which would strand the
    focus for good."""
    cells = requirements(requires(("Elsewhere", "Off-page focus")))

    assert dependencies(cells)["Gamma"][0] == []


def test_a_requirement_that_is_not_a_link_at_all_is_ignored():
    cells = requirements(requires("<li>Is fully independent</li>", "Alpha"))

    assert dependencies(cells)["Gamma"][0] == [["Alpha"]]


def test_mutual_exclusion_is_read_from_both_sides():
    """The page writes it under one focus; the other has to know as well."""
    found = dependencies(requirements(excludes("Alpha")))

    assert found["Gamma"][1] == ["Alpha"]
    assert found["Alpha"][1] == ["Gamma"]


def test_a_focus_is_not_made_exclusive_with_itself():
    assert dependencies(requirements(excludes("Gamma")))["Gamma"][1] == []


def test_requirements_and_exclusions_of_one_focus_are_read_together():
    cells = requirements(requires("Alpha"), excludes("Beta"))
    found = dependencies(cells)

    assert found["Gamma"] == ([["Alpha"]], ["Beta"])


def test_only_the_requirement_cell_is_read():
    """A real row's effects cell is full of wording that reads like a requirement
    without being one — it is a different cell, and never looked at."""
    cells = requirements(requires("Alpha"), effects="<ul>" + requires("Beta") + "</ul>")

    assert dependencies(cells)["Gamma"][0] == [["Alpha"]]


def test_unmet_is_empty_once_a_group_has_one_of_its_focuses():
    gamma = focus("Gamma", requires=[["Alpha", "Beta"]])

    assert unmet(gamma, set()) == [["Alpha", "Beta"]]
    assert unmet(gamma, {"Beta"}) == []


def test_every_group_has_to_be_met():
    gamma = focus("Gamma", requires=[["Alpha"], ["Beta"]])

    assert unmet(gamma, {"Alpha"}) == [["Beta"]]
    assert unmet(gamma, {"Alpha", "Beta"}) == []


def test_conflicts_names_the_planned_focus_that_rules_one_out():
    gamma = focus("Gamma", excludes=["Alpha"])

    assert conflicts(gamma, {"Beta"}) == []
    assert conflicts(gamma, {"Alpha", "Beta"}) == ["Alpha"]


def test_available_wants_nothing_missing_and_nothing_clashing():
    gamma = focus("Gamma", requires=[["Alpha"]], excludes=["Beta"])

    assert not available(gamma, set())
    assert available(gamma, {"Alpha"})
    assert not available(gamma, {"Alpha", "Beta"})


def test_a_focus_with_no_dependencies_is_always_available():
    assert available(focus("Gamma"), set())


def test_the_first_focus_completes_a_full_term_after_the_start():
    """1 Jan 1936 is when work starts, not a day of it: the first focus lands 35
    days later."""
    assert completion_date(DEFAULT_DAYS_PER_FOCUS) == date(1936, 2, 5)


def test_each_focus_completes_a_term_after_the_one_before():
    assert completion_date(2 * DEFAULT_DAYS_PER_FOCUS) == date(1936, 3, 11)
    assert completion_date(3 * DEFAULT_DAYS_PER_FOCUS) == date(1936, 4, 15)


def test_completion_honours_a_different_start():
    assert completion_date(10, date(1937, 1, 1)) == date(1937, 1, 11)


def test_an_unmarked_focus_costs_the_plans_own_term():
    assert days_for("Anschluss") == DEFAULT_DAYS_PER_FOCUS
    assert days_for("Anschluss", {"Rhineland": LONG_DAYS}) == DEFAULT_DAYS_PER_FOCUS
    assert days_for("Anschluss", {}, 40) == 40


def test_a_marked_focus_costs_what_it_was_marked():
    assert days_for("Anschluss", {"Anschluss": SHORT_DAYS}) == SHORT_DAYS
    assert days_for("Anschluss", {"Anschluss": LONG_DAYS}) == LONG_DAYS


@pytest.mark.parametrize("mark", [None, 0, -5, "soon", [70]])
def test_a_mark_that_is_not_a_term_falls_back(mark):
    """A mark comes out of a stored plan; one bad value should not stop the plan
    from drawing."""
    assert days_for("Anschluss", {"Anschluss": mark}) == DEFAULT_DAYS_PER_FOCUS


@pytest.mark.parametrize("value", [None, "", "not-a-date", "1936-13-99", 1936])
def test_an_unreadable_start_date_falls_back(value):
    assert parse_start(value) == DEFAULT_START


def test_a_readable_start_date_is_used():
    assert parse_start("1937-06-02") == date(1937, 6, 2)


def test_schedule_numbers_and_dates_each_focus_in_order():
    rows = schedule(["Rhineland", "Fortify_the_Vaterland"], by_id(catalog()))

    assert [row["position"] for row in rows] == [1, 2]
    assert [row["name"] for row in rows] == ["Remilitarize the Rhineland", "Fortify the Vaterland"]
    assert rows[0]["completes_on"] == "1936-02-05"
    assert rows[1]["completes_on"] == "1936-03-11"


def test_schedule_drops_a_focus_the_catalog_has_lost():
    """A plan saved before a re-import should lose the focus that went away,
    and the ones after it should close the gap rather than keep its slot."""
    rows = schedule(["gone", "Anschluss"], by_id(catalog()))

    assert [row["id"] for row in rows] == ["Anschluss"]
    assert rows[0]["position"] == 1
    assert rows[0]["completes_on"] == "1936-02-05"


def test_schedule_gives_a_marked_focus_its_own_term():
    """A 70-day focus pushes itself and everything after it back by 35 days."""
    rows = schedule(
        ["Rhineland", "Fortify_the_Vaterland"],
        by_id(catalog()),
        focus_days={"Rhineland": LONG_DAYS},
    )

    assert [row["days"] for row in rows] == [LONG_DAYS, DEFAULT_DAYS_PER_FOCUS]
    assert rows[0]["completes_on"] == "1936-03-11"
    assert rows[1]["completes_on"] == "1936-04-15"


def test_schedule_gives_a_short_focus_a_half_term():
    rows = schedule(["Rhineland"], by_id(catalog()), focus_days={"Rhineland": SHORT_DAYS})

    assert rows[0]["days"] == SHORT_DAYS
    assert rows[0]["completes_on"] == "1936-01-15"


def test_a_mark_on_a_focus_the_plan_does_not_hold_changes_nothing():
    rows = schedule(["Rhineland"], by_id(catalog()), focus_days={"Anschluss": LONG_DAYS})

    assert rows[0]["completes_on"] == "1936-02-05"


def test_total_days_counts_only_what_the_catalog_knows():
    assert total_days(["Anschluss", "gone"], by_id(catalog())) == DEFAULT_DAYS_PER_FOCUS


def test_total_days_adds_up_the_marked_terms():
    marks = {"Anschluss": LONG_DAYS, "Rhineland": SHORT_DAYS}
    plan = ["Anschluss", "Rhineland", "Fortify_the_Vaterland"]

    assert total_days(plan, by_id(catalog()), focus_days=marks) == (
        LONG_DAYS + SHORT_DAYS + DEFAULT_DAYS_PER_FOCUS
    )


def dependent_catalog():
    """Alpha, then Beta which needs it, and Gamma which Beta rules out."""
    return by_id([
        focus("Alpha", "Alpha", position=0),
        focus("Beta", "Beta", position=1, requires=[["Alpha"]]),
        focus("Gamma", "Gamma", position=2, excludes=["Beta"]),
    ])


def test_schedule_says_nothing_is_missing_when_the_order_works():
    rows = schedule(["Alpha", "Beta"], dependent_catalog())

    assert [row["needs"] for row in rows] == [[], []]
    assert [row["clashes"] for row in rows] == [[], []]


def test_schedule_marks_a_focus_planned_before_what_it_needs():
    """A prerequisite has to be *earlier*: dragging it below is a plan that
    cannot be played, not merely an odd one."""
    rows = schedule(["Beta", "Alpha"], dependent_catalog())

    assert rows[0]["needs"] == [["Alpha"]]
    assert rows[1]["needs"] == []


def test_schedule_marks_a_focus_the_plan_has_already_ruled_out():
    rows = schedule(["Beta", "Gamma"], dependent_catalog())

    assert rows[1]["clashes"] == ["Beta"]
    # The one that came first is fine; it is the later focus that cannot be taken.
    assert rows[0]["clashes"] == []


def test_an_empty_plan_takes_no_time():
    assert schedule([], by_id(catalog())) == []
    assert total_days([], by_id(catalog())) == 0
