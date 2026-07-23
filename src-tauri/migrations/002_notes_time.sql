CREATE TABLE notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    project_id INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE VIRTUAL TABLE notes_fts USING fts5(
    path UNINDEXED,
    title,
    body,
    tokenize = 'unicode61'
);

CREATE TABLE tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE note_tags (
    note_id INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (note_id, tag_id)
);

CREATE TABLE note_links (
    source_path TEXT NOT NULL,
    target_name TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'wiki',
    PRIMARY KEY (source_path, target_name, kind)
);

CREATE TABLE clients (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    default_rate REAL NOT NULL DEFAULT 0,
    currency TEXT NOT NULL DEFAULT 'USD',
    archived INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    client_id INTEGER REFERENCES clients(id),
    name TEXT NOT NULL,
    color TEXT NOT NULL DEFAULT '#4D5FDB',
    billable INTEGER NOT NULL DEFAULT 0,
    rate REAL NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE time_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER REFERENCES projects(id),
    note_path TEXT,
    description TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    stopped_at INTEGER,
    source TEXT NOT NULL CHECK (source IN ('timer', 'manual', 'idle')),
    billable INTEGER NOT NULL DEFAULT 0,
    rate_snapshot REAL NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    CHECK (stopped_at IS NULL OR stopped_at > started_at)
);

CREATE INDEX time_entries_started_at_idx ON time_entries(started_at DESC);
CREATE UNIQUE INDEX one_running_timer_idx
    ON time_entries((1))
    WHERE stopped_at IS NULL;

INSERT INTO clients(name, default_rate, currency, created_at)
VALUES ('Personal', 0, 'USD', unixepoch());

INSERT INTO projects(client_id, name, color, billable, rate, created_at)
VALUES (
    (SELECT id FROM clients WHERE name = 'Personal' LIMIT 1),
    'General',
    '#4D5FDB',
    0,
    0,
    unixepoch()
);

INSERT INTO settings(key, value) VALUES
    ('idle_threshold_minutes', '5'),
    ('record_window_titles', 'false'),
    ('notes_template', '"blank"');
