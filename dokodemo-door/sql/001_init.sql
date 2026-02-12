CREATE TABLE IF NOT EXISTS entries (
  id UUID PRIMARY KEY,
  title TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (
    kind IN (
      'book',
      'manga',
      'article',
      'animation',
      'movie',
      'series',
      'note',
      'link'
    )
  ),
  status TEXT NOT NULL CHECK (
    status IN ('planned', 'in_progress', 'completed', 'dropped')
  ),
  notes TEXT NOT NULL DEFAULT '',
  url TEXT,
  source TEXT NOT NULL DEFAULT 'manual',
  tags TEXT[] NOT NULL DEFAULT '{}',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_entries_created_at ON entries (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_entries_kind_status ON entries (kind, status);
