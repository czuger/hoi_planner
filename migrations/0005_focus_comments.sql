-- {focus id: [text]}: free comments written on a focus's line, in the order they sit there.
ALTER TABLE plans
    ADD COLUMN focus_comments TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(focus_comments));
