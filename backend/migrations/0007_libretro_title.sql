-- Add libretro_title column for No-Intro naming convention titles
ALTER TABLE games ADD COLUMN libretro_title TEXT;

-- Index for libretro title lookups
CREATE INDEX IF NOT EXISTS idx_games_libretro_title ON games(libretro_title);
