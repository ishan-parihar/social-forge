-- Alt text for uploaded media (accessibility). Nullable: existing rows have none.
ALTER TABLE media ADD COLUMN alt_text TEXT;
