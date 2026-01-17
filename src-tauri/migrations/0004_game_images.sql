-- Game images table for on-demand image download from LaunchBox CDN
-- Stores metadata about available images; actual files downloaded on-demand

CREATE TABLE IF NOT EXISTS game_images (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    launchbox_db_id INTEGER NOT NULL,
    filename TEXT NOT NULL,           -- CDN path (e.g., "Nintendo - NES/Box - Front/Super Mario Bros.-01.jpg")
    image_type TEXT NOT NULL,         -- "Box - Front", "Screenshot - Gameplay", "Clear Logo", etc.
    region TEXT,                      -- Optional regional variant
    crc32 TEXT,                       -- Checksum for verification
    downloaded INTEGER DEFAULT 0 NOT NULL, -- Boolean: has been downloaded locally
    local_path TEXT,                  -- Local file path after download
    priority INTEGER DEFAULT 0,       -- Download priority (lower = higher priority)
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Index for fast lookups by game
CREATE INDEX IF NOT EXISTS idx_game_images_db_id ON game_images(launchbox_db_id);

-- Index for filtering by image type
CREATE INDEX IF NOT EXISTS idx_game_images_type ON game_images(image_type);

-- Index for finding images that need downloading
CREATE INDEX IF NOT EXISTS idx_game_images_downloaded ON game_images(downloaded);

-- Composite index for common queries: game + type + not downloaded
CREATE INDEX IF NOT EXISTS idx_game_images_pending ON game_images(launchbox_db_id, image_type, downloaded);

-- Track download queue status
CREATE TABLE IF NOT EXISTS image_download_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_image_id INTEGER NOT NULL REFERENCES game_images(id),
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, downloading, completed, failed
    progress REAL DEFAULT 0,                 -- 0.0 to 1.0
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_download_queue_status ON image_download_queue(status);
CREATE INDEX IF NOT EXISTS idx_download_queue_image ON image_download_queue(game_image_id);
