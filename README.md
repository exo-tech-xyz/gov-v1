# Solana Governance Voter Snapshot

This repo contains:

- `cli/`: A command-line tool for Operators to generate stake snapshots and vote on-chain.
- `programs/gov-v1/`: The Anchor-based on-chain program used to coordinate Operator voting and finalize snapshot consensus.

[→ Governance Voter Snapshot Program Design](programs/gov-v1/README.md)
[→ Verifier Service README](verifier-service/README.md)
[→ Verifier Service Deployment](verifier-service/DEPLOYMENT.md)

---

## Table of Contents

- [Project Structure](#project-structure)
- [Stake Pool Handling](#stake-pool-handling)
- [Vote Account](#vote-account)
  - [Stake Calculation](#stake-calculation)
  - [Missing Vote Account](#missing-vote-account)
- [Testing](#testing)
- [CLI Usage](#cli-usage-via-cargo-run)
  - [Program Setup](#program-setup-after-deployment)
  - [Snapshot Handling](#snapshot-handling)
  - [Log On-Chain State](#log-on-chain-state)
  - [Voting Flow](#voting-flow)
  - [Finalization & Tie-Breaking](#finalization--tie-breaking)
- [Troubleshooting](#troubleshooting)
- [Additional Testing Commands](#additional-testing-commands)

---

## Project Structure

```
.
├── cli/                  # CLI tool for snapshot ops & voting
├── programs/
    └── gov-v1/           # On-chain governance snapshot program
└── tests/                # Anchor program integration tests
```

---

## Stake Pool Handling

The governance snapshot system handles stake accounts delegated by stake pools by changing the voting wallet from withdraw authorities (typically PDAs) to appropriate voting wallets to enable stake pool operators to participate in governance on behalf of their delegated stake.

### SPL Stake Pool Program

For stake accounts delegated through the **SPL Stake Pool program**, the system changes the voter from the withdraw authority (which is a PDA) to the **manager authority**.

### Marinade Liquid Staking Program

For stake accounts delegated through the **Marinade Liquid Staking Program**, the system changes the voter from the withdraw authority (which is a PDA) to the **operations wallet authority** (`opLSF7LdfyWNBby5o6FT8UFsr2A4UGKteECgtLSYrSm`).

### Sanctum Pools

For **Sanctum Pools**, since stake is either delegated to a single validator per LST or distributed with majority stake to a few validators, the LST operators are already able to vote through the validator itself. Therefore, the system keeps the voter for stake accounts as the withdraw authority (which will not be able to vote since it's a PDA). This approach recognizes that Sanctum's model already provides governance participation through validator-level voting.

### Individual Stake Accounts

For individual stake accounts not managed by any stake pool program, the system uses the withdraw authority directly as the voting wallet, allowing individual stakers to participate in governance.

---

## Vote Account

### Stake Calculation

Vote account effective stake is calculated by summing the individual active stake accounts delegated to the vote account that reads from the Bank's StakesCache. This bottom up approach differs from using the value record in Bank's `epoch_stakes` computed at epoch boundary.

### Missing Vote Account

If a vote account delegated to is missing (closed by the manager), the system will set the voting wallet to the default address `11111111111111111111111111111111`. This implies that the delegators can continue to vote, but the vote account will not be able to vote.

---

## Dependencies

1. Clone `jito-tip-router` to parent directory and switch to `6d0d8244314ff7c04625b531f033b770a8c7aafc` commit.
2. In the cloned repo, modify references of `branch = "v2.1-upgrade"` (which no longer exists) to `rev = "358fbc3c20d947c977a136808f9fbf7f070e478b"` in `Cargo.lock` and `Cargo.toml`.
3. Ensure system is using Rust Version `1.86.0`, otherwise install with:

```bash
rustup toolchain install 1.86.0 // install
rustup default 1.86.0 // set as default
rustc --version // verify version
```

4. Build repo with `cargo build`

---

## Testing

Anchor tests can be executed directly from the root directory with `anchor test` which spins up a local validator. Note that setup of env variables is required.

---

## CLI Usage (via cargo run)

All commands assume:

- You're running from project root using `RUST_LOG=info cargo run --bin cli -- ...`
  - `--payer-path` signs transactions
  - `--authority-path` signs Operator votes
- Replace `~/.config/solana/id.json` with path to keypair file
- Replace `key1,key2,key3...` with actual base58-encode pubkeys

Use `RUST_LOG=info` to enable logs.

Setup env variables:

```bash
export RESTAKING_PROGRAM_ID=RestkWeAVL8fRGgzhfeoqFhsqKRchg6aa1XrcH96z4Q
export VAULT_PROGRAM_ID=Vau1t6sLNxnzB7ZDsef8TLbPLfyZMYXH8WTNqUdm9g8
export TIP_ROUTER_PROGRAM_ID=11111111111111111111111111111111
```

---

### Program Setup (after deployment)

```bash
# Initialize ProgramConfig global singleton on-chain
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  init-program-config

# Add or remove operators from whitelist
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  update-operator-whitelist -a key1,key2,key3 -r key4,key5

# Update config (all arguments are optional):
# threshold, vote duration, tie-breaker-admin, new admin authority
RUST_LOG=info cargo run --bin cli -- \
  --payer-path ~/.config/solana/id.json \
  --authority-path ~/.config/solana/id.json \
  update-program-config \
  --min-consensus-threshold-bps 6000 \
  --vote-duration 180 \
  --tie-breaker-admin key1 \
  --new-authority-path ~/.config/solana/id.json
```

---

### Snapshot Handling

```bash
# Generates a Solana ledger snapshot for a specific slot (from validator bank state)
# and stores at snapshot path.
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn cargo run --bin cli -- --ledger-path test-ledger --full-snapshots-path test-ledger/backup-snapshots --backup-snapshots-dir test-ledger/backup-snapshots snapshot-slot --slot 340850340

# (DEV MODE - Use Release Mode for production snapshots)
# Generates MetaMerkleSnapshot from the Solana ledger snapshot and stores at save path.
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn cargo run --bin cli -- --ledger-path test-ledger --full-snapshots-path test-ledger/backup-snapshots --backup-snapshots-dir test-ledger/backup-snapshots generate-meta-merkle --slot 340850340 --save-path ./

# (RELEASE MODE)
# Generates MetaMerkleSnapshot from the Solana ledger snapshot using release mode and tmp storage config (linux)
TMPDIR=/mnt/nvme2/solana/tmp \
RUSTFLAGS="-C target-cpu=native" \
RAYON_NUM_THREADS=$(nproc) ZSTD_NBTHREADS=$(nproc) \
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn \
cargo run --release --bin cli -- \
  --ledger-path test-ledger \
  --full-snapshots-path test-ledger/backup-snapshots \
  --backup-snapshots-dir test-ledger/backup-snapshots \
  generate-meta-merkle --slot 361319354

# (RELEASE MODE)
# Generates MetaMerkleSnapshot from the Solana ledger snapshot using release mode and tmp storage config (macos)
TMPDIR=/tmp \
RUSTFLAGS="-C target-cpu=native" \
RAYON_NUM_THREADS=$(sysctl -n hw.ncpu) ZSTD_NBTHREADS=$(sysctl -n hw.ncpu) \
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn \
cargo run --release --bin cli -- \
  --ledger-path test-ledger \
  --full-snapshots-path test-ledger/backup-snapshots \
  --backup-snapshots-dir test-ledger/backup-snapshots \
  generate-meta-merkle --slot 340850340

# Log Merkle root, hash,' and operator signature from snapshot file
RUST_LOG=info cargo run --bin cli -- --authority-path ~/.config/solana/id.json log-meta-merkle-hash  --read-path ./meta_merkle-340850340.zip --is-compressed
```

---

### Log On-Chain State

```bash
# Log ProgramConfig
RUST_LOG=info cargo run --bin cli -- log --ty program-config

# Log BallotBox (e.g. id = 0)
RUST_LOG=info cargo run --bin cli -- log --ty ballot-box --id 0

# Log ConsensusResult of a vote account for a specific ballot (e.g. id = 0)
RUST_LOG=info cargo run --bin cli -- log --ty consensus-result --id 0 --vote-account key1
```

---

### Voting Flow

```bash
# Create a new BallotBox
RUST_LOG=info cargo run --bin cli -- --payer-path ~/.config/solana/id.json --authority-path ~/.config/solana/id.json init-ballot-box

# Vote with root + hash
RUST_LOG=info cargo run --bin cli -- --payer-path ~/.config/solana/id.json --authority-path ~/.config/solana/id.json cast-vote --id 1 --root ByVtRpEnLyD1eVS8Bq21VvDnMffsqPAypaMT9KMZCZcJ --hash 4seYTnZyZNby5ZQTy8ajAapDiMgUYrvYx4hzYRXVn4zH

# Vote using a snapshot file
RUST_LOG=info cargo run --bin cli -- --payer-path ~/.config/solana/id.json --authority-path ~/.config/solana/id.json cast-vote-from-snapshot --id 1 --read-path ./meta_merkle-340850340.zip

# Remove vote (before consensus and voting expiry)
RUST_LOG=info cargo run --bin cli -- --payer-path ~/.config/solana/id.json --authority-path ~/.config/solana/id.json remove-vote --id 1
```

---

### Finalization & Tie-Breaking

```bash
# Finalize winning ballot (after consensus)
RUST_LOG=info cargo run --bin cli -- --payer-path ~/.config/solana/id.json --authority-path ~/.config/solana/id.json finalize-ballot --id 1

# Set tie-breaking result if consensus was not reached
RUST_LOG=info cargo run --bin cli -- --payer-path ~/.config/solana/id.json --authority-path ~/.config/solana/id.json set-tie-breaker --id 1 --idx 0
```

---

## Troubleshooting

### Snapshot Bank Verification Error

If you encounter an error similar to:

```
Snapshot bank for slot 340850340 failed to verify
```

**Solution:** Comment out the line causing the `panic` invocation in the `jito-solana` dependency crate. Snapshot verification failure does not impede generation of a merkle tree snapshot from the source file.

### Genesis Creation Time Mismatch

If you encounter an error such as:

```
Bank snapshot genesis creation time does not match genesis.bin creation time
```

**Solution:** Comment out the `assert_eq` statement in the `jito-solana` dependency crate. Genesis mismatch could occur when the snapshot is retrieved from a different RPC, but does not impede merkle generation.

---

## Additional Testing Commands

### To get genesis config:

1. Create test keypairs:

```
solana-keygen new -o stake-keypair.json
solana-keygen new -o identity-keypair.json
solana-keygen new -o vote-keypair.json
```

2. Extract

```
IDENTITY=$(solana-keygen pubkey identity-keypair.json)
VOTE=$(solana-keygen pubkey vote-keypair.json)
STAKE=$(solana-keygen pubkey stake-keypair.json)
```

3. Get genesis config.

```
solana-genesis   --bootstrap-validator "$IDENTITY" "$VOTE" "$STAKE"   --ledger tmp/testnet-ledger/ --
faucet-lamports 100000000000 -u testnet --cluster-type testnet
```

### To test snapshotting with localnet:

1. Setup cli env

```
export RESTAKING_PROGRAM_ID=RestkWeAVL8fRGgzhfeoqFhsqKRchg6aa1XrcH96z4Q
export VAULT_PROGRAM_ID=Vau1t6sLNxnzB7ZDsef8TLbPLfyZMYXH8WTNqUdm9g8
export TIP_ROUTER_PROGRAM_ID=11111111111111111111111111111111
```

2. Start validator with

```
solana-test-validator
```

3. Run CLI for generating ledger snapshot for a slot (e.g. 100)

```
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn cargo run --bin cli -- --ledger-path test-ledger --full-snapshots-path test-ledger/backup-snapshots --backup-snapshots-dir test-ledger/backup-snapshots snapshot-slot --slot 100
```

4. Run CLI for generating the MeteMerkleSnapshot from the ledger snapshot

```
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn cargo run --bin cli -- --ledger-path test-ledger --full-snapshots-path test-ledger/backup-snapshots --backup-snapshots-dir test-ledger/backup-snapshots generate-meta-merkle --slot 340850340
```

### To generate MetaMerkleSnapshot from testnet snapshots.

1. Find a testnet node with

```
solana gossip -u testnet
```

2. Download snapshot and genesis config from the testnet

```
wget --trust-server-names http://38.147.105.98:8899/snapshot.tar.bz2

wget http://160.202.131.117:8899/genesis.tar.bz2
```

3. Extract snapshot

```
tar -xf genesis.tar.bz2 -C test-ledger/
```

4. Move snapshot to `test-ledger/backup-snapshots/`.
5. Setup cli env (if needed)

```
export RESTAKING_PROGRAM_ID=RestkWeAVL8fRGgzhfeoqFhsqKRchg6aa1XrcH96z4Q
export VAULT_PROGRAM_ID=Vau1t6sLNxnzB7ZDsef8TLbPLfyZMYXH8WTNqUdm9g8
export TIP_ROUTER_PROGRAM_ID=11111111111111111111111111111111
```

5. Clear temp files from `test-ledger` directory after generating.

```
find test-ledger -mindepth 1 -maxdepth 1 \
  ! -name 'backup-snapshots' \
  ! -name 'rocksdb' \
  ! -name 'genesis.bin' \
  -exec rm -rf {} +
```
