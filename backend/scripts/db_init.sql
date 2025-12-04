-- Initialize schema for VeilCast backend (idempotent)

TRUNCATE TABLE votes RESTART IDENTITY CASCADE;
TRUNCATE TABLE commitments RESTART IDENTITY CASCADE;
TRUNCATE TABLE poll_members RESTART IDENTITY CASCADE;
TRUNCATE TABLE polls RESTART IDENTITY CASCADE;
TRUNCATE TABLE members RESTART IDENTITY CASCADE;
ALTER SEQUENCE IF EXISTS polls_id_seq RESTART WITH 0;
ALTER SEQUENCE IF EXISTS members_id_seq RESTART WITH 0;
ALTER SEQUENCE IF EXISTS commitments_id_seq RESTART WITH 0;
ALTER SEQUENCE IF EXISTS votes_id_seq RESTART WITH 0;

CREATE TABLE IF NOT EXISTS polls (
    id BIGSERIAL PRIMARY KEY,
    question TEXT NOT NULL,
    options JSONB NOT NULL,
    commit_phase_end TIMESTAMPTZ NOT NULL,
    reveal_phase_end TIMESTAMPTZ NOT NULL,
    category TEXT NOT NULL DEFAULT 'General',
    commit_sync_completed BOOLEAN NOT NULL DEFAULT false,
    membership_root TEXT NOT NULL,
    correct_option SMALLINT,
    resolved BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
ALTER TABLE polls ADD COLUMN IF NOT EXISTS category TEXT NOT NULL DEFAULT 'General';
ALTER TABLE polls ADD COLUMN IF NOT EXISTS commit_sync_completed BOOLEAN NOT NULL DEFAULT false;
UPDATE polls SET category = 'General' WHERE category IS NULL OR category = '';
UPDATE polls SET commit_sync_completed = false WHERE commit_sync_completed IS NULL;

CREATE TABLE IF NOT EXISTS members (
    id SERIAL PRIMARY KEY,
    identity_secret TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS commitments (
    id SERIAL PRIMARY KEY,
    poll_id BIGINT NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    commitment TEXT NOT NULL,
    identity_secret TEXT NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    onchain_submitted BOOLEAN NOT NULL DEFAULT false
);
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'commitments' AND column_name = 'id'
    ) THEN
        ALTER TABLE commitments ADD COLUMN id BIGSERIAL PRIMARY KEY;
    END IF;
END$$;
ALTER TABLE commitments ADD COLUMN IF NOT EXISTS onchain_submitted BOOLEAN NOT NULL DEFAULT false;
UPDATE commitments SET onchain_submitted = false WHERE onchain_submitted IS NULL;
-- Backfill legacy rows to avoid duplicate identity_secret = '' when adding unique index
UPDATE commitments SET identity_secret = commitment WHERE identity_secret IS NULL OR identity_secret = '';
DELETE FROM commitments c
USING (
    SELECT ctid, ROW_NUMBER() OVER (PARTITION BY poll_id, identity_secret ORDER BY recorded_at DESC, id DESC) AS rn
    FROM commitments
) d
WHERE c.ctid = d.ctid AND d.rn > 1;
DROP INDEX IF EXISTS commitments_poll_commitment_idx;
CREATE INDEX IF NOT EXISTS commitments_poll_commitment_idx ON commitments(poll_id, commitment);
CREATE UNIQUE INDEX IF NOT EXISTS commitments_poll_identity_idx ON commitments(poll_id, identity_secret);

CREATE TABLE IF NOT EXISTS votes (
    id SERIAL PRIMARY KEY,
    poll_id BIGINT NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    nullifier TEXT NOT NULL,
    choice SMALLINT NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX IF NOT EXISTS votes_poll_nullifier_idx ON votes(poll_id, nullifier);

CREATE TABLE IF NOT EXISTS poll_members (
    poll_id BIGINT NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    identity_secret TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(poll_id, identity_secret)
);
