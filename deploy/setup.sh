#!/bin/sh
# Komun setup script for Alpine Linux (OpenRC). Run as root on the target server.
# See docs/DEVELOPMENT.md (provisioning) and docs/DEPLOY.md for the full walkthrough.
set -e

KOMUN_USER="komun"
KOMUN_HOME="/opt/komun"
DB_NAME="${DB_NAME:-komun}"
DB_USER="${DB_USER:-komun}"
DB_PASSWORD="${DB_PASSWORD:-change-me}"
BINARY="./komun-server"
CONFIG="./config.toml"
INITD="./deploy/komun.initd"
NGINX_CONF="./deploy/nginx-komun.conf"

echo "=== Setting up Komun on Alpine ==="

# 1. User and directories (there is no relay directory any more).
addgroup -S "$KOMUN_USER" 2>/dev/null || true
adduser -S -s /bin/false -h "$KOMUN_HOME" -H -G "$KOMUN_USER" "$KOMUN_USER" 2>/dev/null || true
mkdir -p "$KOMUN_HOME/data/avatars" "$KOMUN_HOME/data/post-images"
chown -R "$KOMUN_USER:$KOMUN_USER" "$KOMUN_HOME"

# 2. Database and role.
echo "Setting up database..."
su -s /bin/sh - postgres -c "psql -c \"CREATE USER $DB_USER WITH PASSWORD '$DB_PASSWORD';\"" 2>/dev/null || true
su -s /bin/sh - postgres -c "psql -c \"CREATE DATABASE $DB_NAME OWNER $DB_USER;\"" 2>/dev/null || true

# 3. Provision the schema in the documented order: create the bookkeeping table, load
#    001_schema.sql by hand, bookmark it with its real sha384, then boot once so the
#    migrator applies 002 and later. NEVER edit 001_schema.sql.
echo "Provisioning schema..."
su -s /bin/sh - postgres -c "psql -d $DB_NAME" <<'SQL'
CREATE TABLE IF NOT EXISTS _sqlx_migrations (
    version        BIGINT PRIMARY KEY,
    description    TEXT NOT NULL,
    installed_on   TIMESTAMPTZ NOT NULL DEFAULT now(),
    success        BOOLEAN NOT NULL,
    checksum       BYTEA NOT NULL,
    execution_time BIGINT NOT NULL
);
SQL
su -s /bin/sh - postgres -c "psql -d $DB_NAME -f migrations/001_schema.sql"
CHECKSUM=$(sha384sum migrations/001_schema.sql | cut -d' ' -f1)
su -s /bin/sh - postgres -c "psql -d $DB_NAME -c \"INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) VALUES (1, 'schema', true, decode('$CHECKSUM','hex'), 0);\""

# 4. Deploy binary and config.
echo "Deploying binary..."
cp "$BINARY" "$KOMUN_HOME/"
cp "$CONFIG" "$KOMUN_HOME/"
chmod +x "$KOMUN_HOME/komun-server"
chown "$KOMUN_USER:$KOMUN_USER" "$KOMUN_HOME/komun-server" "$KOMUN_HOME/config.toml"

# 5. Install service.
echo "Installing service..."
cp "$INITD" /etc/init.d/komun
chmod +x /etc/init.d/komun
rc-update add komun 2>/dev/null || true

# 6. Nginx config.
if [ -f /etc/nginx/http.d/default.conf ]; then
    cp "$NGINX_CONF" /etc/nginx/http.d/komun.conf
    echo "Nginx config installed. Edit server_name + TLS, then restart nginx."
fi

echo ""
echo "=== Setup complete ==="
echo "1. Edit $KOMUN_HOME/config.toml ([database] url and [node])."
echo "2. rc-service komun start   # the migrator applies migrations 002+ on first boot"
echo "Health: curl http://localhost:3000/api/health"
