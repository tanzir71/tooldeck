CREATE TABLE settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE tool_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tool_id TEXT NOT NULL,
    input_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('done', 'job', 'failed')),
    output_path TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE jobs (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN ('queued', 'running', 'completed', 'failed', 'cancelled')
    ),
    progress REAL NOT NULL DEFAULT 0 CHECK (progress >= 0 AND progress <= 1),
    input_json TEXT NOT NULL,
    output_path TEXT,
    error TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE undo_ledger (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    op_kind TEXT NOT NULL,
    before_json TEXT NOT NULL,
    after_json TEXT NOT NULL,
    reversible INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO settings(key, value) VALUES
    ('theme', '"system"'),
    ('persona', '"freelancer"'),
    ('network_enabled', 'false');
