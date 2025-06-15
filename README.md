To get genesis config:
1. Create test keypairs:
```
solana-keygen new -o stake-keypair.json
solana-keygen new -o identity-keypair.json
solana-keygen new -o vote-keypair.json
```
2. Extract
IDENTITY=$(solana-keygen pubkey identity-keypair.json)
VOTE=$(solana-keygen pubkey vote-keypair.json)
STAKE=$(solana-keygen pubkey stake-keypair.json)

3. Get geensis config
solana-genesis   --bootstrap-validator "$IDENTITY" "$VOTE" "$STAKE"   --ledger tmp/testnet-ledger/ --
faucet-lamports 100000000000 -u testnet --cluster-type testnet

To test snapshotting with local net:
1. Start validator with `solana-test-validator`
2. Run CLI for Snapshot for a slot (e.g. 100)
```
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn cargo run --bin cli -- --ledger-path test-ledger --full-snapshots-path test-ledger/backup-snapshots --backup-snapshots-dir test-ledger/backup-snapshots snapshot-slot --slot 100
```