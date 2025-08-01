# Governance Merkle Verifier Service

A self-contained Rust web service for serving Merkle proofs and leaf nodes for Solana governance voting.

## Quick Start

```bash
# Run the service
RUST_LOG=info cargo run --bin verifier-service

# The service will start on http://localhost:3000
```

## API Endpoints

- `GET /healthz` - Health check
- `GET /meta` - Metadata for most recent snapshot
- `POST /upload` - Upload and index Merkle snapshots
- `GET /voter/:voting_wallet` - Get vote and stake account summaries
- `GET /proof/vote_account/:vote_account` - Get Merkle proof for vote account
- `GET /proof/stake_account/:stake_account` - Get Merkle proof for stake account

## Development Status

This is a minimal implementation that will be expanded with:

- SQLite database integration
- Signature verification
- Merkle proof generation
- Data retention policies
- Comprehensive error handling
