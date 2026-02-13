-- Create categories table
CREATE TABLE IF NOT EXISTS categories (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  description TEXT NOT NULL DEFAULT '',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_categories_name ON categories (name);

-- Seed existing kinds as categories (fixed UUIDs for determinism)
INSERT INTO categories (id, name) VALUES
  ('c0000000-0000-0000-0000-000000000001', 'book'),
  ('c0000000-0000-0000-0000-000000000002', 'manga'),
  ('c0000000-0000-0000-0000-000000000003', 'article'),
  ('c0000000-0000-0000-0000-000000000004', 'animation'),
  ('c0000000-0000-0000-0000-000000000005', 'movie'),
  ('c0000000-0000-0000-0000-000000000006', 'series'),
  ('c0000000-0000-0000-0000-000000000007', 'note'),
  ('c0000000-0000-0000-0000-000000000008', 'link')
ON CONFLICT (name) DO NOTHING;

-- Create tags table
CREATE TABLE IF NOT EXISTS tags (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tags_name ON tags (name);

-- Create entry_tags junction table
CREATE TABLE IF NOT EXISTS entry_tags (
  entry_id TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
  tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (entry_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_entry_tags_tag_id ON entry_tags (tag_id);

-- Remove CHECK constraint on entries.kind
ALTER TABLE entries DROP CONSTRAINT IF EXISTS entries_kind_check;

-- Migrate existing tags_json data into tags + entry_tags
INSERT INTO tags (id, name)
SELECT DISTINCT
  gen_random_uuid()::text,
  tag_value
FROM entries,
  jsonb_array_elements_text(tags_json::jsonb) AS tag_value
WHERE tags_json != '[]'
ON CONFLICT (name) DO NOTHING;

INSERT INTO entry_tags (entry_id, tag_id)
SELECT e.id, t.id
FROM entries e,
  jsonb_array_elements_text(e.tags_json::jsonb) AS tag_value
  JOIN tags t ON t.name = tag_value
WHERE e.tags_json != '[]'
ON CONFLICT DO NOTHING;
