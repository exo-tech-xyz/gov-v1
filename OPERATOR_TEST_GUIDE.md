# SOL Gov Operator Test Guide

The following document is a guide for Operator on-boarding to the NCN testing portion of the SOL gov project. 

## Background

At accelerate during a community governance round table, [Exo](https://exotechnologies.xyz/) pitched a group about a new way of network level governance that would get rid of the archaic token transferring mechanism and allow stakers to participate in the voting process without reliance on their validator. This project was accepted by the working group and initial development was funded by Solana Foundation. 

The system is relies on 2 smart contracts: 

1. snapshot NCN program
2. Voting program

In simplest terms the *snapshot NCN program* is a Node Consensus Network (NCN) program that manages a list of operators and ballot boxes. The operators use the ballot boxes to vote and come to consensus on the root hash of a merkle tree representation of the network’s active stake at a point in time. 

The *Voting program* uses the agreed upon merkle tree as the basis for voting weights. Validators voting weights start out as the sum of their delegations. But as their stakers cast their votes, the validator’s vote weight is decremented. 

We’ve explained the added utility in the shortest terms possible to keep this document relevant to operator on-boarding.

## Operators

We’re looking for a few new operators to on-board while we test. If you engage and perform the operations well, then you’ll likely be added to the mainnet NCN. Incentives for being an operator are TBD.

## RPC Requirements

### Machine Requirements

- Disk: Standard validator storage ( + ~500GB for snapshots (~100GB per snapshot)

### Settings

- Agave version ^3.0.6
- Validator Settings:
    - Ledger Limit (Shred Count): 200,000,000
    - Full Snapshots Retention: 2
    - Full Snapshots Interval: 100,000
    - Incremental Snapshots Retention: 30
    - Incremental Snapshots Interval: 2500
- LimitNOFILE=2000000

# Pre-Requisites

1. Secure machine for mainnet RPC node (with recommended requirements and settings). Make sure it’s caught up and maintaining the tip of the chain.
2. Setup [CLI dependencies](https://github.com/exo-tech-xyz/gov-v1?tab=readme-ov-file#dependencies)
3. Test run through [snapshot, merkle tree generation](https://www.notion.so/SOL-Gov-Operator-Test-Guide-2e79434d602381ab827ccb73c99cc75e?pvs=21) and log the [merkle root](https://github.com/exo-tech-xyz/gov-v1/tree/main?tab=readme-ov-file#snapshot-generation-and-handling).
4. Deploy a verifier server ([guide](https://github.com/exo-tech-xyz/gov-v1/blob/main/verifier-service/DEPLOYMENT.md)).
5. Test run uploading of a merkle snapshot to the verifier server and fetching from the other endpoints.

## Snapshot and Merkle Tree Generation

There are currently two approaches to using the CLI for snapshot and merkle tree generation:

### Automated Workflow (Recommended)

- The automated workflow initializes a task that watches for the target slot to pass, backs up ledger and snapshots, and starts the snapshot, followed by merkle tree generation.
- Replace the ledger directory paths as appropriate based on local disk setup. Below are sample directory and file paths based on our setup.
    
    ```rust
    // INITIAL SETUP
    // Create backup ledger directory.
    mkdir /mnt/ledger/gov-ledger-backup
    mkdir /mnt/ledger/gov-ledger-backup/snapshots/
    
    // Copy genesis to backup ledger.
    cp /mnt/ledger/genesis.bin /mnt/ledger/gov-ledger-backup/genesis.bin
    
    // BEFORE EACH SNAPSHOT GENERATION
    // Once the target_slot is known, e.g. 375070000
    // Initiate await-snapshot task to watch and start snapshot & merkle tree
    // once slot passes
    RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=$(nproc) ZSTD_NBTHREADS=$(nproc) \
    RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn \
    cargo run --release --bin cli -- \
      await-snapshot \
      --scan-interval 1 \
      --slot 376419403 \
      --snapshots-dir /mnt/ledger/snapshots \
      --backup-snapshots-dir /mnt/ledger/gov-ledger-backup/snapshots \
      --backup-ledger-dir /mnt/ledger/gov-ledger-backup \
      --agave-ledger-tool-path /home/jito/agave/target/release/agave-ledger-tool \
      --ledger-path /mnt/ledger \
      --generate-meta-merkle
    ```
    

### Manual Workflow

- This approach is useful when the automated workflow fails to complete.
- Replace the ledger directory paths as appropriate based on local disk setup. Below are sample directory and file paths based on our setup.
    
    ```rust
    // INITIAL SETUP
    // Create backup ledger directory.
    mkdir /mnt/ledger/gov-ledger-backup
    mkdir /mnt/ledger/gov-ledger-backup/snapshots/
    
    // Copy genesis to backup ledger.
    cp /mnt/ledger/genesis.bin /mnt/ledger/gov-ledger-backup/genesis.bin
    
    // BEFORE EACH SNAPSHOT GENERATION
    // For example: target_slot = 375070000
    
    // Backup latest available incremental snapshot that is <= target_slot
    cp /mnt/ledger/snapshots/incremental-snapshot-375035297-375067824-AQf6woTbG5hwjMnDgvXKeu6mxtr7eNn7oE956y8vMvda.tar.zst\
      /mnt/ledger/gov-ledger-backup/snapshots/
      
    // Backup Full Snapshot with starting slot that matches incremental
    cp /mnt/ledger/snapshots/snapshot-375035297-Ct6wCvQfVvwT6NSsK57LQRVVSKeesQisKLaNxvL3Vxf7.tar.zst\
      /mnt/ledger/gov-ledger-backup/snapshots/
    
    // Using Agave Ledger Tool, backup ledger between slots from 
    // end of incremental snapshot to (target_slot + 32)
    /home/jito/agave/target/release/agave-ledger-tool \
      blockstore --ignore-ulimit-nofile-error -l /mnt/ledger \
      copy --starting-slot 375067824 --ending-slot 375070032 \
      --target-ledger /mnt/ledger/gov-ledger-backup/
      
    // Increase file descriptor limit (if required - recommended to set 2000000)
    ulimit -n 1000000
    
    // Generate snapshot for target_slot using backup ledger 
    // (set threads as appropriate)
    RUSTFLAGS="-C target-cpu=native" RAYON_NUM_THREADS=16 ZSTD_NBTHREADS=16 \
    RUST_LOG=info,solana_runtime=info,solana_accounts_db=info,solana_metrics=info \
    cargo run --release --bin cli -- \
      --ledger-path /mnt/ledger/gov-ledger-backup \
      --full-snapshots-path /mnt/ledger/gov-ledger-backup/snapshots \
      --backup-snapshots-dir /mnt/ledger/gov-ledger-backup/snapshots \
      snapshot-slot --slot 375070000
      
    // Generates MetaMerkleSnapshot from the Solana ledger snapshot using release mode and tmp storage config (linux)
    // Create a tmp directory for `TMPDIR` and `account-paths` for storing intermediary files.
    // Output snapshot is stored in current directory by default.
    TMPDIR=/mnt/ledger/gov-tmp \
    RUSTFLAGS="-C target-cpu=native" \
    RAYON_NUM_THREADS=$(nproc) ZSTD_NBTHREADS=$(nproc) \
    RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn \
    cargo run --release --bin cli -- \
      --ledger-path /mnt/ledger \
      --account-paths /mnt/ledger/gov-ledger-backup \
      --backup-snapshots-dir /mnt/ledger/gov-ledger-backup/snapshots \
      generate-meta-merkle --slot 375070000
    
    ```
    
- Out of memory issues may occur if playback of more than 10,000 slots is required. The current workaround is to invoke the `snapshot-slot` iteratively for 10,000 slots progress each time (to generate intermediary full snapshots) until the target slot is reached.

# Once  you’ve completed the pre-requests ping your point of contact and await further instructions.

# Test Plan

1. Determine the slot that the node must come to consensus on. This is normally handled via coordination with the voting program, but that’s still work in progress. So for these tests the test coordinator will provide you with a specific slot.
    - [Test coordinator / Lead Operator] Create a new BallotBox via `InitBallotBox` instruction, and share the `ballot_id` with other operators.
2. [Start](https://www.notion.so/SOL-Gov-Operator-Test-Guide-2e79434d602381ab827ccb73c99cc75e?pvs=21) `await-snapshot` CLI command to generate snapshot and merkle tree automatically once slot passes. The generation process may take 2-3 hours.
3. Log merkle root & snapshot hash from the merkle snapshot file, as well as the snapshot signature (used later for snapshot upload).
    
    ```markdown
    # Replace directory paths and slot
    RUST_LOG=info cargo run --bin cli -- --authority-path ~/.config/solana/id.json log-meta-merkle-hash  --read-path ./meta_merkle-367628001.zip --is-compressed
    ```
    
4. Vote using the **Mainnet** snapshot file on the *Devnet* *****NCN **P**rogram*.
    
    ```markdown
    # https://github.com/exo-tech-xyz/gov-v1?tab=readme-ov-file#voting-flow
    # Replace directory paths, slot, ballot_id, read_paths
    RUST_LOG=info cargo run --bin cli -- \
      --payer-path ~/.config/solana/id.json \
      --authority-path ~/.config/solana/id.json \
      --rpc-url https://api.devnet.solana.com \
      cast-vote-from-snapshot --id 1 \
      --read-path ./meta_merkle-340850340.zip
    ```
    
5. [Test coordinator] Invoke [finalize ballot commands](https://github.com/exo-tech-xyz/gov-v1?tab=readme-ov-file#finalization--tie-breaking) that creates the `ConsensusResult` account.
6. Once consensus is reached, [upload](https://github.com/exo-tech-xyz/gov-v1/tree/main/verifier-service#upload-a-snapshot) snapshot to the Verifier Service.
    
    ```markdown
    # Replace local host with public IP address of the web server.due to timeout limits.
    # Replace slot, merkle root, signature and snapshot path.
    curl -X POST http://localhost:3000/upload \
      -F "slot=340850340" \
      -F "network=testnet" \
      -F "merkle_root=34sfrZPCyuLXsq5v1ybahTVSwQQE6A3VJyr9JcgxsW21" \
      -F "signature=3nn1EGUqZ5GSXgfAs86miP4z5HtVdKYdQeDdhm1p2M5XxfK16cxwBJYonFdN4BDT7qzpx6TyEhHUrnF2Bh7wGm71" \
      -F "file=@meta_merkle-340850340.zip" \
      -w "\nHTTP Status: %{http_code}\n" \
      -s
    ```