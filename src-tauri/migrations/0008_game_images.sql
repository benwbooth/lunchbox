-- Game images table for LaunchBox CDN lookups
CREATE TABLE IF NOT EXISTS game_images (
    id INTEGER PRIMARY KEY,
    launchbox_db_id INTEGER NOT NULL,
    filename TEXT NOT NULL,  -- UUID filename like "3c4cc1f6-051a-43f5-b904-b60eed55b074.jpg"
    image_type TEXT NOT NULL,  -- "Box - Front", "Screenshot - Gameplay", etc.
    region TEXT,  -- "North America", "Europe", etc.
    crc32 TEXT,
    UNIQUE(launchbox_db_id, filename)
);

-- Index for fast lookups by game ID and type
CREATE INDEX IF NOT EXISTS idx_game_images_lookup ON game_images(launchbox_db_id, image_type);
