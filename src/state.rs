use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, ReadonlyStorage, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};

use crate::hand::{Hand, Hands};

pub static CONFIG_KEY: &[u8] = b"config";
pub const PREFIX_OFFERS: &[u8] = b"offers";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub count: i32,
    pub owner: CanonicalAddr,
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
    pub offeror: CanonicalAddr,
    pub offeree: CanonicalAddr,
    pub offeror_nft: String,
    pub offeree_nft: String,
    pub offeror_hands: Hands,
    pub offeree_hands: Hands,
    pub offeror_draw_point: u8,
}

impl Offer {
    pub fn new(
        id: u64,
        offeror: CanonicalAddr,
        offeror_nft: String,
        offeree_nft: String,
        hands: Vec<u8>,
        draw_point: u8,
    ) -> Offer {
        Offer {
            id: id,
            status: OfferStatus::Offered,
            offeror: offeror,
            offeree: vec![].into(),
            offeror_nft: offeror_nft,
            offeree_nft: offeree_nft,
            offeror_hands: hands.into(),
            offeree_hands: Vec::<Hand>::new().into(),
            offeror_draw_point: draw_point,
        }
    }

    pub fn accept_offer(&mut self, offeree: CanonicalAddr, hands: Vec<u8>) {
        self.status = OfferStatus::Accepted;
        self.offeree = offeree;
        self.offeree_hands = hands.into();
    }

    pub fn decline_offer(&mut self, offeree: CanonicalAddr) {
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
