"""The one page, and the health check."""

from core.db import replace_focuses, save_draft, save_plan
from helpers import catalog


def test_the_page_lists_every_focus(client, fake_db):
    replace_focuses(catalog())
    body = client.get("/").get_data(as_text=True)

    assert "Remilitarize the Rhineland" in body
    assert "Fortify the Vaterland" in body


def test_the_page_groups_focuses_under_their_branch(client, fake_db):
    replace_focuses(catalog())
    body = client.get("/").get_data(as_text=True)

    assert 'data-section="Rhineland Branch"' in body
    assert 'data-section="Anschluss Sub-branch"' in body


def test_the_page_links_icons_rather_than_serving_them(client, fake_db):
    replace_focuses(catalog())
    body = client.get("/").get_data(as_text=True)

    assert "https://hoi4.paradoxwikis.com/images/" in body


def test_the_page_offers_the_saved_plans(client, fake_db):
    replace_focuses(catalog())
    save_plan("Barbarossa rush", ["Anschluss"], "1936-01-01")

    assert "Barbarossa rush" in client.get("/").get_data(as_text=True)


def test_the_page_carries_the_plan_that_was_being_built(client, fake_db):
    """The draft comes down with the page, so the panel draws it on the first
    render rather than fetching it afterwards."""
    replace_focuses(catalog())
    save_draft("Half named", ["Anschluss"], "1937-01-01")

    body = client.get("/").get_data(as_text=True)

    assert "data-draft=" in body
    assert "Anschluss" in body
    assert "1937-01-01" in body


def test_the_page_draws_with_no_draft_to_pick_up(client, fake_db):
    replace_focuses(catalog())

    assert "data-draft='{}'" in client.get("/").get_data(as_text=True)


def test_the_page_carries_the_urls_a_plan_travels_through(client, fake_db):
    """Every route app.js calls is named by the page, so none is spelled out in
    the JavaScript and none breaks under a URL prefix."""
    replace_focuses(catalog())

    body = client.get("/").get_data(as_text=True)

    assert 'data-export-url="/api/plans/export"' in body
    assert 'data-import-url="/api/plans/import"' in body


def test_the_draft_is_not_offered_among_the_saved_plans(client, fake_db):
    replace_focuses(catalog())
    save_draft("Half named", ["Anschluss"], "1936-01-01")

    body = client.get("/").get_data(as_text=True)

    assert "<option value=\"~draft\">" not in body


def test_an_empty_catalog_says_how_to_seed_it(client, fake_db):
    body = client.get("/").get_data(as_text=True)

    assert "import_focuses" in body


def test_the_page_still_draws_when_mongo_is_down(client, mongo_is_down):
    response = client.get("/")

    assert response.status_code == 200
    assert "Could not reach the database" in response.get_data(as_text=True)


def test_health_reports_the_catalog_size(client, fake_db):
    replace_focuses(catalog())
    body = client.get("/health").get_json()

    assert body["status"] == "ok"
    assert body["focuses"] == 3
    assert body["database"] == "hoi4_planner_dev"


def test_health_is_degraded_when_mongo_is_down(client, mongo_is_down):
    response = client.get("/health")

    assert response.status_code == 503
    assert response.get_json()["status"] == "degraded"
