use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, ReadonlyStorage, Storage};
use cosmwasm_storage::{bucket, bucket_read, Bucket, ReadonlyBucket};

use crate::hand::{Hand, Hands};

pub const PREFIX_OFFERS: &[u8] = b"offers";

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
