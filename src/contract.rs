use cosmwasm_std::{
    to_binary, Api, Binary, Context, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier,
    StdError, StdResult, Storage, WasmMsg,
};

use crate::hand::{Hand, Hands, MatchResult};
use crate::msg::{HandleMsg, InitMsg, QueryMsg, Snip721HandleMsg};
use crate::state::{offers, offers_read, Offer, OfferStatus};

pub fn init<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::MakeOffer {
            id,
            offeree,
            offeror_nft_contract,
            offeror_nft,
            offeror_code_hash,
            offeree_nft_contract,
            offeree_nft,
            offeree_code_hash,
            offeror_hands,
            offeror_draw_point,
        } => try_offer(
            deps,
            env,
            id,
            offeree,
            offeror_nft_contract,
            offeror_nft,
            offeror_code_hash,
            offeree_nft_contract,
            offeree_nft,
            offeree_code_hash,
            offeror_hands,
            offeror_draw_point,
        ),
        HandleMsg::AcceptOffer { id, offeree_hands } => try_accept(deps, env, id, offeree_hands),
        HandleMsg::DeclineOffer { id } => try_decline(deps, env, id),
    }
}

pub fn try_offer<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    id: u64,
    offeree: HumanAddr,
    offeror_nft_contract: HumanAddr,
    offeror_nft: String,
    offeror_code_hash: String,
    offeree_nft_contract: HumanAddr,
    offeree_nft: String,
    offeree_code_hash: String,
    hands: Vec<u8>,
    draw_point: i8,
) -> StdResult<HandleResponse> {
    match offers(&mut deps.storage).may_load(&id.to_be_bytes()) {
        Ok(None) => {}
        _ => return Err(StdError::generic_err(format!("duplicated id({})", id))),
    }

    let offer = Offer::new(
        id,
        env.message.sender.clone(),
        offeree,
        offeror_nft_contract,
        offeror_nft,
        offeror_code_hash,
        offeree_nft_contract,
        offeree_nft,
        offeree_code_hash,
        hands,
        draw_point,
    );

    offers(&mut deps.storage).save(&id.to_be_bytes(), &offer)?;

    Ok(HandleResponse::default())
}

pub fn try_accept<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    id: u64,
    hands: Vec<u8>,
) -> StdResult<HandleResponse> {
    let mut offer = validate_offeree(deps, env.message.sender.clone(), id)?;

    offer.accept_offer(env.message.sender.clone(), hands);

    let offeror_hands = &offer.offeror_hands;
    let offeree_hands = &offer.offeree_hands;

    let result = offeror_hands.compete(offeree_hands, offer.offeror_draw_point);

    let mut ctx = Context::new();
    ctx.add_log("action", "competed");

    if result.eq(&MatchResult::Win) {
        offer.winner = "offeror".to_string();
        let msg = to_binary(&Snip721HandleMsg::TransferNft {
            recipient: offer.offeror.clone(),
            token_id: offer.offeree_nft.clone(),
            memo: None,
            padding: None,
        })?;
        ctx.add_message(WasmMsg::Execute {
            contract_addr: offer.offeree_nft_contract.clone(),
            callback_code_hash: offer.offeree_code_hash.clone(),
            msg,
            send: vec![],
        });
    } else if result.eq(&MatchResult::Lose) {
        offer.winner = "offeree".to_string();
        let msg = to_binary(&Snip721HandleMsg::TransferNft {
            recipient: offer.offeree.clone(),
            token_id: offer.offeror_nft.clone(),
            memo: None,
            padding: None,
        })?;
        ctx.add_message(WasmMsg::Execute {
            contract_addr: offer.offeror_nft_contract.clone(),
            callback_code_hash: offer.offeror_code_hash.clone(),
            msg,
            send: vec![],
        });
    } else {
        offer.winner = "draw".to_string();
    };

    ctx.add_log("winner", &offer.winner);

    offers(&mut deps.storage).update(&id.to_be_bytes(), |_| Ok(offer.clone()))?;

    Ok(ctx.into())
}

pub fn try_decline<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    id: u64,
) -> StdResult<HandleResponse> {
    let mut offer = validate_offeree(deps, env.message.sender.clone(), id)?;
    offer.decline_offer(env.message.sender.clone());

    offers(&mut deps.storage).update(&id.to_be_bytes(), |_| Ok(offer))?;

    Ok(HandleResponse::default())
}

fn validate_offeree<S: Storage, A: Api, Q: Querier>(
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

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Offer { id } => query_offer(&deps, id),
    }
}

fn query_offer<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    id: u64,
) -> StdResult<Binary> {
    match offers_read(&deps.storage).may_load(&id.to_be_bytes()) {
        Ok(Some(mut o)) => {
            if o.status == OfferStatus::Offered {
                o.offeror_hands = Vec::<Hand>::new().into();
            }
            return to_binary(&o);
        }
        _ => return to_binary(""),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{CosmosMsg, StdError};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {};
        let env = mock_env("creator", &[]);

        init(&mut deps, env, msg).unwrap();
    }

    #[test]
    fn try_offer() {
        let mut deps = mock_dependencies(20, &[]);
        let msg = InitMsg {};
        let env = mock_env("creator", &[]);
        init(&mut deps, env, msg).unwrap();

        let offer_id = 123;
        let env = mock_env("offeror", &[]);
        let msg = HandleMsg::MakeOffer {
            id: offer_id,
            offeree: "offeree".into(),
            offeror_nft_contract: "offeror_contract".into(),
            offeror_nft: "1".to_string(),
            offeror_code_hash: "offeror_code_hash".to_string(),
            offeree_nft_contract: "offeree_contract".into(),
            offeree_nft: "2".to_string(),
            offeree_code_hash: "offeree_code_hash".to_string(),
            offeror_hands: vec![1, 2, 3],
            offeror_draw_point: 2,
        };

        // succeed
        handle(&mut deps, env.clone(), msg.clone()).unwrap();

        // failed by duplicated id
        let res = handle(&mut deps, env, msg);
        assert_eq!(
            Some(StdError::generic_err(format!(
                "duplicated id({})",
                offer_id
            ))),
            res.err()
        );

        // check query
        let msg = QueryMsg::Offer { id: offer_id };
        let res = query(&deps, msg);
        let expected = to_binary(&Offer {
            id: offer_id,
            status: OfferStatus::Offered,
            offeror: "offeror".into(),
            offeree: "offeree".into(),
            offeror_nft_contract: "offeror_contract".into(),
            offeror_nft: "1".to_string(),
            offeror_code_hash: "offeror_code_hash".to_string(),
            offeree_nft_contract: "offeree_contract".into(),
            offeree_nft: "2".to_string(),
            offeree_code_hash: "offeree_code_hash".to_string(),
            offeror_hands: Vec::<Hand>::new().into(),
            offeree_hands: Vec::<Hand>::new().into(),
            offeror_draw_point: 2,
            winner: "".to_string(),
        });

        assert_eq!(expected, res);
    }

    #[test]
    fn try_accept() {
        let mut deps = mock_dependencies(20, &[]);
        let msg = InitMsg {};
        let env = mock_env("creator", &[]);
        init(&mut deps, env, msg).unwrap();

        let offer_id = 123;
        let offeror_nft = "123".to_string();
        let offeree_nft = "321".to_string();
        let env = mock_env("offeror", &[]);
        let msg = HandleMsg::MakeOffer {
            id: offer_id,
            offeree: "offeree".into(),
            offeror_nft_contract: "offeror_contract".into(),
            offeror_nft: offeror_nft.clone(),
            offeror_code_hash: "offeror_code_hash".to_string(),
            offeree_nft_contract: "offeree_contract".into(),
            offeree_nft: offeree_nft.clone(),
            offeree_code_hash: "offeree_code_hash".to_string(),
            offeror_hands: vec![1, 2, 3],
            offeror_draw_point: -1,
        };

        handle(&mut deps, env.clone(), msg.clone()).unwrap();

        let env = mock_env("offeree", &[]);
        let msg = HandleMsg::AcceptOffer {
            id: offer_id,
            offeree_hands: vec![3, 2, 1],
        };

        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(1, res.messages.len());

        let transfer_msg = to_binary(&Snip721HandleMsg::TransferNft {
            recipient: "offeror".into(),
            token_id: offeree_nft.clone(),
            memo: None,
            padding: None,
        })
        .unwrap();
        let msg: CosmosMsg = WasmMsg::Execute {
            contract_addr: "offeree_contract".into(),
            callback_code_hash: "offeree_code_hash".to_string(),
            msg: transfer_msg,
            send: vec![],
        }
        .into();

        assert_eq!(msg, res.messages[0]);

        // check query
        let msg = QueryMsg::Offer { id: offer_id };
        let res = query(&deps, msg);
        let expected = to_binary(&Offer {
            id: offer_id,
            status: OfferStatus::Accepted,
            offeror: "offeror".into(),
            offeree: "offeree".into(),
            offeror_nft_contract: "offeror_contract".into(),
            offeror_nft: offeror_nft.clone(),
            offeror_code_hash: "offeror_code_hash".to_string(),
            offeree_nft_contract: "offeree_contract".into(),
            offeree_nft: offeree_nft.clone(),
            offeree_code_hash: "offeree_code_hash".to_string(),
            offeror_hands: vec![Hand::Rock, Hand::Paper, Hand::Scissors].into(),
            offeree_hands: vec![Hand::Scissors, Hand::Paper, Hand::Rock].into(),
            offeror_draw_point: -1,
            winner: "offeror".to_string(),
        });

        assert_eq!(expected, res);
    }

    // #[test]
    // fn query_offer() {

    // }
}
