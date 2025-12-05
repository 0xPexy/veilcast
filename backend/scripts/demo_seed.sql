BEGIN;

TRUNCATE TABLE votes RESTART IDENTITY CASCADE;
TRUNCATE TABLE commitments RESTART IDENTITY CASCADE;
TRUNCATE TABLE poll_members RESTART IDENTITY CASCADE;
TRUNCATE TABLE poll_secrets RESTART IDENTITY CASCADE;
TRUNCATE TABLE polls RESTART IDENTITY CASCADE;
TRUNCATE TABLE members RESTART IDENTITY CASCADE;
TRUNCATE TABLE user_stats RESTART IDENTITY CASCADE;

ALTER SEQUENCE IF EXISTS polls_id_seq RESTART WITH 1;
ALTER SEQUENCE IF EXISTS members_id_seq RESTART WITH 1;
ALTER SEQUENCE IF EXISTS commitments_id_seq RESTART WITH 1;
ALTER SEQUENCE IF EXISTS votes_id_seq RESTART WITH 1;
ALTER SEQUENCE IF EXISTS poll_secrets_id_seq RESTART WITH 1;

INSERT INTO members (identity_secret) VALUES
('demo_identity_atlas'),
('demo_identity_meridian'),
('demo_identity_aurora'),
('demo_identity_kepler'),
('demo_identity_hikari'),
('demo_identity_nimbus');

INSERT INTO user_stats (identity_secret, username, xp, total_votes, correct_votes, tier) VALUES
('demo_identity_atlas', 'AtlasAlpha',    1080, 42, 29, 'Master Oracle'),
('demo_identity_meridian', 'MeridianWave', 860, 39, 24, 'Gold Seer'),
('demo_identity_aurora', 'AuroraSpark',   620, 33, 20, 'Gold Seer'),
('demo_identity_kepler', 'KeplerEdge',    480, 28, 16, 'Silver Sage'),
('demo_identity_hikari', 'HikariVision',  260, 19,  9, 'Bronze Adept'),
('demo_identity_nimbus', 'NimbusOracle',  140, 12,  5, 'Apprentice');

INSERT INTO polls (id, question, options, commit_phase_end, reveal_phase_end, category, membership_root, owner, reveal_tx_hash, correct_option, resolved, commit_sync_completed)
VALUES
(1, 'By late September 2025, was Nvidia the world''s largest public company by market cap?',
 '[ "Yes, Nvidia led the pack", "No, Microsoft/Apple stayed ahead" ]'::jsonb,
 NOW() - INTERVAL '42 days', NOW() - INTERVAL '41 days',
 'Markets', '0xveil-nvdaTop25', 'demo-admin', '0xdemo-reveal-nvdaTop25', 0, true, true),
(2, 'Did Apple touch a $4T market cap in late October 2025?',
 '[ "Yes, $4T was hit", "No, stayed below $4T" ]'::jsonb,
 NOW() - INTERVAL '38 days', NOW() - INTERVAL '37 days',
 'Markets', '0xveil-aapl4t25', 'demo-admin', '0xdemo-reveal-aapl4t25', 0, true, true),
(3, 'In October 2025, did Bitcoin set a new ATH above $126,000?',
 '[ "Yes, > $126k", "No, never cleared $126k" ]'::jsonb,
 NOW() - INTERVAL '34 days', NOW() - INTERVAL '33 days',
 'Crypto', '0xveil-btcATH25', 'demo-admin', '0xdemo-reveal-btcATH25', 0, true, true),
(4, 'By late November 2025, had Bitcoin fallen back below $100K?',
 '[ "Yes, trading sub-$100k", "No, still above $100k" ]'::jsonb,
 NOW() - INTERVAL '30 days', NOW() - INTERVAL '29 days',
 'Crypto', '0xveil-btcSub100k', 'demo-admin', '0xdemo-reveal-btcSub100k', 0, true, true),
(5, 'Had the first U.S. spot Solana ETF launched and raised $400M+ by October 2025?',
 '[ "Yes, Bitwise BSOL hit $400M+", "No, still no major SOL ETF" ]'::jsonb,
 NOW() - INTERVAL '26 days', NOW() - INTERVAL '25 days',
 'Crypto', '0xveil-solETF25', 'demo-admin', '0xdemo-reveal-solETF25', 0, true, true),
(6, 'By December 2025, could BoA wealth advisers recommend crypto ETP allocations for 2026?',
 '[ "Yes, recommendation allowed", "No, execution-only access" ]'::jsonb,
 NOW() - INTERVAL '22 days', NOW() - INTERVAL '21 days',
 'Finance', '0xveil-bacCrypto', 'demo-admin', '0xdemo-reveal-bacCrypto', 0, true, true),
(7, 'Did the LA Dodgers defeat the Toronto Blue Jays in seven games to win the 2025 World Series?',
 '[ "Yes, Dodgers in 7", "No, Blue Jays (or others) won" ]'::jsonb,
 NOW() - INTERVAL '18 days', NOW() - INTERVAL '17 days',
 'Sports', '0xveil-ws2025LAD', 'demo-admin', '0xdemo-reveal-ws2025LAD', 0, true, true);

INSERT INTO poll_members (poll_id, identity_secret)
SELECT p.poll_id, u.identity_secret
FROM (VALUES (1),(2),(3),(4),(5),(6),(7)) AS p(poll_id)
CROSS JOIN (VALUES
    ('demo_identity_atlas'),
    ('demo_identity_meridian'),
    ('demo_identity_aurora'),
    ('demo_identity_kepler'),
    ('demo_identity_hikari'),
    ('demo_identity_nimbus')
) AS u(identity_secret);

DROP TABLE IF EXISTS demo_prepared;
CREATE TEMP TABLE demo_prepared AS
WITH demo_votes(poll_id, identity_secret, choice) AS (
    VALUES
        (1, 'demo_identity_atlas',   0),
        (1, 'demo_identity_meridian',0),
        (1, 'demo_identity_aurora',  0),
        (1, 'demo_identity_kepler',  1),
        (1, 'demo_identity_hikari',  0),
        (1, 'demo_identity_nimbus',  0),
        (2, 'demo_identity_atlas',   0),
        (2, 'demo_identity_meridian',0),
        (2, 'demo_identity_aurora',  0),
        (2, 'demo_identity_kepler',  1),
        (2, 'demo_identity_hikari',  0),
        (2, 'demo_identity_nimbus',  0),
        (3, 'demo_identity_atlas',   0),
        (3, 'demo_identity_meridian',0),
        (3, 'demo_identity_aurora',  0),
        (3, 'demo_identity_kepler',  1),
        (3, 'demo_identity_hikari',  0),
        (3, 'demo_identity_nimbus',  0),
        (4, 'demo_identity_atlas',   0),
        (4, 'demo_identity_meridian',0),
        (4, 'demo_identity_aurora',  0),
        (4, 'demo_identity_kepler',  1),
        (4, 'demo_identity_hikari',  0),
        (4, 'demo_identity_nimbus',  0),
        (5, 'demo_identity_atlas',   0),
        (5, 'demo_identity_meridian',0),
        (5, 'demo_identity_aurora',  0),
        (5, 'demo_identity_kepler',  1),
        (5, 'demo_identity_hikari',  0),
        (5, 'demo_identity_nimbus',  0),
        (6, 'demo_identity_atlas',   0),
        (6, 'demo_identity_meridian',0),
        (6, 'demo_identity_aurora',  0),
        (6, 'demo_identity_kepler',  1),
        (6, 'demo_identity_hikari',  0),
        (6, 'demo_identity_nimbus',  0),
        (7, 'demo_identity_atlas',   0),
        (7, 'demo_identity_meridian',0),
        (7, 'demo_identity_aurora',  0),
        (7, 'demo_identity_kepler',  1),
        (7, 'demo_identity_hikari',  0),
        (7, 'demo_identity_nimbus',  0)
)
    SELECT
        dv.poll_id,
        dv.identity_secret,
        dv.choice,
        format('0xcommit-%s-%s', dv.poll_id, dv.identity_secret) AS commitment,
        format('0xnullifier-%s-%s', dv.poll_id, dv.identity_secret) AS nullifier,
        format('0xproof-%s-%s', dv.poll_id, dv.identity_secret) AS proof,
        format(
            '%s%s',
            dv.poll_id * 1000,
            lpad((ROW_NUMBER() OVER (PARTITION BY dv.poll_id ORDER BY dv.identity_secret))::text, 2, '0')
        ) AS secret
    FROM demo_votes dv;

INSERT INTO commitments (poll_id, choice, commitment, identity_secret, secret, nullifier, proof, public_inputs, onchain_submitted)
SELECT
    poll_id,
    choice,
    commitment,
    identity_secret,
    secret,
    nullifier,
    proof,
    ARRAY[
        choice::text,
        commitment,
        nullifier
    ],
    true
FROM demo_prepared;

INSERT INTO votes (poll_id, nullifier, choice)
SELECT poll_id, nullifier, choice FROM demo_prepared;

INSERT INTO poll_secrets (poll_id, identity_secret, secret)
SELECT poll_id, identity_secret, secret FROM demo_prepared
ON CONFLICT (poll_id, identity_secret) DO UPDATE SET secret = EXCLUDED.secret;

SELECT setval('polls_id_seq',        COALESCE((SELECT MAX(id) FROM polls), 0),        true);
SELECT setval('members_id_seq',      COALESCE((SELECT MAX(id) FROM members), 0),      true);
SELECT setval('commitments_id_seq',  COALESCE((SELECT MAX(id) FROM commitments), 0),  true);
SELECT setval('votes_id_seq',        COALESCE((SELECT MAX(id) FROM votes), 0),        true);
SELECT setval('poll_secrets_id_seq', COALESCE((SELECT MAX(id) FROM poll_secrets), 0), true);

COMMIT;
