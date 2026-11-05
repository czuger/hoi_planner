-- The research tree, read from the saved wiki pages. `year` and `base_days` are empty for the
-- secret and special-project techs, whose rows say neither.
CREATE TABLE technologies (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    icon TEXT NOT NULL,
    icon_page TEXT NOT NULL,
    domain TEXT NOT NULL,
    section TEXT NOT NULL,
    position INTEGER NOT NULL,
    year INTEGER,
    base_days INTEGER,
    requires TEXT NOT NULL CHECK (json_valid(requires)),
    excludes TEXT NOT NULL CHECK (json_valid(excludes))
) STRICT;

CREATE INDEX technologies_position ON technologies (position);

-- {focus id: [technology ids]}: a technology takes the date of the focus it is attached to.
ALTER TABLE plans
    ADD COLUMN focus_technologies TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(focus_technologies));
