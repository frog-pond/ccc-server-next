CREATE TABLE IF NOT EXISTS fixture_records (
    name TEXT PRIMARY KEY,
    payload TEXT NOT NULL CHECK (json_valid(payload))
) STRICT;

-- The prototype uses a new table for the checked format. Existing unchecked
-- reply bytes from the earlier local schema are never served.
CREATE TABLE IF NOT EXISTS fixture_cache_entry (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    last_read_ms INTEGER NOT NULL,
    value_started_ms INTEGER,
    value_payload TEXT,
    failure_count INTEGER,
    backoff_until_ms INTEGER,
    failure_payload TEXT,
    CHECK ((value_started_ms IS NULL) = (value_payload IS NULL)),
    CHECK ((failure_count IS NULL) = (backoff_until_ms IS NULL)
        AND (failure_count IS NULL) = (failure_payload IS NULL)),
    -- Compare with the canonical JSON layout written by ObjectReply. The
    -- equality checks also reject missing, extra, and duplicate fields.
    CONSTRAINT completed_value CHECK (
        value_payload IS NULL OR CASE WHEN json_valid(value_payload) THEN
            json_type(value_payload, '$.result.value') IS 'integer'
            AND json_extract(value_payload, '$.result.value') BETWEEN 0 AND 4294967295
            AND value_payload IS json_object(
                'state', 'complete', 'result',
                json_object('outcome', 'value', 'value', json_extract(value_payload, '$.result.value'))
            )
        ELSE 0 END
    ),
    CONSTRAINT completed_failure CHECK (
        failure_payload IS NULL OR CASE WHEN json_valid(failure_payload) THEN
            CASE json_extract(failure_payload, '$.result.failure.kind')
                WHEN 'http_status' THEN
                    json_type(failure_payload, '$.result.failure.status') IS 'integer'
                    AND (json_extract(failure_payload, '$.result.failure.status') BETWEEN 100 AND 199
                        OR json_extract(failure_payload, '$.result.failure.status') BETWEEN 300 AND 599)
                    AND failure_payload IS json_object(
                        'state', 'complete', 'result', json_object(
                            'outcome', 'failed', 'failure', json_object(
                                'kind', 'http_status', 'status', json_extract(failure_payload, '$.result.failure.status')
                            )
                        )
                    )
                ELSE
                    json_extract(failure_payload, '$.result.failure.kind') IN (
                        'fetch', 'timeout', 'invalid_status', 'fixture_setup', 'read_body', 'decode', 'pipeline'
                    ) IS TRUE
                    AND failure_payload IS json_object(
                        'state', 'complete', 'result', json_object(
                            'outcome', 'failed', 'failure',
                            json_object('kind', json_extract(failure_payload, '$.result.failure.kind'))
                        )
                    )
            END
        ELSE 0 END
    )
) STRICT;
