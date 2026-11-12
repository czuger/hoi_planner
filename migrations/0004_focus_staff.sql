-- {focus id: [staff ids]}: a member takes the date of the focus it is attached to. It replaces
-- `staff_ids`, the hires in picking order, which had no date to give them.
ALTER TABLE plans
    ADD COLUMN focus_staff TEXT NOT NULL DEFAULT '{}' CHECK (json_valid(focus_staff));

ALTER TABLE plans DROP COLUMN staff_ids;
