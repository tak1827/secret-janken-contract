use cosmwasm_std::{
    debug_print, to_binary, Api, Binary, Context, Env, Extern, HandleResponse, InitResponse,
    Querier, StdError, StdResult, Storage,
};

use crate::hand::MatchResult;
use crate::msg::{CountResponse, CustomMsg, HandleMsg, InitMsg, QueryMsg};
use crate::state::{config_read, offers, offers_read, Offer};

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
) -> StdResult<HandleResponse<CustomMsg>> {
    match msg {
        HandleMsg::MakeOffer {
            id,
            offeror_nft,
            offeree_nft,
            offeror_hands,
            offeror_draw_point,
        } => try_offer(
            deps,
            env,
            id,
            offeror_nft,
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
    offeror_nft: String,
    offeree_nft: String,
    hands: Vec<u8>,
    draw_point: u8,
) -> StdResult<HandleResponse<CustomMsg>> {
    match offers(&mut deps.storage).may_load(&id.to_be_bytes()) {
        Ok(None) => {}
        _ => return Err(StdError::generic_err(format!("duplicated id({})", id))),
    }

    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    let offer = Offer::new(
        id,
        sender_address_raw,
        offeror_nft,
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
) -> StdResult<HandleResponse<CustomMsg>> {
    let mut offer = offers(&mut deps.storage).load(&id.to_be_bytes())?;
    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    offer.accept_offer(sender_address_raw, hands);

    offers(&mut deps.storage).update(&id.to_be_bytes(), |_| Ok(offer.clone()))?;

    let offeror_hands = &offer.offeror_hands;
    let offeree_hands = &offer.offeree_hands;

    let result = offeror_hands.compete(offeree_hands, offer.offeror_draw_point);
    let winner = if result.eq(&MatchResult::Win) {
        "offeree".to_string()
    } else if result.eq(&MatchResult::Lose) {
        "offeror".to_string()
    } else {
        "draw".to_string()
    };

    let msg = CustomMsg::MatchResult {
        winner: winner.clone(),
        offeror_hands: offeror_hands.to_u8_vec(),
        offeree_hands: offeree_hands.to_u8_vec(),
    };

    let mut ctx = Context::new();
    ctx.add_log("action", "competed");
    ctx.add_log("winner", &winner);
    ctx.add_message(msg);

    debug_print("successfully accepted");
    Ok(ctx.into())
}

pub fn try_decline<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    id: u64,
) -> StdResult<HandleResponse<CustomMsg>> {
    let mut offer = offers(&mut deps.storage).load(&id.to_be_bytes())?;
    let sender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    offer.decline_offer(sender_address_raw);

    offers(&mut deps.storage).update(&id.to_be_bytes(), |_| Ok(offer))?;

    debug_print("successfully declined");
    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetCount {} => to_binary(&query_count(deps)?),
    }
}

fn query_count<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<CountResponse> {
    let state = config_read(&deps.storage).load()?;
    Ok(CountResponse { count: state.count })
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

        // we can just call .unwrap() to assert this was a success
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
            offeror_nft: "offeror_nft".to_string(),
            offeree_nft: "offeree_nft".to_string(),
            offeror_hands: vec![1, 2, 3],
            offeror_draw_point: 3,
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
        let env = mock_env("offeror", &[]);
        let msg = HandleMsg::MakeOffer {
            id: offer_id,
            offeror_nft: "offeror_nft".to_string(),
            offeree_nft: "offeree_nft".to_string(),
            offeror_hands: vec![1, 2, 3],
            offeror_draw_point: 3,
        };

        handle(&mut deps, env.clone(), msg.clone()).unwrap();

        let env = mock_env("offeree", &[]);
        let msg = HandleMsg::AcceptOffer {
            id: offer_id,
            offeree_hands: vec![3, 2, 1],
        };

        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(1, res.messages.len());

        let msg: CosmosMsg<CustomMsg> = CustomMsg::MatchResult {
            winner: "draw".to_string(),
            offeror_hands: vec![1, 2, 3],
            offeree_hands: vec![3, 2, 1],
        }
        .into();
        assert_eq!(msg, res.messages[0]);
    }
}
