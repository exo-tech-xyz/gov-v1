## Verifier Service Operator Deployment Guide (AWS EC2)

This guide walks an operator through provisioning an AWS EC2 instance and running the Verifier Service in Docker. It includes security group settings, environment variables, and verification steps.

### Prerequisites

- AWS account with permissions to create EC2 instances, Security Groups, and Elastic IPs
- An SSH key pair for access
- The operator's base58 Solana public key to authorize uploads (`OPERATOR_PUBKEY`)

### 1) Launch an EC2 instance

1. In the AWS Console, go to EC2 → Launch instance
2. Name: "verifier-service"
3. AMI: Ubuntu Server 22.04 LTS (x86_64)
4. Instance type: x86_64 class (e.g., c6a.xlarge) or similar
5. Key pair: Select or create one for SSH access
6. Network settings (Security Group):
   - Allow SSH on port 22 (Anywhere for testing; preferably restrict to your IP)
   - Allow HTTP on port 80 from Anywhere (0.0.0.0/0, ::/0)
   - If using Cloudflare proxy for rate limiting: no AWS change required. Optionally restrict inbound 80 to Cloudflare IP ranges to block direct origin hits
7. Storage: gp3 volume, at least 40 GB (headroom for growth and DB indices)
8. Launch instance

### 2) Allocate and associate an Elastic IP (recommended)

- EC2 → Elastic IPs → Allocate Elastic IP address → Associate with the instance
- This makes your server’s address stable across reboots

### 3) Connect to the instance

Use the EC2 console "Connect" button or SSH:

```bash
ssh -i /path/to/key.pem ubuntu@<EC2_PUBLIC_DNS_OR_IP>
```

### 4) Run the Verifier Service in Docker

The repository includes `verifier-service/src/scripts/setup.sh` which installs Docker, prepares the data directory, pulls the image, and starts the container. Set the environment variables in the script, copy it to the server and run it.

### 5) Verify the deployment

From the instance:

```bash
curl -i http://127.0.0.1/healthz
sudo docker ps
sudo docker logs --tail=200 verifier
```

From your workstation (replace with your public DNS/IP):

```bash
curl -i http://<EC2_PUBLIC_DNS>/healthz
```

Example public DNS: `ec2-18-221-54-191.us-east-2.compute.amazonaws.com`

### 6) Environment variables (supported)

- IMAGE: Docker image to use
- OPERATOR_PUBKEY: base58 operator public key (required)
- DB_PATH: SQLite path inside container; default `/data/governance.db`
- PORT: container listen port; default `3000` (we map host 80 → container 3000)
- RUST_LOG: e.g. `info`
- SQLITE_MAX_CONNECTIONS: default 4 for file DB
- UPLOAD_BODY_LIMIT: bytes; default 100MB
- GLOBAL_REFILL_INTERVAL, GLOBAL_RATE_BURST: request rate limiting (defaults 10/10)
- UPLOAD_REFILL_INTERVAL, UPLOAD_RATE_BURST: upload route rate limiting (defaults 60/2)

### 7) Cloudflare (TBC)

- Enable proxy on DNS (orange cloud) to route traffic through Cloudflare
- Configure Cloudflare rate limiting rules for your paths (e.g., /upload, /proof/\*)
- Optional: restrict EC2 Security Group 80/443 to Cloudflare IP ranges to block direct-to-origin
- Decide TLS mode (Full Strict recommended) and set up origin TLS (Nginx/ALB) if using HTTPS

### 8) Start the database cleanup cron

This repository includes `verifier-service/src/scripts/cleanup.sh` which installs a cron job that periodically prunes old rows from the SQLite database. Set the environment variables in the script, copy it to the server and run it.

- To check that cron is running and view logs:

```bash
# Is cron active?
systemctl is-active cron 2>/dev/null || systemctl is-active crond 2>/dev/null

# Inspect the installed entry
sudo cat /etc/cron.d/verifier-cleanup

# View cleanup logs (file appears on first cron run)
sudo tail -n 100 /var/log/verifier-cleanup.log || echo "Log not created yet; trigger a run or wait for the next schedule."

# Follow live once it exists
sudo tail -f /var/log/verifier-cleanup.log

# Cron service logs (Ubuntu/Debian)
sudo journalctl -u cron --since "1 hour ago" | tail -n 200
# Or via syslog
grep CRON /var/log/syslog | tail -n 200
```

- To trigger a cleanup immediately (same as cron runs):

```bash
DB=/srv/verifier/data/governance.db DAYS=60 SLOTS_PER_DAY=216000 /usr/bin/bash /usr/local/bin/verifier-cleanup-sql.sh
```

- Remove/kill the cleanup cron

```bash
# Remove the cron entry and reload cron
sudo rm -f /etc/cron.d/verifier-cleanup
sudo service cron reload 2>/dev/null || sudo service crond reload 2>/dev/null || true

# (Optional) remove the runner script
sudo rm -f /usr/local/bin/verifier-cleanup-sql.sh

# If a cleanup is currently running, stop it
pgrep -fa 'verifier-cleanup-sql.sh|sqlite3' | awk '{print $1}' | xargs -r sudo kill
```
