-- Create categories table
CREATE TABLE IF NOT EXISTS categories (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  description TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_categories_name ON categories (name);

-- Seed existing kinds as categories (fixed UUIDs for determinism)
INSERT OR IGNORE INTO categories (id, name) VALUES
  ('c0000000-0000-0000-0000-000000000001', 'book'),
  ('c0000000-0000-0000-0000-000000000002', 'manga'),
  ('c0000000-0000-0000-0000-000000000003', 'article'),
  ('c0000000-0000-0000-0000-000000000004', 'animation'),
  ('c0000000-0000-0000-0000-000000000005', 'movie'),
  ('c0000000-0000-0000-0000-000000000006', 'series'),
  ('c0000000-0000-0000-0000-000000000007', 'note'),
  ('c0000000-0000-0000-0000-000000000008', 'link');

-- Create tags table
CREATE TABLE IF NOT EXISTS tags (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_tags_name ON tags (name);

-- Create entry_tags junction table
CREATE TABLE IF NOT EXISTS entry_tags (
  entry_id TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
  tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (entry_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_entry_tags_tag_id ON entry_tags (tag_id);

-- SQLite: recreate entries table without CHECK constraint on kind
CREATE TABLE entries_new (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  kind TEXT NOT NULL,
  status TEXT NOT NULL CHECK (
    status IN ('planned', 'in_progress', 'completed', 'dropped')
  ),
  notes TEXT NOT NULL DEFAULT '',
  url TEXT,
  source TEXT NOT NULL DEFAULT 'manual',
  tags_json TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

INSERT INTO entries_new SELECT * FROM entries;
DROP TABLE entries;
ALTER TABLE entries_new RENAME TO entries;

CREATE INDEX IF NOT EXISTS idx_entries_created_at ON entries (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_entries_kind_status ON entries (kind, status);

-- Migrate existing tags_json data into tags + entry_tags
INSERT OR IGNORE INTO tags (id, name)
SELECT
  lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' ||
    substr(hex(randomblob(2)),2) || '-' ||
    substr('89ab', abs(random()) % 4 + 1, 1) ||
    substr(hex(randomblob(2)),2) || '-' ||
    hex(randomblob(6))) AS id,
  j.value AS name
FROM entries e, json_each(e.tags_json) j
WHERE e.tags_json != '[]'
GROUP BY j.value;

INSERT OR IGNORE INTO entry_tags (entry_id, tag_id)
SELECT e.id, t.id
FROM entries e, json_each(e.tags_json) j
JOIN tags t ON t.name = j.value
WHERE e.tags_json != '[]';
