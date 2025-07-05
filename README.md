## Solana Governance Voter Snapshot Program 

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
RUST_LOG=info,solana_runtime=warn,solana_accounts_db=warn,solana_metrics=warn cargo run --bin cli -- --ledger-path test-ledger --full-snapshots-path test-ledger/backup-snapshots --backup-snapshots-dir test-ledger/backup-snapshots generate-meta-merkle --slot 340850340 --epoch 0
```

### To generate MetaMerkleSnapshot from testnet snapshots.
1. Find a testnet node with 
```
solana gossip -u testnet
```
2. Download snapshot and genesis config from the testnet
```
wget --trust-server-names http://64.34.80.79:8899/snapshot.tar.bz2

wget http://160.202.131.117:8899/genesis.tar.bz2
```
3. Extract snapshot 
```
tar -xf genesis.tar.bz2 -C test-ledger/
```
4. Move snapshot to `test-ledger/backup-snapshots/`.
4. Setup cli env (if needed)
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