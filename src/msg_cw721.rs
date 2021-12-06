use cosmwasm_std::HumanAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    TransferNft {
        /// recipient of the transfer
        recipient: HumanAddr,
        /// id of the token to transfer
        token_id: String,
        /// optional memo for the tx
        memo: Option<String>,
        /// optional message length padding
        padding: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    OwnerOf {
        token_id: String,
        /// optional address and key requesting to view the token owner
        viewer: Option<ViewerInfo>,
        /// optionally include expired Approvals in the response list.  If ommitted or
        /// false, expired Approvals will be filtered out of the response
        include_expired: Option<bool>,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    OwnerOf {
        owner: HumanAddr,
        approvals: Vec<Cw721Approval>,
    },
}

/// CW721 Approval
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw721Approval {
    /// address that can transfer the token
    pub spender: HumanAddr,
    /// expiration of this approval
    pub expires: Expiration,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
/// at the given point in time and after, Expiration will be considered expired
pub enum Expiration {
    /// expires at this block height
    AtHeight(u64),
    /// expires at the time in seconds since 01/01/1970
    AtTime(u64),
    /// never expires
    Never,
}

/// the address and viewing key making an authenticated query request
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ViewerInfo {
    /// querying address
    pub address: HumanAddr,
    /// authentication key string
    pub viewing_key: String,
}
