# NEAR - NFT Auction


Prerequisites:
+ [`cargo` (Rust package manager)](https://rust-lang.org/tools/install/)
+ [`near` and `cargo-near` to build contracts](https://docs.near.org/smart-contracts/quickstart#prerequisites)
+ [`neard` to run a local node](https://near-nodes.io/validator/compile-and-run-a-node)
+ `git` to clone 'github.com/near-examples/NFT'
+ `sed`


### Running the NEAR node

```bash
# Initialize local genesis
neard --home ./.localnet init --chain-id localnet

# Start node (^C to stop)
neard --home ./.localnet run
```


### Configuring the local network

```bash
near config add-connection \
  --network-name localnet \
  --connection-name localnet \
  --rpc-url http://localhost:3030/ \
  --wallet-url http://localhost:3030/ \
  --explorer-transaction-url http://localhost:3030/ \
  --linkdrop-account-id localnet
```


### Managing accounts

There are 4 accounts involved:
1. NFT Auction Contract
2. Example NFT Contract
3. Example NFT Owner
4. Example Bidder

```bash
# Import test.near, who owns funds in localnet by default, as a known wallet
near account import-account using-private-key
## stdin>
##   1. Private (secret) key: Enter private-key found in ./.localnet/validator_key.json
##   2. Network: localnet
##   3. Account Id: test.near
##   4. Keychain: Legacy keychain

# Function to create a new localnet account, funded by test.near
create-account() {
  near create-account $1 \
    --use-account test.near \
    --network-id localnet
}

# Function to additionally fund a localnet account, paid by test.near
fund-account() {
  near send test.near $1 $2 \
    --network-id localnet
}

# Create & fund the 4 required accounts

## 1. NFT Auction Contract
create-account nftauction.test.near
fund-account nftauction.test.near 10
## 2. Example NFT Contract
create-account nft.test.near
fund-account nft.test.near 10
## 3. Example NFT Owner
create-account john.test.near
fund-account john.test.near 10
## 4. Example Bidder
create-account alice.test.near
fund-account alice.test.near 10
```

### Building the Contracts

1. NFT Auction Contract
```bash
cargo near build
## stdin> Select 'non-reproducible-wasm' for testing
```

2. Example NFT Contract
```bash
# Preferably go to a different directory first
git clone https://github.com/near-examples/NFT example-nft
cd ./example-nft

# Switch from `stable` to `1.86.0` Rust channel
# NOTE: If your Rust toolchain channel == 1.86.0, this is not required
sed -i 's/stable/1.86.0/g' ./rust-toolchain.toml

# Build
cargo near build
```

### Deploying the Contracts

```bash
# Deploy nftauction to localnet
near deploy nftauction.test.near \
  --wasmFile ./target/near/nftauction.wasm \
  --network-id localnet

# Deploy example-nft to localnet
near deploy nft.test.near \
  --wasmFile /path/to/example-nft/target/near/non_fungible_token.wasm \
  --initFunction new_default_meta \
  --init-args '{"owner_id" : "john.test.near"}' \
  --network-id localnet
```


### Interacting with the Contracts

1. No bidders

    ```bash
    # Have John mint an NFT with token-id "first" to himself
    #
    # NOTE: Change deposit if it isn't enough, to the value suggested in the output
    near call nft.test.near \
      nft_mint \
      '{
        "token_id": "first",
        "token_owner_id": "john.test.near",
        "token_metadata": {}
      }' \
      --network-id localnet \
      --use-account john.test.near \
      --deposit 0.00565
    
    # Approve ownership of our NFT to nftauction,
    # along with the options that will configure the start of the auction passed to field `msg`
    #   + `timespan`: 100 ----> Auction will end 100 seconds from now
    #   + `minimum_bid`: 0 ---> Auction will start at a minimum bid of 0 NEAR
    #
    # NOTE: Change deposit if it isn't enough, to the value suggested in the output
    near call nft.test.near \
      nft_approve '{
        "token_id": "first",
        "account_id": "nftauction.test.near",
        "msg": "{ \"timespan\": 100, \"minimum_bid\": \"0\" }"
      }' \
      --network-id localnet \
      --use-account john.test.near \
      --deposit 0.00033
    
    # Make sure that now there is 1 ongoing auction
    near view nftauction.test.near \
      len \
      --network-id localnet
    # stdout> 1 [expected]
    
    # Since our start time was so small, ensure that our auction is in fact already over
    near view nftauction.test.near \
      expired \
      '{
        "nft": "nft.test.near",
        "token_id": "first"
      }' \
      --network-id localnet
    # stdout> true [expected]
    
    # Have John end the auction
    #
    # NOTE: Change deposit if it isn't enough, to the value suggested in the output
    near call nftauction.test.near \
      end_auction \
      '{
        "nft": "nft.test.near",
        "token_id": "first"
      }' \
      --network-id localnet \
      --use-account john.test.near \
      --deposit 0.00026
    ```

2. One bidder

`TODO`

