# 0) Modify environment variables for your server
# Setup Env
IMAGE="username/verifier-service:latest" 
OPERATOR_PUBKEY="C5m2XDwZmjc7yHpy8N4KhQtFJLszasVpfB4c5MTuCsmg" 
PORT_HOST=80
PORT_CONTAINER=3000
DATA_DIR=/srv/verifier/data
DB_PATH=/data/governance.db

# Service Env
GLOBAL_REFILL_INTERVAL=10
GLOBAL_RATE_BURST=10
UPLOAD_REFILL_INTERVAL=60
UPLOAD_RATE_BURST=2
# Upload body size limit (bytes)
UPLOAD_BODY_LIMIT=$((100 * 1024 * 1024)) # 100MB
# SQLite pool size
SQLITE_MAX_CONNECTIONS=4

# 1) Install Docker
sudo apt-get update
sudo apt-get install -y docker.io ca-certificates
sudo systemctl enable --now docker

# 2) Prepare persistent state dir (UID 10001 matches your Dockerfile USER)
sudo mkdir -p "$(dirname "$DATA_DIR")"
sudo mkdir -p "$DATA_DIR"
sudo chown -R 10001:10001 /srv/verifier

# 3) Pull (optional but nice to see errors early)
sudo docker pull "$IMAGE"

# 4) Re-create container idempotently, then run (daemonized, restarts on reboot/crash)
# Stop and remove existing container if it exists
sudo docker rm -f verifier >/dev/null 2>&1 || true

sudo docker run -d --name verifier --restart unless-stopped \
  -p ${PORT_HOST}:${PORT_CONTAINER} \
  -e OPERATOR_PUBKEY="${OPERATOR_PUBKEY}" \
  -e DB_PATH="${DB_PATH}" \
  -e PORT="${PORT_CONTAINER}" \
  -e RUST_LOG=info \
  -e GLOBAL_REFILL_INTERVAL="${GLOBAL_REFILL_INTERVAL}" \
  -e GLOBAL_RATE_BURST="${GLOBAL_RATE_BURST}" \
  -e UPLOAD_REFILL_INTERVAL="${UPLOAD_REFILL_INTERVAL}" \
  -e UPLOAD_RATE_BURST="${UPLOAD_RATE_BURST}" \
  -e UPLOAD_BODY_LIMIT="${UPLOAD_BODY_LIMIT}" \
  -e SQLITE_MAX_CONNECTIONS="${SQLITE_MAX_CONNECTIONS}" \
  -v ${DATA_DIR}:/data \
  "${IMAGE}"

# 5) Verify
sudo docker ps
curl -fsS "http://127.0.0.1:${PORT_HOST}/healthz" || sudo docker logs --tail=200 verifier