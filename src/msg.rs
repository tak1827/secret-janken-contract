use cosmwasm_std::{HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    MakeOffer {
        id: u64,
        offeror_nft_contract: HumanAddr,
        offeror_nft: Uint128,
        offeree_nft_contract: HumanAddr,
        offeree_nft: Uint128,
        offeror_hands: Vec<u8>,
        offeror_draw_point: u8,
    },
    AcceptOffer {
        id: u64,
        offeree_hands: Vec<u8>,
    },
    DeclineOffer {
        id: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Offer { id: u64 },
}

// // We define a custom struct for each query response
// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// pub struct CountResponse {
//     pub count: i32,
// }
