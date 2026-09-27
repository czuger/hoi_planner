"""The seeding script: page -> JSON -> database."""

import json
import os

import pytest

import import_focuses
from core.db import load_focuses
from helpers import wiki_heading, wiki_row


@pytest.fixture
def paths(tmp_path, monkeypatch):
    """The script pointed at a page and a JSON file of the test's own."""
    page = tmp_path / "focuses.html"
    catalog_json = tmp_path / "focuses.json"
    monkeypatch.setattr(import_focuses, "WIKI_PAGE", page)
    monkeypatch.setattr(import_focuses, "FOCUSES_JSON", catalog_json)
    return page, catalog_json


def test_parses_the_page_and_seeds_the_database(paths, fake_db, capsys):
    page, catalog_json = paths
    page.write_text(wiki_heading() + wiki_row(), encoding="utf-8")

    assert import_focuses.main([]) == 0
    assert [f["name"] for f in load_focuses()] == ["Fortify the Vaterland"]
    # The JSON is written on the way through, for --from-json later.
    assert json.loads(catalog_json.read_text())[0]["id"] == "Fortify_the_Vaterland"
    assert "1 focuses stored" in capsys.readouterr().out


def test_json_only_leaves_the_database_alone(paths, fake_db):
    page, catalog_json = paths
    page.write_text(wiki_row(), encoding="utf-8")

    assert import_focuses.main(["--json-only"]) == 0
    assert catalog_json.exists()
    assert load_focuses() == []


def test_from_json_seeds_without_reading_the_page(paths, fake_db):
    page, catalog_json = paths
    catalog_json.write_text(json.dumps([{
        "id": "Anschluss", "name": "Anschluss", "icon": "https://x/i.png",
        "icon_page": "https://x/F", "section": "Anschluss Sub-branch", "position": 0,
    }]), encoding="utf-8")

    assert import_focuses.main(["--from-json"]) == 0
    assert [f["id"] for f in load_focuses()] == ["Anschluss"]
    # The page was never there, and was never needed.
    assert not page.exists()


def test_a_missing_page_says_so(paths, fake_db):
    with pytest.raises(FileNotFoundError, match="focuses.html"):
        import_focuses.main([])


def test_a_missing_json_says_so(paths, fake_db):
    with pytest.raises(FileNotFoundError, match="focuses.json"):
        import_focuses.main(["--from-json"])


def test_a_page_with_no_focus_is_an_error(paths, fake_db, capsys):
    page, _ = paths
    page.write_text("<p>nothing</p>", encoding="utf-8")

    assert import_focuses.main([]) == 1
    assert "No focus found" in capsys.readouterr().err


def test_an_unreachable_database_is_reported_not_raised(paths, mongo_is_down, capsys):
    page, _ = paths
    page.write_text(wiki_row(), encoding="utf-8")

    assert import_focuses.main([]) == 1
    assert "Could not write to the database" in capsys.readouterr().err


def test_env_and_uri_pick_the_database_this_run_writes_to(paths, fake_db, capsys):
    """--env / --mongo-uri only set what core.config and core.db read anyway."""
    page, _ = paths
    page.write_text(wiki_row(), encoding="utf-8")

    assert import_focuses.main(
        ["--env", "prod", "--mongo-uri", "mongodb://localhost:27018"]
    ) == 0
    assert "1 focuses stored in hoi4_planner_prod" in capsys.readouterr().out
    # Set for good, so the connection opened afterwards is the one asked for.
    assert os.environ["APP_ENV"] == "prod"
    assert os.environ["MONGO_URI"] == "mongodb://localhost:27018"


def test_without_them_nothing_is_pointed_anywhere(paths, fake_db, capsys):
    page, _ = paths
    page.write_text(wiki_row(), encoding="utf-8")

    assert import_focuses.main([]) == 0
    # clean_env dropped APP_ENV, so this is the default database, untouched.
    assert "stored in hoi4_planner_dev" in capsys.readouterr().out


def test_the_target_is_printed_before_it_is_written_to(paths, fake_db, capsys):
    page, _ = paths
    page.write_text(wiki_row(), encoding="utf-8")

    import_focuses.main(["--mongo-uri", "mongodb://alice:hunter2@nuc150:27017"])
    out = capsys.readouterr().out
    assert "seeding hoi4_planner_dev on mongodb://nuc150:27017" in out
    # A password on the way to a remote mongod is not printed.
    assert "hunter2" not in out


def test_an_unknown_env_is_refused_by_the_parser(paths, fake_db):
    with pytest.raises(SystemExit):
        import_focuses.main(["--env", "staging"])
