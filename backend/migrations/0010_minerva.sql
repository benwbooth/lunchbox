-- Index on game_files by import_source for filtering minerva vs graboid imports
CREATE INDEX IF NOT EXISTS idx_game_files_source ON game_files(import_source);
