BEGIN IMMEDIATE;

ALTER TABLE emulator_platforms
    ADD COLUMN wiki_rank INTEGER CHECK (wiki_rank IS NULL OR wiki_rank >= 1);
ALTER TABLE emulator_platforms
    ADD COLUMN wiki_verdict TEXT CHECK (
        wiki_verdict IS NULL
        OR wiki_verdict IN ('recommended', 'partial', 'not')
    );
CREATE UNIQUE INDEX emulator_platforms_wiki_rank
    ON emulator_platforms(platform_id, wiki_rank)
    WHERE wiki_rank IS NOT NULL;

INSERT INTO schema_migrations (version, name, applied_at)
VALUES (6, 'emulationwiki recommendation ranks', '1970-01-01T00:00:00Z');

COMMIT;
