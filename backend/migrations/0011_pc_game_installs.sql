CREATE TABLE IF NOT EXISTS pc_game_installs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    launchbox_db_id INTEGER NOT NULL UNIQUE,
    platform TEXT NOT NULL,
    collection TEXT NOT NULL,
    shortname TEXT NOT NULL,
    source_archive_path TEXT NOT NULL,
    companion_archive_path TEXT,
    metadata_archive_path TEXT NOT NULL,
    install_root TEXT NOT NULL,
    launch_config_path TEXT NOT NULL,
    prepared_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    last_used_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_pc_game_installs_collection
    ON pc_game_installs(collection);
