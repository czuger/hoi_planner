"""A plan as a file: the envelope written, and the envelope read back."""

import pytest

from core.focuses import by_id
from core.plan_file import FORMAT, VERSION, from_file, to_file
from helpers import catalog


def plan(**fields):
    """A stored plan, as `core.db.load_plan()` hands one over."""
    return {
        "name": "Barbarossa rush",
        "focus_ids": ["Rhineland", "Anschluss"],
        "start_date": "1936-01-01",
        "days_per_focus": 35,
        "focus_days": {"Anschluss": 70},
        **fields,
    }


def exported(**fields):
    """A file as `to_file()` writes it, before anything is changed about it."""
    return to_file(plan(**fields), by_id(catalog()))


# --- writing one --------------------------------------------------------------


def test_the_file_says_what_it_is():
    file = exported()

    assert file["format"] == FORMAT
    assert file["version"] == VERSION


def test_the_file_carries_the_whole_plan():
    file = exported()

    assert file["name"] == "Barbarossa rush"
    assert file["focus_ids"] == ["Rhineland", "Anschluss"]
    assert file["start_date"] == "1936-01-01"
    assert file["days_per_focus"] == 35
    assert file["focus_days"] == {"Anschluss": 70}


def test_the_echo_names_and_dates_every_focus():
    file = exported()

    assert file["focuses"] == [
        {
            "position": 1,
            "id": "Rhineland",
            "name": "Remilitarize the Rhineland",
            "days": 35,
            "completes_on": "1936-02-05",
        },
        {
            "position": 2,
            "id": "Anschluss",
            "name": "Anschluss",
            "days": 70,
            "completes_on": "1936-04-15",
        },
    ]


def test_the_echo_is_dated_from_the_plans_own_start():
    file = exported(start_date="1937-06-01", focus_ids=["Rhineland"], focus_days={})

    assert file["focuses"][0]["completes_on"] == "1937-07-06"


def test_a_focus_the_catalog_lost_is_left_out_of_the_echo_but_kept_in_the_plan():
    file = to_file(plan(focus_ids=["Rhineland", "Gone"]), by_id(catalog()))

    assert file["focus_ids"] == ["Rhineland", "Gone"]
    assert [row["id"] for row in file["focuses"]] == ["Rhineland"]


def test_the_plan_travels_without_a_catalog_to_name_it():
    file = to_file(plan())

    assert file["focus_ids"] == ["Rhineland", "Anschluss"]
    assert file["focuses"] == []


def test_nothing_of_the_database_travels():
    file = to_file(plan(id="barbarossa-rush", created_at="then", updated_at="now"))

    assert "id" not in file
    assert "created_at" not in file
    assert "updated_at" not in file


def test_exporting_the_same_plan_twice_gives_the_same_file():
    assert exported() == exported()


def test_a_plan_missing_its_fields_still_exports():
    file = to_file({})

    assert file["focus_ids"] is None
    assert file["focuses"] == []


# --- reading one back ---------------------------------------------------------


def test_what_was_written_is_what_is_read_back():
    read, problem = from_file(exported())

    assert problem is None
    assert read == {
        key: plan()[key]
        for key in ("name", "focus_ids", "start_date", "days_per_focus", "focus_days")
    }


def test_the_readable_echo_is_ignored():
    file = exported()
    file["focuses"] = [{"id": "Something_else", "name": "made up"}]

    read, _ = from_file(file)

    assert read["focus_ids"] == ["Rhineland", "Anschluss"]


@pytest.mark.parametrize("data", [None, [], "a plan", 3])
def test_json_that_is_not_an_object_is_refused(data):
    read, problem = from_file(data)

    assert read is None
    assert "JSON object" in problem


def test_json_that_is_not_one_of_ours_is_refused():
    read, problem = from_file({"focus_ids": ["Rhineland"]})

    assert read is None
    assert FORMAT in problem


def test_a_file_with_no_version_is_refused():
    file = exported()
    del file["version"]

    read, problem = from_file(file)

    assert read is None
    assert "version" in problem


@pytest.mark.parametrize("version", [0, -1, "1", 1.0, True])
def test_a_version_that_is_not_a_version_is_refused(version):
    read, problem = from_file({**exported(), "version": version})

    assert read is None
    assert "version" in problem


def test_a_file_from_a_newer_planner_is_refused_rather_than_misread():
    read, problem = from_file({**exported(), "version": VERSION + 1})

    assert read is None
    assert str(VERSION + 1) in problem


def test_the_values_are_handed_over_as_they_came():
    """What a plan may hold is checked once, where a save is checked — not here."""
    read, problem = from_file({"format": FORMAT, "version": VERSION, "focus_ids": "nonsense"})

    assert problem is None
    assert read["focus_ids"] == "nonsense"
