use cosmwasm_std::{
    log, to_binary, Api, Binary, Context, Env, Extern, HandleResponse, HumanAddr, InitResponse,
    Querier, StdResult, Storage, WasmMsg,
};
use secret_toolkit::crypto::sha_256;
use snip721_reference_impl::msg::HandleMsg as Cw721HandleMsg;

use crate::hand::{Hand, MatchResult};
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{
    config, config_read, offers, offers_read, read_viewing_key, write_viewing_key, Offer,
    OfferStatus, State,
};
use crate::validation::{validate_nft, validate_offer_id, validate_offeree};
use crate::viewing_key::ViewingKey;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        prng_seed: sha_256(base64::encode(msg.prng_seed).as_bytes()).to_vec(),
    };
    config(&mut deps.storage).save(&state)?;

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
        HandleMsg::GenerateViewingKey { entropy, .. } => {
            try_generate_viewing_key(deps, env, entropy)
        }
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
    validate_offer_id(&deps, id)?;
    validate_nft(
        &deps,
        offeror_nft_contract.clone(),
        offeror_nft.clone(),
        offeror_code_hash.clone(),
        env.message.sender.clone(),
    )?;
    validate_nft(
        &deps,
        offeree_nft_contract.clone(),
        offeree_nft.clone(),
        offeree_code_hash.clone(),
        offeree.clone(),
    )?;

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

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "offered")],
        data: None,
    })
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

    let mut ctx = Context::new();
    ctx.add_log("action", "accepted");

    let result = offeror_hands.compete(offeree_hands, offer.offeror_draw_point);

    if result.eq(&MatchResult::Draw) {
        offer.winner = "draw".to_string();
    } else {
        offer.winner = if result.eq(&MatchResult::Win) {
            "offeror".to_string()
        } else {
            "offeree".to_string()
        };
        let msg = to_binary(&Cw721HandleMsg::TransferNft {
            recipient: if result.eq(&MatchResult::Win) {
                offer.offeror.clone()
            } else {
                offer.offeree.clone()
            },
            token_id: if result.eq(&MatchResult::Win) {
                offer.offeree_nft.clone()
            } else {
                offer.offeree_nft.clone()
            },
            memo: None,
            padding: None,
        })?;
        ctx.add_message(WasmMsg::Execute {
            contract_addr: if result.eq(&MatchResult::Win) {
                offer.offeree_nft_contract.clone()
            } else {
                offer.offeror_nft_contract.clone()
            },
            callback_code_hash: if result.eq(&MatchResult::Win) {
                offer.offeree_code_hash.clone()
            } else {
                offer.offeror_code_hash.clone()
            },
            msg,
            send: vec![],
        });
    }

    offers(&mut deps.storage).update(&id.to_be_bytes(), |_| Ok(offer))?;

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

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "declined")],
        data: None,
    })
}

pub fn try_generate_viewing_key<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    entropy: String,
) -> StdResult<HandleResponse> {
    let config: State = config_read(&deps.storage).load()?;
    let key = ViewingKey::new(&env, &config.prng_seed, (&entropy).as_ref());

    let message_sender = deps.api.canonical_address(&env.message.sender)?;
    write_viewing_key(&mut deps.storage, &message_sender, &key);

    Ok(HandleResponse {
        messages: vec![],
        log: vec![log("action", "generated")],
        data: Some(to_binary(&key)?),
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Offer {
            id,
            address,
            viewing_key,
        } => query_offer(&deps, id, address, viewing_key),
    }
}

fn query_offer<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    id: u64,
    address: HumanAddr,
    viewing_key: String,
) -> StdResult<Binary> {
    let key = ViewingKey(viewing_key);
    let message_sender = deps.api.canonical_address(&address)?;
    let expected_key = read_viewing_key(&deps.storage, &message_sender);
    let can_view_hands = if key.check_viewing_key(expected_key.unwrap().as_slice()) {
        true
    } else {
        false
    };
    match offers_read(&deps.storage).may_load(&id.to_be_bytes()) {
        Ok(Some(mut o)) => {
            let hide_hands = o.status == OfferStatus::Offered && !can_view_hands;
            if hide_hands {
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
        let mut deps = mock_dependencies(0, &[]);
        let msg = InitMsg {
            prng_seed: "prng_seed".to_string(),
        };
        init(&mut deps, mock_env("creator", &[]), msg).unwrap();
    }

    // #[test]
    // fn try_offer() {
    //     let mut deps = mock_dependencies(20, &[]);
    //     let msg = InitMsg {};
    //     let env = mock_env("creator", &[]);
    //     init(&mut deps, env, msg).unwrap();

    //     let offer_id = 123;
    //     let env = mock_env("offeror", &[]);
    //     let msg = HandleMsg::MakeOffer {
    //         id: offer_id,
    //         offeree: "offeree".into(),
    //         offeror_nft_contract: "offeror_contract".into(),
    //         offeror_nft: "1".to_string(),
    //         offeror_code_hash: "offeror_code_hash".to_string(),
    //         offeree_nft_contract: "offeree_contract".into(),
    //         offeree_nft: "2".to_string(),
    //         offeree_code_hash: "offeree_code_hash".to_string(),
    //         offeror_hands: vec![1, 2, 3],
    //         offeror_draw_point: 2,
    //     };

    //     // succeed
    //     handle(&mut deps, env.clone(), msg.clone()).unwrap();

    //     // failed by duplicated id
    //     let res = handle(&mut deps, env, msg);
    //     assert_eq!(
    //         Some(StdError::generic_err(format!(
    //             "duplicated id({})",
    //             offer_id
    //         ))),
    //         res.err()
    //     );

    //     // check query
    //     let msg = QueryMsg::Offer { id: offer_id };
    //     let res = query(&deps, msg);
    //     let expected = to_binary(&Offer {
    //         id: offer_id,
    //         status: OfferStatus::Offered,
    //         offeror: "offeror".into(),
    //         offeree: "offeree".into(),
    //         offeror_nft_contract: "offeror_contract".into(),
    //         offeror_nft: "1".to_string(),
    //         offeror_code_hash: "offeror_code_hash".to_string(),
    //         offeree_nft_contract: "offeree_contract".into(),
    //         offeree_nft: "2".to_string(),
    //         offeree_code_hash: "offeree_code_hash".to_string(),
    //         offeror_hands: Vec::<Hand>::new().into(),
    //         offeree_hands: Vec::<Hand>::new().into(),
    //         offeror_draw_point: 2,
    //         winner: "".to_string(),
    //     });

    //     assert_eq!(expected, res);
    // }

    // #[test]
    // fn try_accept() {
    //     let mut deps = mock_dependencies(20, &[]);
    //     let msg = InitMsg {};
    //     let env = mock_env("creator", &[]);
    //     init(&mut deps, env, msg).unwrap();

    //     let offer_id = 123;
    //     let offeror_nft = "123".to_string();
    //     let offeree_nft = "321".to_string();
    //     let env = mock_env("offeror", &[]);
    //     let msg = HandleMsg::MakeOffer {
    //         id: offer_id,
    //         offeree: "offeree".into(),
    //         offeror_nft_contract: "offeror_contract".into(),
    //         offeror_nft: offeror_nft.clone(),
    //         offeror_code_hash: "offeror_code_hash".to_string(),
    //         offeree_nft_contract: "offeree_contract".into(),
    //         offeree_nft: offeree_nft.clone(),
    //         offeree_code_hash: "offeree_code_hash".to_string(),
    //         offeror_hands: vec![1, 2, 3],
    //         offeror_draw_point: -1,
    //     };

    //     handle(&mut deps, env.clone(), msg.clone()).unwrap();

    //     let env = mock_env("offeree", &[]);
    //     let msg = HandleMsg::AcceptOffer {
    //         id: offer_id,
    //         offeree_hands: vec![3, 2, 1],
    //     };

    //     let res = handle(&mut deps, env, msg).unwrap();
    //     assert_eq!(1, res.messages.len());

    //     let transfer_msg = to_binary(&Cw721HandleMsg::TransferNft {
    //         recipient: "offeror".into(),
    //         token_id: offeree_nft.clone(),
    //         memo: None,
    //         padding: None,
    //     })
    //     .unwrap();
    //     let msg: CosmosMsg = WasmMsg::Execute {
    //         contract_addr: "offeree_contract".into(),
    //         callback_code_hash: "offeree_code_hash".to_string(),
    //         msg: transfer_msg,
    //         send: vec![],
    //     }
    //     .into();

    //     assert_eq!(msg, res.messages[0]);

    //     // check query
    //     let msg = QueryMsg::Offer { id: offer_id };
    //     let res = query(&deps, msg);
    //     let expected = to_binary(&Offer {
    //         id: offer_id,
    //         status: OfferStatus::Accepted,
    //         offeror: "offeror".into(),
    //         offeree: "offeree".into(),
    //         offeror_nft_contract: "offeror_contract".into(),
    //         offeror_nft: offeror_nft.clone(),
    //         offeror_code_hash: "offeror_code_hash".to_string(),
    //         offeree_nft_contract: "offeree_contract".into(),
    //         offeree_nft: offeree_nft.clone(),
    //         offeree_code_hash: "offeree_code_hash".to_string(),
    //         offeror_hands: vec![Hand::Rock, Hand::Paper, Hand::Scissors].into(),
    //         offeree_hands: vec![Hand::Scissors, Hand::Paper, Hand::Rock].into(),
    //         offeror_draw_point: -1,
    //         winner: "offeror".to_string(),
    //     });

    //     assert_eq!(expected, res);
    // }

    // #[test]
    // fn query_offer() {

    // }
}
