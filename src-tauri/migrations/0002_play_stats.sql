-- Play statistics tracking
CREATE TABLE IF NOT EXISTS play_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    launchbox_db_id INTEGER NOT NULL,
    game_title TEXT NOT NULL,
    platform TEXT NOT NULL,
    play_count INTEGER DEFAULT 0 NOT NULL,
    total_play_time_seconds INTEGER DEFAULT 0 NOT NULL,
    last_played TIMESTAMP,
    first_played TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    UNIQUE(launchbox_db_id)
);

CREATE INDEX IF NOT EXISTS idx_play_stats_launchbox_id ON play_stats(launchbox_db_id);
CREATE INDEX IF NOT EXISTS idx_play_stats_last_played ON play_stats(last_played);
CREATE INDEX IF NOT EXISTS idx_play_stats_play_count ON play_stats(play_count);
