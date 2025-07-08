-- Add up migration script here
CREATE TABLE IF NOT EXISTS demo (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL
)