"""The JSON the planner saves through, and the file a plan travels as."""

import json

from core.db import load_draft, load_plans, replace_focuses, save_draft, save_plan
from core.plan_file import FORMAT, VERSION
from helpers import catalog


def post(client, **payload):
    return client.post("/api/plans", json={"name": "My plan", "focus_ids": [], **payload})


def put_draft(client, **payload):
    return client.put("/api/draft", json={"focus_ids": [], **payload})


def test_saving_a_plan_stores_it(client, fake_db):
    response = post(client, name="Barbarossa rush", focus_ids=["Anschluss", "Rhineland"])

    assert response.status_code == 201
    assert response.get_json()["plan"]["id"] == "barbarossa-rush"
    assert load_plans()[0]["focus_ids"] == ["Anschluss", "Rhineland"]


def test_the_stored_order_is_the_order_posted(client, fake_db):
    post(client, focus_ids=["c", "a", "b"])

    assert load_plans()[0]["focus_ids"] == ["c", "a", "b"]


def test_a_plan_needs_a_name(client, fake_db):
    response = client.post("/api/plans", json={"focus_ids": []})

    assert response.status_code == 400
    assert "name" in response.get_json()["error"]


def test_a_name_with_nothing_to_slug_is_refused(client, fake_db):
    response = post(client, name="!!!")

    assert response.status_code == 400
    assert load_plans() == []


def test_focus_ids_must_be_a_list(client, fake_db):
    response = post(client, focus_ids="Anschluss")

    assert response.status_code == 400


def test_focus_ids_must_be_ids(client, fake_db):
    response = post(client, focus_ids=["Anschluss", 7])

    assert response.status_code == 400
    assert load_plans() == []


def test_days_per_focus_must_be_a_positive_whole_number(client, fake_db):
    assert post(client, days_per_focus="soon").status_code == 400
    assert post(client, days_per_focus=0).status_code == 400


def test_days_per_focus_defaults_to_thirty_five(client, fake_db):
    post(client)

    assert load_plans()[0]["days_per_focus"] == 35


def test_focus_days_are_stored_against_their_focus(client, fake_db):
    post(client, focus_ids=["Anschluss", "Rhineland"], focus_days={"Anschluss": 70})

    assert load_plans()[0]["focus_days"] == {"Anschluss": 70}


def test_focus_days_default_to_none_marked(client, fake_db):
    post(client, focus_ids=["Anschluss"])

    assert load_plans()[0]["focus_days"] == {}


def test_focus_days_must_be_an_object(client, fake_db):
    assert post(client, focus_days=[70, 35]).status_code == 400


def test_focus_days_must_be_positive_whole_numbers(client, fake_db):
    assert post(client, focus_days={"Anschluss": "soon"}).status_code == 400
    assert post(client, focus_days={"Anschluss": 0}).status_code == 400


def test_listing_gives_every_plan(client, fake_db):
    save_plan("First", [], "1936-01-01")
    save_plan("Second", [], "1936-01-01")

    names = [plan["name"] for plan in client.get("/api/plans").get_json()["plans"]]
    assert names == ["Second", "First"]


def test_one_plan_can_be_fetched_back(client, fake_db):
    save_plan("Barbarossa rush", ["Anschluss"], "1937-05-02")
    plan = client.get("/api/plans/barbarossa-rush").get_json()["plan"]

    assert plan["focus_ids"] == ["Anschluss"]
    assert plan["start_date"] == "1937-05-02"


def test_fetching_a_plan_that_is_not_there(client, fake_db):
    assert client.get("/api/plans/nope").status_code == 404


def test_deleting_a_plan(client, fake_db):
    save_plan("Doomed", [], "1936-01-01")

    assert client.delete("/api/plans/doomed").status_code == 200
    assert load_plans() == []


def test_deleting_a_plan_that_is_not_there(client, fake_db):
    assert client.delete("/api/plans/nope").status_code == 404


def test_every_route_reports_an_unreachable_database(client, mongo_is_down):
    assert client.get("/api/plans").status_code == 503
    assert client.get("/api/plans/any").status_code == 503
    assert post(client).status_code == 503
    assert client.delete("/api/plans/any").status_code == 503
    assert put_draft(client).status_code == 503
    assert client.delete("/api/draft").status_code == 503
    assert client.post("/api/plans/export", json={"focus_ids": []}).status_code == 503
    assert client.get("/api/plans/any/export").status_code == 503
    assert client.post("/api/plans/import", json=a_file()).status_code == 503


# --- the draft: what the planner writes after every edit ----------------------


def test_the_draft_is_kept_without_a_name(client, fake_db):
    response = put_draft(client, focus_ids=["Anschluss", "Rhineland"])

    assert response.status_code == 200
    assert load_draft()["focus_ids"] == ["Anschluss", "Rhineland"]
    assert load_plans() == []


def test_the_draft_keeps_the_name_being_typed(client, fake_db):
    put_draft(client, name="Barbarossa ru")

    assert load_draft()["name"] == "Barbarossa ru"


def test_writing_the_draft_again_replaces_it(client, fake_db):
    put_draft(client, focus_ids=["Anschluss"])
    put_draft(client, focus_ids=["Anschluss", "Rhineland"], focus_days={"Rhineland": 14})

    assert load_draft()["focus_ids"] == ["Anschluss", "Rhineland"]
    assert load_draft()["focus_days"] == {"Rhineland": 14}


def test_the_draft_is_checked_like_a_save_is(client, fake_db):
    assert put_draft(client, focus_ids="Anschluss").status_code == 400
    assert put_draft(client, days_per_focus=0).status_code == 400
    assert put_draft(client, focus_days=["Anschluss"]).status_code == 400
    assert put_draft(client, focus_days={"Anschluss": -1}).status_code == 400
    assert load_draft() is None


def test_clearing_throws_the_draft_away(client, fake_db):
    save_draft("", ["Anschluss"], "1936-01-01")

    response = client.delete("/api/draft")

    assert response.status_code == 200
    assert response.get_json()["cleared"] is True
    assert load_draft() is None


def test_clearing_a_plan_that_was_never_kept_is_not_an_error(client, fake_db):
    response = client.delete("/api/draft")

    assert response.status_code == 200
    assert response.get_json()["cleared"] is False


def test_clearing_the_draft_leaves_the_saved_plans_alone(client, fake_db):
    save_plan("Barbarossa rush", ["Anschluss"], "1936-01-01")
    save_draft("", ["Rhineland"], "1936-01-01")

    client.delete("/api/draft")

    assert [plan["id"] for plan in load_plans()] == ["barbarossa-rush"]


# --- a plan as a file ---------------------------------------------------------


def a_file(**fields):
    """A plan file, as `POST /api/plans/import` expects one."""
    return {
        "format": FORMAT,
        "version": VERSION,
        "name": "Barbarossa rush",
        "focus_ids": ["Rhineland", "Anschluss"],
        "start_date": "1936-01-01",
        "days_per_focus": 35,
        "focus_days": {"Anschluss": 70},
        **fields,
    }


def export(client, **payload):
    """The panel, exported — what the Export button posts."""
    return client.post(
        "/api/plans/export",
        json={"name": "Barbarossa rush", "focus_ids": ["Rhineland"], **payload},
    )


def downloaded(response):
    """The file out of a download, and the name it came down under."""
    disposition = response.headers["Content-Disposition"]
    return json.loads(response.get_data(as_text=True)), disposition


def test_exporting_the_panel_gives_a_plan_file(client, fake_db):
    replace_focuses(catalog())

    file, disposition = downloaded(export(client, focus_ids=["Rhineland", "Anschluss"]))

    assert file["format"] == FORMAT
    assert file["focus_ids"] == ["Rhineland", "Anschluss"]
    assert "attachment" in disposition


def test_the_download_is_named_after_the_plan(client, fake_db):
    _, disposition = downloaded(export(client, name="Barbarossa rush"))

    assert 'filename="barbarossa-rush.hoi4plan.json"' in disposition


def test_a_plan_with_no_usable_name_still_downloads(client, fake_db):
    _, disposition = downloaded(export(client, name=""))

    assert 'filename="plan.hoi4plan.json"' in disposition


def test_the_exported_file_names_and_dates_the_focuses(client, fake_db):
    replace_focuses(catalog())

    file, _ = downloaded(export(client, focus_ids=["Rhineland"]))

    assert file["focuses"] == [
        {
            "position": 1,
            "id": "Rhineland",
            "name": "Remilitarize the Rhineland",
            "days": 35,
            "completes_on": "1936-02-05",
        }
    ]


def test_the_panel_is_checked_the_way_a_save_is(client, fake_db):
    assert export(client, days_per_focus=0).status_code == 400
    assert export(client, focus_ids="Rhineland").status_code == 400


def test_a_saved_plan_exports_by_id(client, fake_db):
    replace_focuses(catalog())
    save_plan("Barbarossa rush", ["Rhineland"], "1936-01-01", focus_days={"Rhineland": 14})

    file, disposition = downloaded(client.get("/api/plans/barbarossa-rush/export"))

    assert file["name"] == "Barbarossa rush"
    assert file["focus_days"] == {"Rhineland": 14}
    assert 'filename="barbarossa-rush.hoi4plan.json"' in disposition


def test_exporting_a_plan_that_is_not_there(client, fake_db):
    assert client.get("/api/plans/nope/export").status_code == 404


def test_a_plan_survives_the_round_trip(client, fake_db):
    replace_focuses(catalog())
    save_plan("Barbarossa rush", ["Rhineland", "Anschluss"], "1937-06-01",
              days_per_focus=20, focus_days={"Anschluss": 70})
    file, _ = downloaded(client.get("/api/plans/barbarossa-rush/export"))

    plan = client.post("/api/plans/import", json=file).get_json()["plan"]

    assert plan == {
        "name": "Barbarossa rush",
        "focus_ids": ["Rhineland", "Anschluss"],
        "start_date": "1937-06-01",
        "days_per_focus": 20,
        "focus_days": {"Anschluss": 70},
    }


def test_importing_stores_nothing(client, fake_db):
    replace_focuses(catalog())

    client.post("/api/plans/import", json=a_file())

    assert load_plans() == []
    assert load_draft() is None


def test_importing_says_which_focuses_this_catalog_has_lost(client, fake_db):
    replace_focuses(catalog())

    body = client.post(
        "/api/plans/import", json=a_file(focus_ids=["Rhineland", "Sea_Lion"])
    ).get_json()

    assert body["unknown"] == ["Sea_Lion"]
    # Kept all the same: the plan is the order, and it is not ours to shorten.
    assert body["plan"]["focus_ids"] == ["Rhineland", "Sea_Lion"]


def test_a_file_that_is_not_one_of_ours_is_refused(client, fake_db):
    replace_focuses(catalog())

    response = client.post("/api/plans/import", json={"focus_ids": ["Rhineland"]})

    assert response.status_code == 400
    assert FORMAT in response.get_json()["error"]


def test_an_imported_file_is_checked_the_way_a_save_is(client, fake_db):
    replace_focuses(catalog())

    assert client.post(
        "/api/plans/import", json=a_file(focus_days={"Rhineland": -1})
    ).status_code == 400
    assert client.post(
        "/api/plans/import", json=a_file(focus_ids=[7])
    ).status_code == 400
