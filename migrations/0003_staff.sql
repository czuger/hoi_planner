-- The staff and designers of the German Reich page: its Personnel and Companies tables.
-- `section` is the table a row comes from, which is the post it fills. `icon` and `icon_page`
-- are empty when the page shows no portrait.
CREATE TABLE staff (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    icon TEXT NOT NULL,
    icon_page TEXT NOT NULL,
    section TEXT NOT NULL,
    position INTEGER NOT NULL,
    requires TEXT NOT NULL CHECK (json_valid(requires)),
    excludes TEXT NOT NULL CHECK (json_valid(excludes))
) STRICT;

CREATE INDEX staff_position ON staff (position);

-- [staff ids], in the order they were picked: each fills the post of the table it comes from.
ALTER TABLE plans
    ADD COLUMN staff_ids TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(staff_ids));
