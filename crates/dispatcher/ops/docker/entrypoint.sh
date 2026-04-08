#!/bin/bash
set -e

cd /var/app/crates/dispatcher

echo "Running dispatcher migrations..."
sqlx migrate run --source ./db/migrations --database-url "mysql://${MYSQL_USER}:${MYSQL_PASSWORD}@${MYSQL_HOST}:${MYSQL_PORT}/process_dispatcher"

echo "Running mvp migrations..."
sqlx migrate run --source ./db/mvp_migrations --database-url "mysql://${MYSQL_USER}:${MYSQL_PASSWORD}@${MYSQL_HOST}:${MYSQL_PORT}/mvp"

echo "Starting dispatcher..."
cd /var/app
exec cargo run --bin process_dispatcher
