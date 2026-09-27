"""Every read and write — against an in-memory server, never a real mongod."""

import pytest

from core.db import (
    DRAFT_PLAN_ID,
    clear_draft,
    delete_plan,
    focus_count,
    load_focuses,
    load_plan,
    load_draft,
    load_plans,
    replace_focuses,
    save_draft,
    save_plan,
    slugify,
)
from helpers import catalog, focus


def test_replace_focuses_stores_the_catalog(fake_db):
    assert replace_focuses(catalog()) == 3
    assert {doc["id"] for doc in fake_db["focuses"].docs} == {
        "Fortify_the_Vaterland", "Rhineland", "Anschluss"
    }


def test_a_re_import_loses_what_the_page_no_longer_has(fake_db):
    replace_focuses(catalog())
    replace_focuses([focus("Anschluss", "Anschluss")])

    assert [doc["id"] for doc in fake_db["focuses"].docs] == ["Anschluss"]


def test_re_importing_nothing_empties_the_catalog(fake_db):
    replace_focuses(catalog())
    assert replace_focuses([]) == 0
    assert load_focuses() == []


def test_focuses_come_back_in_page_order(fake_db):
    replace_focuses(list(reversed(catalog())))

    assert [f["position"] for f in load_focuses()] == [0, 1, 2]
    assert load_focuses()[0]["id"] == "Fortify_the_Vaterland"


def test_a_focus_keeps_its_icon_links(fake_db):
    replace_focuses(catalog())
    stored = load_focuses()[0]

    assert stored["icon"].startswith("https://hoi4.paradoxwikis.com/")
    assert stored["icon_page"].startswith("https://hoi4.paradoxwikis.com/File:")


def test_focus_count_reports_the_catalog_size(fake_db):
    assert focus_count() == 0
    replace_focuses(catalog())
    assert focus_count() == 3


@pytest.mark.parametrize(
    "name,expected",
    [
        ("Barbarossa rush", "barbarossa-rush"),
        ("  Spaced  Out  ", "spaced-out"),
        ("Plan #1: Anschluss!", "plan-1-anschluss"),
        ("Café Züger", "cafe-zuger"),
        ("!!!", ""),
    ],
)
def test_slugify(name, expected):
    assert slugify(name) == expected


def test_save_plan_stores_the_order(fake_db):
    plan = save_plan("Barbarossa rush", ["Rhineland", "Anschluss"], "1936-01-01")

    assert plan["id"] == "barbarossa-rush"
    assert plan["name"] == "Barbarossa rush"
    assert plan["focus_ids"] == ["Rhineland", "Anschluss"]
    assert plan["days_per_focus"] == 35


def test_save_plan_keeps_the_terms_of_the_focuses_it_holds(fake_db):
    plan = save_plan("Marked", ["Rhineland", "Anschluss"], "1936-01-01", 35, {"Rhineland": 70})

    assert plan["focus_days"] == {"Rhineland": 70}


def test_save_plan_drops_a_term_for_a_focus_not_in_the_order(fake_db):
    """A focus dragged out of the plan takes its term with it, rather than leaving
    a mark behind that nothing on the page can see or clear."""
    plan = save_plan("Marked", ["Rhineland"], "1936-01-01", 35, {"Anschluss": 14})

    assert plan["focus_days"] == {}


def test_a_plan_with_no_terms_marked_has_none(fake_db):
    assert save_plan("Plain", ["Rhineland"], "1936-01-01")["focus_days"] == {}


def test_saving_the_same_name_revises_one_plan(fake_db):
    first = save_plan("My plan", ["Rhineland"], "1936-01-01")
    second = save_plan("My plan", ["Anschluss", "Rhineland"], "1937-01-01")

    assert len(load_plans()) == 1
    assert second["focus_ids"] == ["Anschluss", "Rhineland"]
    assert second["start_date"] == "1937-01-01"
    # An overwrite is the same plan revised, so it keeps its birthday.
    assert second["created_at"] == first["created_at"]
    assert second["updated_at"] > first["updated_at"]


def test_a_name_with_nothing_to_slug_is_refused(fake_db):
    assert save_plan("!!!", ["Rhineland"], "1936-01-01") is None
    assert load_plans() == []


def test_plans_come_back_most_recently_touched_first(fake_db):
    save_plan("First", [], "1936-01-01")
    save_plan("Second", [], "1936-01-01")

    assert [plan["name"] for plan in load_plans()] == ["Second", "First"]


def test_load_plan_finds_one_by_its_slug(fake_db):
    save_plan("Barbarossa rush", ["Anschluss"], "1936-01-01")

    assert load_plan("barbarossa-rush")["focus_ids"] == ["Anschluss"]


def test_load_plan_is_none_when_there_is_no_such_plan(fake_db):
    assert load_plan("nope") is None


def test_delete_plan_says_whether_there_was_one(fake_db):
    save_plan("Doomed", [], "1936-01-01")

    assert delete_plan("doomed") is True
    assert delete_plan("doomed") is False
    assert load_plans() == []


def test_a_plan_may_hold_a_focus_the_catalog_lost(fake_db):
    """Nothing joins the two collections: a re-import must not fail a save, and
    `schedule()` is what drops the id later."""
    replace_focuses([])
    plan = save_plan("Stale", ["gone_focus"], "1936-01-01")

    assert plan["focus_ids"] == ["gone_focus"]


# --- the draft: the plan being built, saved on every edit ----------------------


def test_save_draft_stores_the_plan_without_a_name_to_slug(fake_db):
    draft = save_draft("", ["Rhineland"], "1936-01-01")

    assert draft["id"] == DRAFT_PLAN_ID
    assert load_draft()["focus_ids"] == ["Rhineland"]


def test_the_draft_is_one_document_however_often_it_is_written(fake_db):
    first = save_draft("", ["Rhineland"], "1936-01-01")
    second = save_draft("Half named", ["Rhineland", "Anschluss"], "1936-01-01")

    assert len(fake_db["plans"].docs) == 1
    assert second["focus_ids"] == ["Rhineland", "Anschluss"]
    assert second["name"] == "Half named"
    assert second["created_at"] == first["created_at"]


def test_the_draft_keeps_the_terms_of_the_focuses_it_holds(fake_db):
    draft = save_draft("", ["Rhineland"], "1936-01-01", 35, {"Rhineland": 70, "Gone": 14})

    assert draft["focus_days"] == {"Rhineland": 70}


def test_the_draft_is_not_offered_among_the_saved_plans(fake_db):
    save_plan("Barbarossa rush", ["Anschluss"], "1936-01-01")
    save_draft("", ["Rhineland"], "1936-01-01")

    assert [plan["id"] for plan in load_plans()] == ["barbarossa-rush"]


def test_a_plan_named_draft_is_not_the_draft(fake_db):
    """No name can slug to the draft's id, so saving one cannot land on it."""
    save_draft("", ["Rhineland"], "1936-01-01")
    save_plan("Draft", ["Anschluss"], "1936-01-01")

    assert load_draft()["focus_ids"] == ["Rhineland"]
    assert load_plan("draft")["focus_ids"] == ["Anschluss"]


def test_load_draft_is_none_before_anything_is_built(fake_db):
    assert load_draft() is None


def test_clear_draft_says_whether_there_was_one(fake_db):
    save_draft("", ["Rhineland"], "1936-01-01")

    assert clear_draft() is True
    assert clear_draft() is False
    assert load_draft() is None


def test_clearing_the_draft_leaves_the_saved_plans_alone(fake_db):
    save_plan("Barbarossa rush", ["Anschluss"], "1936-01-01")
    save_draft("", ["Rhineland"], "1936-01-01")

    clear_draft()

    assert [plan["id"] for plan in load_plans()] == ["barbarossa-rush"]
