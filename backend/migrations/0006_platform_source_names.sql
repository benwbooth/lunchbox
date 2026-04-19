-- Add source-specific platform names for image lookups

ALTER TABLE platforms ADD COLUMN launchbox_name TEXT;
ALTER TABLE platforms ADD COLUMN libretro_name TEXT;
ALTER TABLE platforms ADD COLUMN openvgdb_system_id INTEGER;
ALTER TABLE platforms ADD COLUMN manufacturer TEXT;
ALTER TABLE platforms ADD COLUMN release_date TEXT;
ALTER TABLE platforms ADD COLUMN category TEXT;
ALTER TABLE platforms ADD COLUMN aliases TEXT;

-- Create index for libretro lookups
CREATE INDEX IF NOT EXISTS idx_platforms_libretro ON platforms(libretro_name);
