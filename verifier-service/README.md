# Governance Merkle Verifier Service

A self-contained Rust web service for serving Merkle proofs and leaf nodes for Solana governance voting.

## Quick Start

```bash
# Run the service
RUST_LOG=info cargo run --bin verifier-service

# Run with custom database path
DB_PATH="./data/governance.db" RUST_LOG=info cargo run --bin verifier-service

# Run with in-memory database (for testing)
DB_PATH=":memory:" RUST_LOG=info cargo run --bin verifier-service

# The service will start on http://localhost:3000
```

## API Endpoints

- `GET /healthz` - Health check
- `GET /meta` - Metadata for most recent snapshot
- `POST /upload` - Upload and index Merkle snapshots
- `GET /voter/:voting_wallet` - Get vote and stake account summaries
- `GET /proof/vote_account/:vote_account` - Get Merkle proof for vote account
- `GET /proof/stake_account/:stake_account` - Get Merkle proof for stake account

## Testing

### Upload a Snapshot

To test the upload endpoint with a snapshot (replace fields with actual values)

```bash
curl -X POST http://localhost:3000/upload \
  -F "slot=340850340" \
  -F "network=testnet" \
  -F "merkle_root=8oaP5t8E6GEMVE19NFbCNAUxQ7GZe6q8c6XVWvgBgs5p" \
  -F "signature=1111111111111111111111111111111111111111111111111111111111111111111111111111111111111111" \
  -F "file=@meta_merkle-340850340.zip" \
  -w "\nHTTP Status: %{http_code}\n" \
  -s
```

### Check SQL Database

```bash
sqlite3 ./data/governance.db

# List tables
.tables

# List rows in snapshot_meta
select * from snapshot_meta limit 10;

# List rows in vote_accounts
select * from vote_accounts limit 10;

# List rows in stake_accounts
select * from stake_accounts limit 10;
```

### Health Check

```bash
curl http://localhost:3000/healthz
```

## Development Status

This is a minimal implementation that will be expanded with:

- Signature verification
- Merkle proof generation
- Data retention policies
- Comprehensive error handling
