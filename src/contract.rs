use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, Context, Env, Extern, HandleResponse, HumanAddr,
    InitResponse, Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use secrete_nft::msg::HandleMsg as SecreteHandleMsg;

use crate::hand::MatchResult;
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{offers, offers_read, Offer};

pub fn init<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    debug_print!("Contract was initialized by {}", env.message.sender);
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
            offeror_nft_contract,
            offeror_nft,
            offeree_nft_contract,
            offeree_nft,
            offeror_hands,
            offeror_draw_point,
        } => try_offer(
            deps,
            env,
            id,
            offeror_nft_contract,
            offeror_nft,
            offeree_nft_contract,
            offeree_nft,
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
    offeror_nft_contract: HumanAddr,
    offeror_nft: Uint128,
    offeree_nft_contract: HumanAddr,
    offeree_nft: Uint128,
    hands: Vec<u8>,
    draw_point: u8,
) -> StdResult<HandleResponse> {
    match offers(&mut deps.storage).may_load(&id.to_be_bytes()) {
        Ok(None) => {}
        _ => return Err(StdError::generic_err(format!("duplicated id({})", id))),
    }

    let offer = Offer::new(
        id,
        env.message.sender.clone(),
        offeror_nft_contract,
        offeror_nft,
        offeree_nft_contract,
        offeree_nft,
        hands,
        draw_point,
    );

    offers(&mut deps.storage).save(&id.to_be_bytes(), &offer)?;

    debug_print("successfully offerd");
    Ok(HandleResponse::default())
}

pub fn try_accept<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    id: u64,
    hands: Vec<u8>,
) -> StdResult<HandleResponse> {
    let mut offer = offers(&mut deps.storage).load(&id.to_be_bytes())?;
    offer.accept_offer(env.message.sender.clone(), hands);

    let offeror_hands = &offer.offeror_hands;
    let offeree_hands = &offer.offeree_hands;

    let result = offeror_hands.compete(offeree_hands, offer.offeror_draw_point);

    let mut ctx = Context::new();
    ctx.add_log("action", "competed");

    if result.eq(&MatchResult::Win) {
        offer.winner = "offeror".to_string();
        let msg = to_binary(&SecreteHandleMsg::TransferFrom {
            sender: offer.offeree.clone(),
            recipient: offer.offeror.clone(),
            token_id: offer.offeree_nft,
        })?;
        ctx.add_message(WasmMsg::Execute {
            contract_addr: offer.offeree_nft_contract.clone(),
            callback_code_hash: "".to_string(),
            msg,
            send: vec![],
        });
    } else if result.eq(&MatchResult::Lose) {
        offer.winner = "offeree".to_string();
        let msg = to_binary(&SecreteHandleMsg::TransferFrom {
            sender: offer.offeror.clone(),
            recipient: offer.offeree.clone(),
            token_id: offer.offeror_nft,
        })?;
        ctx.add_message(WasmMsg::Execute {
            contract_addr: offer.offeror_nft_contract.clone(),
            callback_code_hash: "".to_string(),
            msg,
            send: vec![],
        });
    } else {
        offer.winner = "draw".to_string();
    };

    ctx.add_log("winner", &offer.winner);

    offers(&mut deps.storage).update(&id.to_be_bytes(), |_| Ok(offer.clone()))?;

    debug_print("successfully accepted");
    Ok(ctx.into())
}

pub fn try_decline<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    id: u64,
) -> StdResult<HandleResponse> {
    let mut offer = offers(&mut deps.storage).load(&id.to_be_bytes())?;
    offer.decline_offer(env.message.sender.clone());

    offers(&mut deps.storage).update(&id.to_be_bytes(), |_| Ok(offer))?;

    debug_print("successfully declined");
    Ok(HandleResponse::default())
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
    let offer = offers_read(&deps.storage).may_load(&id.to_be_bytes());
    to_binary(&offer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{CosmosMsg, StdError};
    use secrete_nft::msg::HandleMsg as SecreteHandleMsg;

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
            offeror_nft_contract: "offeror_contract".into(),
            offeror_nft: (1 as u64).into(),
            offeree_nft_contract: "offeree_contract".into(),
            offeree_nft: (1 as u64).into(),
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
    }

    #[test]
    fn try_accept() {
        let mut deps = mock_dependencies(20, &[]);
        let msg = InitMsg {};
        let env = mock_env("creator", &[]);
        init(&mut deps, env, msg).unwrap();

        let offer_id = 123;
        let offeror_nft: u64 = 123;
        let offeree_nft: u64 = 321;
        let env = mock_env("offeror", &[]);
        let msg = HandleMsg::MakeOffer {
            id: offer_id,
            offeror_nft_contract: "offeror_contract".into(),
            offeror_nft: offeror_nft.into(),
            offeree_nft_contract: "offeree_contract".into(),
            offeree_nft: offeree_nft.into(),
            offeror_hands: vec![1, 2, 3],
            offeror_draw_point: 2,
        };

        handle(&mut deps, env.clone(), msg.clone()).unwrap();

        let env = mock_env("offeree", &[]);
        let msg = HandleMsg::AcceptOffer {
            id: offer_id,
            offeree_hands: vec![3, 2, 1],
        };

        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(1, res.messages.len());

        let transfer_msg = to_binary(&SecreteHandleMsg::TransferFrom {
            sender: "offeree".into(),
            recipient: "offeror".into(),
            token_id: offeree_nft.into(),
        })
        .unwrap();
        let msg: CosmosMsg = WasmMsg::Execute {
            contract_addr: "offeree_contract".into(),
            callback_code_hash: "".to_string(),
            msg: transfer_msg,
            send: vec![],
        }
        .into();

        assert_eq!(msg, res.messages[0]);
    }
}
