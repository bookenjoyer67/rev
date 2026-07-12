#!/bin/sh
# Komun setup script for Alpine Linux
# Run as root on the target server
set -e

KOMUN_USER="komun"
KOMUN_HOME="/opt/komun"
BINARY="./komun-server"
CONFIG="./config.toml"
INITD="./deploy/komun.initd"
NGINX_CONF="./deploy/nginx-komun.conf"

echo "=== Setting up Komun on Alpine ==="

# 1. Create user and directories
echo "Creating user and directories..."
addgroup -S $KOMUN_USER 2>/dev/null || true
adduser -S -s /bin/false -h $KOMUN_HOME -H -G $KOMUN_USER $KOMUN_USER 2>/dev/null || true
mkdir -p $KOMUN_HOME/data/avatars $KOMUN_HOME/data/post-images $KOMUN_HOME/data/relay
chown -R $KOMUN_USER:$KOMUN_USER $KOMUN_HOME

# 2. Create PostgreSQL database
echo "Setting up database..."
su -s /bin/sh - postgres -c "psql -c \"CREATE USER komun WITH PASSWORD 'komun-dev';\"" 2>/dev/null || true
su -s /bin/sh - postgres -c "psql -c \"CREATE DATABASE komun OWNER komun;\"" 2>/dev/null || true
su -s /bin/sh - postgres -c "psql -d komun -c \"GRANT ALL ON SCHEMA public TO komun;\""
su -s /bin/sh - postgres -c "psql -d komun -c \"GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO komun;\""
su -s /bin/sh - postgres -c "psql -d komun -c \"GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO komun;\""

# 3. Run migrations
echo "Running migrations..."
for f in migrations/*.sql; do
    su -s /bin/sh - postgres -c "psql -d komun -f $f"
done

# 4. Deploy binary
echo "Deploying binary..."
cp "$BINARY" $KOMUN_HOME/
cp "$CONFIG" $KOMUN_HOME/
chmod +x $KOMUN_HOME/komun-server

# 5. Install service
echo "Installing service..."
cp "$INITD" /etc/init.d/komun
chmod +x /etc/init.d/komun
rc-update add komun 2>/dev/null || true

# 6. Nginx config
if [ -f /etc/nginx/http.d/default.conf ]; then
    cp "$NGINX_CONF" /etc/nginx/http.d/komun.conf
    echo "Nginx config installed. Restart nginx to apply."
fi

echo ""
echo "=== Setup complete ==="
echo "Start with:  rc-service komun start"
echo "Check logs:  tail -f /var/log/komun.log"
echo "Health:      curl http://localhost:3001/api/health"
