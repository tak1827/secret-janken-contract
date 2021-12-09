use cosmwasm_std::{CanonicalAddr, HumanAddr, ReadonlyStorage, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, PrefixedStorage, ReadonlyBucket,
    ReadonlyPrefixedStorage, ReadonlySingleton, Singleton,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::hand::{Hand, Hands};
use crate::viewing_key::ViewingKey;

pub const CONFIG_KEY: &[u8] = b"config";
pub const PREFIX_OFFERS: &[u8] = b"offers";
pub const PREFIX_TOKEN_BETS: &[u8] = b"tokenbets";
pub const PREFIX_VIEWING_KEY: &[u8] = b"viewingkey";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub prng_seed: Vec<u8>,
    pub entropy: Vec<u8>,
    pub banker_wallet: HumanAddr,
    pub fee_recipient: HumanAddr,
    pub fee_rate: u64,
}

pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, State> {
    singleton(storage, CONFIG_KEY)
}

pub fn config_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, State> {
    singleton_read(storage, CONFIG_KEY)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OfferStatus {
    Offered,
    Accepted,
    Declined,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Offer {
    pub id: u64,
    pub status: OfferStatus,
    pub offeror: HumanAddr,
    pub offeree: HumanAddr,
    pub offeror_nft_contract: HumanAddr,
    pub offeror_nft: String,
    pub offeror_code_hash: String,
    pub offeree_nft_contract: HumanAddr,
    pub offeree_nft: String,
    pub offeree_code_hash: String,
    pub offeror_hands: Hands,
    pub offeree_hands: Hands,
    pub offeror_draw_point: i8,
    pub winner: String,
}

impl Offer {
    pub fn new(
        id: u64,
        offeror: HumanAddr,
        offeree: HumanAddr,
        offeror_nft_contract: HumanAddr,
        offeror_nft: String,
        offeror_code_hash: String,
        offeree_nft_contract: HumanAddr,
        offeree_nft: String,
        offeree_code_hash: String,
        hands: Vec<u8>,
        draw_point: i8,
    ) -> Offer {
        Offer {
            id,
            status: OfferStatus::Offered,
            offeror,
            offeree,
            offeror_nft_contract,
            offeror_nft,
            offeror_code_hash,
            offeree_nft_contract,
            offeree_nft,
            offeree_code_hash,
            offeror_hands: hands.into(),
            offeree_hands: Vec::<Hand>::new().into(),
            offeror_draw_point: draw_point,
            winner: "".to_string(),
        }
    }

    pub fn accept_offer(&mut self, offeree: HumanAddr, hands: Vec<u8>) {
        self.status = OfferStatus::Accepted;
        self.offeree = offeree;
        self.offeree_hands = hands.into();
    }

    pub fn decline_offer(&mut self, offeree: HumanAddr) {
        self.status = OfferStatus::Declined;
        self.offeree = offeree;
    }
}

pub fn offers<S: Storage>(storage: &mut S) -> Bucket<S, Offer> {
    bucket(PREFIX_OFFERS, storage)
}

pub fn offers_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, Offer> {
    bucket_read(PREFIX_OFFERS, storage)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenBet {
    pub id: u64,
    pub denom: String,
    pub amount: u64,
    pub hand: Hand,
    pub result: String,
}

pub fn token_bets<S: Storage>(storage: &mut S) -> Bucket<S, TokenBet> {
    bucket(PREFIX_TOKEN_BETS, storage)
}

pub fn token_bets_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, TokenBet> {
    bucket_read(PREFIX_TOKEN_BETS, storage)
}

pub fn write_viewing_key<S: Storage>(store: &mut S, owner: &CanonicalAddr, key: &ViewingKey) {
    let mut user_key_store = PrefixedStorage::new(PREFIX_VIEWING_KEY, store);
    user_key_store.set(owner.as_slice(), &key.to_hashed());
}

pub fn read_viewing_key<S: Storage>(store: &S, owner: &CanonicalAddr) -> Option<Vec<u8>> {
    let user_key_store = ReadonlyPrefixedStorage::new(PREFIX_VIEWING_KEY, store);
    user_key_store.get(owner.as_slice())
}
