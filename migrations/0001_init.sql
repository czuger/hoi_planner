-- `requires`, `excludes`, `focus_ids` and `focus_days` are JSON: they are only ever read whole,
-- and a plan may hold ids the catalog has lost, so nothing here joins the two tables.
CREATE TABLE focuses (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    icon TEXT NOT NULL,
    icon_page TEXT NOT NULL,
    section TEXT NOT NULL,
    position INTEGER NOT NULL,
    requires TEXT NOT NULL CHECK (json_valid(requires)),
    excludes TEXT NOT NULL CHECK (json_valid(excludes))
) STRICT;

CREATE INDEX focuses_position ON focuses (position);

CREATE TABLE plans (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    focus_ids TEXT NOT NULL CHECK (json_valid(focus_ids)),
    start_date TEXT NOT NULL,
    days_per_focus INTEGER NOT NULL,
    focus_days TEXT NOT NULL CHECK (json_valid(focus_days)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE INDEX plans_updated_at ON plans (updated_at);
