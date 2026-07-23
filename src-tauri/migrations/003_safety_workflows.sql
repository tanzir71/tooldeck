ALTER TABLE undo_ledger ADD COLUMN undone_at TEXT;

CREATE TABLE workflows (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    steps_json TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE app_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    time_entry_id INTEGER REFERENCES time_entries(id) ON DELETE CASCADE,
    app_name TEXT NOT NULL,
    window_title TEXT,
    captured_at INTEGER NOT NULL
);

CREATE INDEX idx_app_activity_entry_time
    ON app_activity(time_entry_id, captured_at);
