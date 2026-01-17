-- Unified game media table for multi-source image downloads
-- Uses launchbox_db_id as primary identifier, with hash fallback for unmatched games

CREATE TABLE IF NOT EXISTS game_media (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- Primary identifier: LaunchBox database ID (stable across DB rebuilds)
    launchbox_db_id INTEGER,
    -- Fallback identifier: SHA256 hash of platform_id + normalized_title
    game_hash TEXT,
    -- Normalized media type: box-front, box-back, box-3d, screenshot, title-screen, clear-logo, fanart, banner
    media_type TEXT NOT NULL,
    -- Local file path after download (relative to media directory)
    local_path TEXT,
    -- Source that provided this media
    source TEXT NOT NULL,  -- launchbox, libretro, steamgriddb, igdb, emumovies, screenscraper
    -- Original URL from source
    source_url TEXT,
    -- Download status: pending, downloading, completed, failed
    status TEXT NOT NULL DEFAULT 'pending',
    -- Download progress (0.0 to 1.0)
    download_progress REAL DEFAULT 0,
    -- Error message if failed
    error_message TEXT,
    -- Timestamps
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    downloaded_at TIMESTAMP,
    -- Ensure unique media per game per source
    UNIQUE(launchbox_db_id, media_type, source),
    UNIQUE(game_hash, media_type, source),
    -- At least one identifier must be present
    CHECK (launchbox_db_id IS NOT NULL OR game_hash IS NOT NULL)
);

-- Index for fast lookups by launchbox_db_id
CREATE INDEX IF NOT EXISTS idx_game_media_lb_id ON game_media(launchbox_db_id);

-- Index for fast lookups by game_hash
CREATE INDEX IF NOT EXISTS idx_game_media_hash ON game_media(game_hash);

-- Index for finding media by type
CREATE INDEX IF NOT EXISTS idx_game_media_type ON game_media(media_type);

-- Index for finding pending downloads
CREATE INDEX IF NOT EXISTS idx_game_media_status ON game_media(status);

-- Composite index for common query: game + type + completed
CREATE INDEX IF NOT EXISTS idx_game_media_lookup ON game_media(launchbox_db_id, media_type, status);
