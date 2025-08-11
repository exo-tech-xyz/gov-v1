# Governance Merkle Verifier Service

A self-contained Rust web service for serving Merkle proofs and leaf nodes for Solana governance voting.

## Quick Start

```bash
# Set the operator public key for signature verification (replace with your own)
export OPERATOR_PUBKEY="C5m2XDwZmjc7yHpy8N4KhQtFJLszasVpfB4c5MTuCsmg"

# Run the service
RUST_LOG=info cargo run --bin verifier-service

# Optional: Run with custom database path
DB_PATH="./data/governance.db" RUST_LOG=info cargo run --bin verifier-service

# Optional: Run with in-memory database (for testing)
DB_PATH=":memory:" RUST_LOG=info cargo run --bin verifier-service

# The service will start on http://localhost:3000
```

## API Endpoints

- `POST /upload` - Upload and index Merkle snapshots
- `GET /healthz` - Health check
- `GET /meta` - Metadata for most recent snapshot
- `GET /voter/:voting_wallet` - Get vote and stake account summaries
- `GET /proof/vote_account/:vote_account` - Get Merkle proof for vote account
- `GET /proof/stake_account/:stake_account` - Get Merkle proof for stake account

## Security

### Signature Verification

The `/upload` endpoint requires Ed25519 signature verification to prevent unauthorized snapshot uploads:

- **Environment Variable**: Set `OPERATOR_PUBKEY` to the base58-encoded public key of the authorized operator
- **Message Format**: Signatures are verified over `slot.to_le_bytes() || merkle_root_bs58_string.as_bytes()`
- **Signature Format**: Base58-encoded Ed25519 signature

## Testing

### Running Tests

```bash
# Run all tests with required environment variables
RESTAKING_PROGRAM_ID=11111111111111111111111111111111 \
VAULT_PROGRAM_ID=11111111111111111111111111111111 \
TIP_ROUTER_PROGRAM_ID=11111111111111111111111111111111 \
cargo test --bin verifier-service
```

## Docker (runtime-only, prebuilt binary)

```bash
# 1) Build the binary locally
cargo build --release --bin verifier-service

# 2) Build a minimal runtime image (copies the binary only)
docker build -f verifier-service/Dockerfile -t verifier-service:local .

# 3) Run the container (persists DB to ./data)
docker run --rm -p 3000:3000 \
  -e OPERATOR_PUBKEY="$OPERATOR_PUBKEY" \
  -e RUST_LOG=info \
  -v $(pwd)/data:/data \
  verifier-service:local

# Health check
curl -s http://localhost:3000/healthz
```

Environment variables:

- OPERATOR_PUBKEY (required)
- DB_PATH (optional, defaults to /data/governance.db inside container)
- PORT (optional, defaults to 3000)

<!-- TODO: Add docker-compose for dev convenience -->
<!-- TODO: Add Docker HEALTHCHECK using /healthz -->

**Note**: Tests use `serial_test` to run sequentially due to shared environment variable usage.

### Upload a Snapshot

To test the upload endpoint with a snapshot (replace fields with actual values):

```bash
curl -X POST http://localhost:3000/upload \
  -F "slot=340850340" \
  -F "network=testnet" \
  -F "merkle_root=8oaP5t8E6GEMVE19NFbCNAUxQ7GZe6q8c6XVWvgBgs5p" \
  -F "signature=43P1z1o7zXQbK3ocFUVrwGmg1bS8kdwh1tg4FNKJ2f2UDaEZ6pgvwLyMEf2qcXqf2vZ2RrPg9zJAM6pddV645Q2" \
  -F "file=@meta_merkle-340850340.zip" \
  -w "\nHTTP Status: %{http_code}\n" \
  -s
```

### Get Metadata

```bash
curl http://localhost:3000/meta?network=testnet
```

Example response:

```json
{
  "network": "testnet",
  "slot": 340850340,
  "merkle_root": "8oaP5t8E6GEMVE19NFbCNAUxQ7GZe6q8c6XVWvgBgs5p",
  "snapshot_hash": "2ejpKvga5pGMyQGhmi59U6PThwKFzLy8SAjxt5yG8raH",
  "created_at": "2025-08-05T16:17:25.855006+00:00"
}
```

### Get Voter Summary

```bash
curl -i http://localhost:3000/voter/9w7BxC28QqDqCuKSPYVwDi1GeNvrXKhMKUuFzF2T3eUr?network=testnet
```

Example response:

```json
{
  "network": "testnet",
  "snapshot_slot": 340850340,
  "stake_accounts": [
    {
      "active_stake": 9997717120,
      "stake_account": "DXmtAZdYsVZT8ir8uPkuY4cgBtsxWpZU4QKdpcAbFngo",
      "vote_account": "1vgZrjS88D7RA1CbcSAovvyd6cSVqk3Ag1Ty2kSrJVd"
    }
  ],
  "vote_accounts": [
    {
      "active_stake": 32615703997228,
      "vote_account": "1vgZrjS88D7RA1CbcSAovvyd6cSVqk3Ag1Ty2kSrJVd"
    }
  ],
  "voting_wallet": "9w7BxC28QqDqCuKSPYVwDi1GeNvrXKhMKUuFzF2T3eUr"
}
```

### Get Vote Proof

```bash
curl -i http://localhost:3000/proof/vote_account/1vgZrjS88D7RA1CbcSAovvyd6cSVqk3Ag1Ty2kSrJVd?network=testnet&slot=340850340
```

Example response:

```json
{
  "network": "testnet",
  "snapshot_slot": 340850340
  "meta_merkle_leaf": {
    "active_stake": 32615703997228,
    "stake_merkle_root": "FcBZ89hYQpb5aYcQeBvnBN8dRHoWsV2FdWdVVE369jw7",
    "vote_account": "1vgZrjS88D7RA1CbcSAovvyd6cSVqk3Ag1Ty2kSrJVd",
    "voting_wallet": "9w7BxC28QqDqCuKSPYVwDi1GeNvrXKhMKUuFzF2T3eUr"
  },
  "meta_merkle_proof": [
    "5MQsFvce5HZbiAa6ZFckbaDjw9834ZhGPVnW8jpmTF2F",
    "FUymD22xJuTyUSm3Rvi4sEzZf5PCAyc183hJ8QGS6PGA",
    "5ocvKp3VR4hrt2yFGiJMXHjwJcZg3ku7WMX2V3zBEC7u",
    "Ht8zCJdynqNS5AFgiMXGv97MSEc1V5bvU2B68uQWZb5Z",
    "2eqx4VuW5vaiDnvfhePW8taCa5AuVpViBKKRmQHbrwtN",
    "7FdjixP6zEFdFHitme2yPhiX1ERwJW257MKAhyob5UKm",
    "39nmdZYqWKMWzhET9tTq5xfWEeDthf69PJHrFPV9R1Ta",
    "BCBq2GRwdmdaVBEvGb6K4BM4uDtgPuaDcHpoekzH5chx",
    "BsKrjCspup6KtKjYe4bbxd2JHmuXEKdF2Efr2q1gHUzh",
    "CPPSFu2AZb7yFSJHcXenrYp6BzLNfeTB6iKtMxYx5cQw",
    "G5U1aG9EKRMX4znZc47LE1rdrJmJ8g5aCNydFRDSsxgf",
    "DRoQrDBYKLPWYUPEg6vvDEri5hHNhNE2bmejLMHkmNik"
  ],
}
```

### Get Stake Proof

```bash
curl -i http://localhost:3000/proof/stake_account/DXmtAZdYsVZT8ir8uPkuY4cgBtsxWpZU4QKdpcAbFngo?network=testnet
```

Example response:

```json
{
  "network": "testnet",
  "snapshot_slot": 340850340,
  "stake_merkle_leaf": {
    "active_stake": 9997717120,
    "stake_account": "DXmtAZdYsVZT8ir8uPkuY4cgBtsxWpZU4QKdpcAbFngo",
    "voting_wallet": "9w7BxC28QqDqCuKSPYVwDi1GeNvrXKhMKUuFzF2T3eUr"
  },
  "stake_merkle_proof": [
    "Gu8E91fBN2XeJECWpmxCH8gnx4zmsBor1ewWWGHyA375",
    "468mq67yo9svHuQK9bqYx71E2opDaJtkqsEEBKEt1Bvr"
  ],
  "vote_account": "1vgZrjS88D7RA1CbcSAovvyd6cSVqk3Ag1Ty2kSrJVd"
}
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
curl -i http://localhost:3000/healthz
```

## Dependencies

### Key Crates

- `axum` - Web framework
- `solana-sdk` - Solana blockchain SDK for signature verification
- `rusqlite` - SQLite database interface
- `serial_test` - Sequential test execution for environment variable isolation
- `anyhow` - Error handling

```

```
