CREATE TABLE IF NOT EXISTS object_placement
(
    struct_name     TEXT                NOT NULL,
    object_id       TEXT                NOT NULL,
    server_address  TEXT                NULL,

    PRIMARY KEY (struct_name, object_id)
);
CREATE INDEX IF NOT EXISTS idx_object_placement_server_address on object_placement(server_address);
