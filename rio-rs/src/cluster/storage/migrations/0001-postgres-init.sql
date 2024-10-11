--  Membership
CREATE TABLE IF NOT EXISTS cluster_provider_members
   (
       ip              TEXT                NOT NULL,
       port            TEXT                NOT NULL,
       last_seen       TIMESTAMPTZ         NOT NULL,
       active          BOOLEAN             NOT NULL DEFAULT FALSE,
       PRIMARY KEY (ip, port)
   );
CREATE INDEX IF NOT EXISTS idx_cluster_provider_members_last_seen on cluster_provider_members(last_seen);
CREATE INDEX IF NOT EXISTS idx_cluster_provider_members_active on cluster_provider_members(active);

-- Clustering check failures
CREATE TABLE IF NOT EXISTS cluster_provider_member_failures
   (
       id              SERIAL PRIMARY KEY,
       ip              TEXT                              NOT NULL,
       port            TEXT                              NOT NULL,
       time            TIMESTAMPTZ                       NOT NULL default CURRENT_TIMESTAMP
   );
CREATE INDEX IF NOT EXISTS idx_cluster_provider_member_failures_time ON cluster_provider_member_failures(time);
CREATE INDEX IF NOT EXISTS idx_cluster_provider_member_failures_ip_port ON cluster_provider_member_failures(ip, port);
