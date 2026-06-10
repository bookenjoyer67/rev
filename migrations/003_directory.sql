CREATE TABLE directory_entries (
    url TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    location_name TEXT,
    location_lat DOUBLE PRECISION,
    location_lon DOUBLE PRECISION,
    communities_count BIGINT DEFAULT 0,
    version TEXT,
    last_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
    registered_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_directory_location ON directory_entries(location_lat, location_lon)
    WHERE location_lat IS NOT NULL AND location_lon IS NOT NULL;
