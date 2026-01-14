-- Favorites tracking
CREATE TABLE IF NOT EXISTS favorites (
    launchbox_db_id INTEGER PRIMARY KEY,
    game_title TEXT NOT NULL,
    platform TEXT NOT NULL,
    added_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_favorites_added ON favorites(added_at);
