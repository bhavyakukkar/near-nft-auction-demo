use std::hash::{DefaultHasher, Hash, Hasher};

use near_contract_standards::non_fungible_token::approval::ext_nft_approval;
use near_sdk::{env, near, require, store::IterableMap, AccountId, NearToken, Promise};

#[near(serializers = [borsh])]
pub struct Bid {
    amount: NearToken,
    paid: bool,
}

#[near(serializers = [borsh])]
pub struct Auction {
    owner: AccountId,
    bids: IterableMap<AccountId, Bid>,
    h_bid: NearToken,
    expiry: u64,
}

#[near(serializers = [borsh])]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NFTId(u64);

impl NFTId {
    pub fn new(nft: &AccountId, token_id: &TokenId) -> Self {
        let mut hasher = DefaultHasher::new();
        nft.hash(&mut hasher);
        token_id.hash(&mut hasher);
        NFTId(hasher.finish())
    }
}

#[near(contract_state)]
pub struct Contract {
    auctions: IterableMap<NFTId, Auction>,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            auctions: IterableMap::new(b"a"),
        }
    }
}

type TokenId = String;

// #[ext_contract(ext_nft)]
// pub trait NonFungibleTokenCore {
//     //approve an account ID to transfer a token on your behalf
//     fn nft_approve(&mut self, token_id: TokenId, account_id: AccountId, msg: Option<String>);

//     //check if the passed in account has access to approve the token ID
//     fn nft_is_approved(
//         &self,
//         token_id: TokenId,
//         approved_account_id: AccountId,
//         approval_id: Option<u64>,
//     ) -> bool;

//     //revoke a specific account from transferring the token on your behalf
//     fn nft_revoke(&mut self, token_id: TokenId, account_id: AccountId);

//     //revoke all accounts from transferring the token on your behalf
//     fn nft_revoke_all(&mut self, token_id: TokenId);
// }

#[near]
impl Contract {
    #[payable]
    pub fn start_auction(
        &mut self,
        nft: AccountId,
        token_id: TokenId,
        timespan: u64,
        minimum_bid: NearToken,
    ) -> Promise {
        // Validations
        require!(timespan > 0, "timestamp must be greater than 0");
        let current_time = env::block_timestamp();
        let Some(expiry) = current_time.checked_add(timespan) else {
            env::panic_str("adding `timespan` to `timestamp` overflowed, `timespan` is too big")
        };

        // Operations
        let promise = ext_nft_approval::ext(nft.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .nft_approve(token_id.clone(), env::current_account_id(), None)
            .as_return();
        let auction = Auction {
            owner: env::signer_account_id(),
            bids: IterableMap::new(b"a"),
            h_bid: minimum_bid,
            expiry,
        };
        let nft_id = NFTId::new(&nft, &token_id);
        self.auctions.insert(nft_id, auction);
        promise
    }

    pub fn end_auction(&mut self, nft: AccountId, token_id: TokenId) -> Promise {
        // Validations
        let nft_id = NFTId::new(&nft, &token_id);
        let Some(auction) = self.auctions.get(&nft_id) else {
            env::panic_str("this nft is not in auction")
        };
        let current_time = env::block_timestamp();
        require!(
            current_time >= auction.expiry,
            "cannot end, auction is still ongoing"
        );

        // Operations
        let promise = match auction.bids.iter().last() {
            // Highest bidder exists
            Some((h_bidder, Bid { amount, paid: _ })) => {
                // Transfer NFT to highest bidder
                ext_nft_approval::ext(nft)
                    .nft_approve(token_id, h_bidder.clone(), None)
                    .as_return()
                    .then(
                        auction
                            .bids
                            .iter()
                            // Don't refund the highest-bidder & those already refunded (having
                            // `paid == true`)
                            //
                            // Bid-entries may already be refunded in case of calls to:
                            // 1. `update_bid`: Bidders old entry just gets marked as paid
                            // 2. `refund_bid`
                            .filter(|(acc_id, Bid { paid, .. })| *acc_id != h_bidder && !paid)
                            .fold(
                                // Pay bid-amount to NFT owner
                                // (always called once)
                                Promise::new(auction.owner.clone()).transfer(*amount),
                                // Refund all bidders that didn't win the bid
                                // (called 0 or more times)
                                |accum_promise, (acc_id, Bid { amount, .. })| {
                                    accum_promise
                                        .then(Promise::new(acc_id.clone()).transfer(*amount))
                                },
                            ),
                    )
            }

            // No bidders, Return NFT to owner
            None => ext_nft_approval::ext(nft)
                .nft_approve(token_id, auction.owner.clone(), None)
                .as_return(),
        };
        assert!(self.auctions.remove(&nft_id).is_some());
        promise
    }

    pub fn make_bid(&mut self, nft: AccountId, token_id: TokenId, amount: NearToken) {
        // Validations
        let nft_id = NFTId::new(&nft, &token_id);
        let Some(auction) = self.auctions.get_mut(&nft_id) else {
            env::panic_str("this nft is not in auction")
        };
        require!(
            amount > auction.h_bid,
            "bid amount does not exceed previous bid or minimum bid amount"
        );
        require!(
            env::attached_deposit() >= amount,
            "provided deposit does not cover bid amount"
        );
        let bidder = env::signer_account_id();
        require!(
            !auction.bids.contains_key(&bidder),
            "bidder has already made a bid, either call `refundBid` or `updateBid`"
        );
        let current_time = env::block_timestamp();
        require!(current_time < auction.expiry, "cannot bid, auction is over");

        // Operations
        auction.bids.insert(
            bidder,
            Bid {
                amount,
                paid: false,
            },
        );
    }

    pub fn len(&self) -> u32 {
        self.auctions.len()
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
}
