use cosmwasm_std::{
    to_binary, Api, Coin, Extern, HumanAddr, Querier, QueryRequest, StdError, Storage, WasmQuery,
};

use crate::msg_cw721::{QueryAnswer, QueryMsg as Cw721QueryMsg};
use crate::state::{config_read, State};
use crate::state::{offers, offers_read, token_bets_read, Offer};

pub fn validate_offer_id<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    id: u64,
) -> Result<bool, StdError> {
    match offers_read(&deps.storage).may_load(&id.to_be_bytes()) {
        Ok(None) => return Ok(true),
        _ => return Err(StdError::generic_err(format!("duplicated id({})", id))),
    }
}

pub fn validate_nft<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    contract_addr: HumanAddr,
    token_id: String,
    callback_code_hash: String,
    expected_owner: HumanAddr,
) -> Result<bool, StdError> {
    let req = Cw721QueryMsg::OwnerOf {
        token_id,
        viewer: None,
        include_expired: None,
    };

    let query = QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr,
        msg: to_binary(&req)?,
        callback_code_hash,
    });

    let res = deps.querier.query::<QueryAnswer>(&query)?;
    let owner = match res {
        QueryAnswer::OwnerOf { owner, .. } => owner,
    };

    if owner != expected_owner {
        return Err(StdError::generic_err(format!(
            "invalid nft owner, got: {:?}, expected: {}",
            owner, expected_owner
        )));
    }

    Ok(true)
}

pub fn validate_offeree<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    offeree: HumanAddr,
    id: u64,
) -> Result<Offer, StdError> {
    let offer = match offers(&mut deps.storage).load(&id.to_be_bytes()) {
        Ok(offer) => offer,
        Err(err) => return Err(StdError::generic_err(format!("invalid id: {:?}", err))),
    };

    if &offer.offeree != &offeree {
        return Err(StdError::generic_err(format!(
            "msg sender is not offeree({})",
            &offer.offeree
        )));
    }
    Ok(offer)
}

pub fn validate_token_bet_id<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    id: u64,
) -> Result<bool, StdError> {
    match token_bets_read(&deps.storage).may_load(&id.to_be_bytes()) {
        Ok(None) => return Ok(true),
        _ => return Err(StdError::generic_err(format!("duplicated id({})", id))),
    }
}

pub fn validate_sent_funds(funds: Vec<Coin>) -> Result<Coin, StdError> {
    if funds.len() != 1 {
        return Err(StdError::generic_err(format!(
            "multiple coin sent({:?})",
            funds
        )));
    }

    let fund = &funds[0];
    if fund.amount.is_zero() {
        return Err(StdError::generic_err(format!(
            "sent fund is zero amount({:?})",
            fund
        )));
    }
    Ok(fund.clone())
}

pub fn validate_balance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: &HumanAddr,
    denom: &str,
    amount: u128,
) -> Result<bool, StdError> {
    let balance = deps.querier.query_balance(address, denom)?;
    if balance.amount.u128() < amount as u128 {
        return Err(StdError::generic_err(format!(
            "insufficient balance in address({})",
            address
        )));
    }
    Ok(true)
}

pub fn validate_withdrawer<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: &HumanAddr,
    denom: &str,
    amount: u64,
) -> Result<bool, StdError> {
    let state: State = config_read(&deps.storage).load()?;
    if address != &state.fee_recipient {
        return Err(StdError::unauthorized());
    }
    validate_balance(deps, address, denom, amount.into())
}
