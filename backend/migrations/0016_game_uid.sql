-- Add a stable game identity (the games.db UUID) so games without a
-- launchbox_db_id (No-Intro / Minerva-only entries) can be tracked
-- individually instead of all colliding on launchbox_db_id = 0.
--
-- game_files previously had a table-level UNIQUE(launchbox_db_id), which forced
-- every id-less game onto the single "0" slot. Recreate the table without that
-- constraint and instead enforce uniqueness per real id (launchbox_db_id > 0)
-- and per game_uid.

CREATE TABLE game_files_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_uid TEXT,
    launchbox_db_id INTEGER NOT NULL,
    game_title TEXT NOT NULL,
    platform TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER,
    imported_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    import_source TEXT DEFAULT 'graboid' NOT NULL,
    graboid_job_id TEXT
);

INSERT INTO game_files_new
    (id, launchbox_db_id, game_title, platform, file_path, file_size, imported_at, import_source, graboid_job_id)
SELECT id, launchbox_db_id, game_title, platform, file_path, file_size, imported_at, import_source, graboid_job_id
FROM game_files;

DROP TABLE game_files;
ALTER TABLE game_files_new RENAME TO game_files;

CREATE UNIQUE INDEX idx_game_files_uid ON game_files(game_uid) WHERE game_uid IS NOT NULL;
CREATE UNIQUE INDEX idx_game_files_launchbox ON game_files(launchbox_db_id) WHERE launchbox_db_id > 0;
CREATE INDEX idx_game_files_source ON game_files(import_source);

-- Carry the same identity on download jobs so it can be propagated to the file.
ALTER TABLE graboid_jobs ADD COLUMN game_uid TEXT;
