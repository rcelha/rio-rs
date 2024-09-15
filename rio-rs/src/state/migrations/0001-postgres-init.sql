CREATE TABLE IF NOT EXISTS state_provider_object_state
(
    object_kind       TEXT                              NOT NULL,
    object_id         TEXT                              NOT NULL,
    state_type        TEXT                              NOT NULL,
    serialized_state  bytea                             NOT NULL,
    PRIMARY KEY (object_kind, object_id, state_type)
);

