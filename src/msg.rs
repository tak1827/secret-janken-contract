use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub prng_seed: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    MakeOffer {
        id: u64,
        offeree: HumanAddr,
        offeror_nft_contract: HumanAddr,
        offeror_nft: String,
        offeror_code_hash: String,
        offeree_nft_contract: HumanAddr,
        offeree_nft: String,
        offeree_code_hash: String,
        offeror_hands: Vec<u8>,
        offeror_draw_point: i8,
    },
    AcceptOffer {
        id: u64,
        offeree_hands: Vec<u8>,
    },
    DeclineOffer {
        id: u64,
    },
    GenerateViewingKey {
        entropy: String,
        padding: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Offer {
        id: u64,
        address: Option<HumanAddr>,
        viewing_key: Option<String>,
    },
    // Offers {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OffersResponse {
    pub ids: Vec<u64>,
}
