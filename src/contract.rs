use cosmwasm_std::{
    coins, log, to_binary, Api, BankMsg, Binary, Context, CosmosMsg, Empty, Env, Extern,
    HandleResponse, HumanAddr, InitResponse, Querier, StdResult, Storage, WasmMsg,
};

use crate::hand::{rand_hand, Hand, MatchResult};
use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::msg_cw721::HandleMsg as Cw721HandleMsg;
use crate::state::{
    config, config_read, offers, offers_read, read_viewing_key, write_viewing_key, Offer,
    OfferStatus, State,
};
use crate::utils::{calculate_fee, sha_256, Prng};
use crate::validation::{validate_balance, validate_nft, validate_offer_id, validate_offeree};
use crate::viewing_key::ViewingKey;

pub const INVERSE_BASIS_POINT: u64 = 10000;
pub const DEFAULT_FEE_RATE: u64 = 300;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let state = State {
        prng_seed: sha_256(base64::encode(msg.prng_seed.clone()).as_bytes()).to_vec(),
        entropy: msg.prng_seed.as_bytes().to_vec(),
        banker_wallet: env.message.sender.clone(),
        fee_recipient: env.message.sender,
        fee_rate: DEFAULT_FEE_RATE,
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
        HandleMsg::BetToken {
            denom,
            amount,
            hand,
            entropy,
        } => try_bet_token(deps, env, denom, amount, hand, entropy),
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
                offer.offeror_nft.clone()
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

pub fn try_bet_token<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    denom: String,
    amount: u64,
    hand: u8,
    entropy: String,
) -> StdResult<HandleResponse> {
    // check sender balance
    validate_balance(deps, &env.message.sender, &denom, amount.into())?;
    // check banker wallet balance
    let mut state: State = config_read(&deps.storage).load()?;
    validate_balance(deps, &state.banker_wallet, &denom, amount.into())?;

    // generate and save new random bytes
    let rng = Prng::new_rand_bytes(&state.entropy, (&entropy).as_ref());
    state.entropy = rng.clone();
    config(&mut deps.storage).save(&state)?;

    // compete
    let opponent_hand = rand_hand(&rng);
    let result = Hand::from(&hand).compete(&opponent_hand);

    let fee = calculate_fee(amount, state.fee_rate);
    let messages: Vec<CosmosMsg<Empty>> = match &result {
        MatchResult::Win => {
            vec![CosmosMsg::Bank(BankMsg::Send {
                from_address: state.fee_recipient.clone(),
                to_address: env.message.sender.clone(),
                amount: coins((amount - fee).into(), &denom),
            })]
        }
        MatchResult::Draw => {
            vec![CosmosMsg::Bank(BankMsg::Send {
                from_address: env.message.sender.clone(),
                to_address: state.fee_recipient.clone(),
                amount: coins(fee.into(), &denom),
            })]
        }
        MatchResult::Lose => {
            vec![CosmosMsg::Bank(BankMsg::Send {
                from_address: env.message.sender.clone(),
                to_address: state.fee_recipient.clone(),
                amount: coins(amount.into(), &denom),
            })]
        }
    };

    Ok(HandleResponse {
        messages,
        log: vec![log("action", "bet"), log("result", result.to_str())],
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
        // QueryMsg::Offers {} => query_offers(&deps),
    }
}

fn query_offer<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    id: u64,
    address: Option<HumanAddr>,
    viewing_key: Option<String>,
) -> StdResult<Binary> {
    let can_view_hands = match viewing_key {
        Some(viewing_key) => {
            let key = ViewingKey(viewing_key);
            let message_sender = deps.api.canonical_address(&address.unwrap())?;
            let expected_key = read_viewing_key(&deps.storage, &message_sender);
            key.check_viewing_key(expected_key.unwrap().as_slice())
        }
        None => false,
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

// fn query_offers<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<Binary> {
//     let res_data: StdResult<Vec<KV<Offer>>> = offers_read(&deps.storage)
//         .range(None, None, Order::Ascending)
//         .collect();
//     let data = res_data.unwrap();
//     let ids: Vec<u64> = data
//         .iter()
//         .map(|(k, _)| u64::from_be_bytes(to_array::<u8, 8>(k.to_vec())))
//         .collect();
//     to_binary(&OffersResponse { ids })
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hand::{Hand, Hands};
    use crate::mock::{mock_dependencies, MockQuerier};
    use crate::utils::calculate_fee;
    use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
    use cosmwasm_std::{from_binary, Coin, StdError};
    use std::collections::HashMap;

    fn initialize() -> Extern<MockStorage, MockApi, MockQuerier> {
        let owners = HashMap::from([
            ("nft_id_1".to_string(), HumanAddr("nft_owner_1".to_string())),
            ("nft_id_2".to_string(), HumanAddr("nft_owner_2".to_string())),
            ("nft_id_3".to_string(), HumanAddr("nft_owner_3".to_string())),
        ]);
        let balance: &[(&HumanAddr, &[Coin])] = &[
            (&HumanAddr::from("bank_wallet"), &coins(10000, "uscrt")),
            (&HumanAddr::from("bettor_1"), &coins(10000, "uscrt")),
            (&HumanAddr::from("bettor_2"), &coins(10000, "uscrt")),
        ];
        let mut deps = mock_dependencies(balance, Some(owners));
        let msg = InitMsg {
            prng_seed: "prng_seed".to_string(),
        };
        init(&mut deps, mock_env("bank_wallet", &[]), msg).unwrap();
        deps
    }

    fn valid_sample_offer_msg(id: u64) -> HandleMsg {
        HandleMsg::MakeOffer {
            id,
            offeree: "nft_owner_2".into(),
            offeror_nft_contract: "offeror_contract".into(),
            offeror_nft: "nft_id_1".to_string(),
            offeror_code_hash: "offeror_code_hash".to_string(),
            offeree_nft_contract: "offeree_contract".into(),
            offeree_nft: "nft_id_2".to_string(),
            offeree_code_hash: "offeree_code_hash".to_string(),
            offeror_hands: vec![1, 2, 3],
            offeror_draw_point: -1,
        }
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[(&HumanAddr::from(""), &[])], None);
        let msg = InitMsg {
            prng_seed: "prng_seed".to_string(),
        };
        init(&mut deps, mock_env("creator", &[]), msg).unwrap();
    }

    #[test]
    fn try_offer() {
        let mut deps = initialize();

        let offer_id = 100;
        let env = mock_env("nft_owner_1", &[]);
        let msg = valid_sample_offer_msg(offer_id);

        // succeed
        handle(&mut deps, env.clone(), msg.clone()).unwrap();

        // failed by duplicated id
        let res = handle(&mut deps, env.clone(), msg);
        assert_eq!(
            Some(StdError::generic_err(format!(
                "duplicated id({})",
                offer_id
            ))),
            res.err()
        );

        // failed by invalid nft_id
        let msg = HandleMsg::MakeOffer {
            id: offer_id + 1,
            offeree: "nft_owner_2".into(),
            offeror_nft_contract: "contract".into(),
            offeror_nft: "invalid_nft_id".to_string(),
            offeror_code_hash: "code_hash".to_string(),
            offeree_nft_contract: "contract".into(),
            offeree_nft: "nft_id_2".to_string(),
            offeree_code_hash: "code_hash".to_string(),
            offeror_hands: vec![1, 2, 3],
            offeror_draw_point: 2,
        };

        let res = handle(&mut deps, env, msg);
        assert_eq!(true, res.is_err());
    }

    #[test]
    fn try_accept() {
        let mut deps = initialize();

        let offer_id = 100;
        let env = mock_env("nft_owner_1", &[]);
        let msg = valid_sample_offer_msg(offer_id);
        handle(&mut deps, env, msg).unwrap();

        let msg = HandleMsg::AcceptOffer {
            id: offer_id,
            offeree_hands: vec![3, 2, 1],
        };

        // faild by invalid sender
        let env = mock_env("invalid_sender", &[]);
        let res = handle(&mut deps, env, msg.clone());
        assert_eq!(
            Some(StdError::generic_err(
                "msg sender is not offeree(nft_owner_2)"
            )),
            res.err()
        );

        // succeed
        let env = mock_env("nft_owner_2", &[]);
        let res = handle(&mut deps, env, msg).unwrap();
        assert_eq!(1, res.messages.len());

        let transfer_msg = to_binary(&Cw721HandleMsg::TransferNft {
            recipient: "nft_owner_1".into(),
            token_id: "nft_id_2".to_string(),
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
    }

    #[test]
    fn query_offer() {
        let mut deps = initialize();

        let offer_id = 100;
        let env = mock_env("nft_owner_1", &[]);
        let msg = valid_sample_offer_msg(offer_id);
        handle(&mut deps, env.clone(), msg).unwrap();

        // offeror hands hidden
        let msg = QueryMsg::Offer {
            id: offer_id,
            address: None,
            viewing_key: None,
        };
        let res = query(&deps, msg).unwrap();
        let offer: Offer = from_binary(&res).unwrap();
        let expected: Hands = Vec::<Hand>::new().into();
        assert_eq!(expected, offer.offeror_hands);

        // offeror hands shown
        let msg = HandleMsg::GenerateViewingKey {
            entropy: "entropy".to_string(),
            padding: None,
        };
        let res = handle(&mut deps, env.clone(), msg).unwrap();
        let data = res.data.unwrap();
        let key: String = from_binary(&data).unwrap();

        let msg = QueryMsg::Offer {
            id: offer_id,
            address: Some(env.message.sender),
            viewing_key: Some(key),
        };
        let res = query(&deps, msg).unwrap();
        let offer: Offer = from_binary(&res).unwrap();
        let expected: Hands = vec![Hand::Rock, Hand::Paper, Hand::Scissors].into();
        assert_eq!(expected, offer.offeror_hands);

        // both hands shown
        let env = mock_env("nft_owner_2", &[]);
        let msg = HandleMsg::AcceptOffer {
            id: offer_id,
            offeree_hands: vec![2, 3, 3],
        };
        handle(&mut deps, env, msg).unwrap();

        let msg = QueryMsg::Offer {
            id: offer_id,
            address: None,
            viewing_key: None,
        };
        let res = query(&deps, msg).unwrap();
        let offer: Offer = from_binary(&res).unwrap();
        let offeror_expected: Hands = vec![Hand::Rock, Hand::Paper, Hand::Scissors].into();
        assert_eq!(offeror_expected, offer.offeror_hands);
        let offeree_expected: Hands = vec![Hand::Paper, Hand::Scissors, Hand::Scissors].into();
        assert_eq!(offeree_expected, offer.offeree_hands);
    }

    #[test]
    fn bet_token() {
        let mut deps = initialize();
        let denom = "uscrt".to_string();

        let mut pass_win = false;
        let mut pass_draw = false;
        let mut pass_lose = false;
        while !pass_win || !pass_draw || !pass_lose {
            let env = mock_env("bettor_1", &[]);
            let amount = 100;
            let fee = calculate_fee(amount, DEFAULT_FEE_RATE);

            let msg = HandleMsg::BetToken {
                denom: denom.clone(),
                amount,
                hand: 1,
                entropy: "entropy".to_string(),
            };
            let res = handle(&mut deps, env, msg).unwrap();

            assert_eq!(2, res.log.len());
            assert_eq!(1, res.messages.len());

            let result = &res.log[1].value;
            let msg_amount = match &res.messages[0] {
                CosmosMsg::Bank(BankMsg::Send { amount, .. }) => amount[0].amount.u128() as u64,
                _ => panic!("unexpected"),
            };

            if result == "win" {
                pass_win = true;
                assert_eq!(msg_amount, amount - fee);
            } else if result == "draw" {
                pass_draw = true;
                assert_eq!(msg_amount, fee);
            } else if result == "lose" {
                pass_lose = true;
                assert_eq!(msg_amount, amount);
            }
        }
    }

    // #[test]
    // fn query_offers() {
    //     let mut deps = initialize();
    //     let id_1 = 100;
    //     let msg = valid_sample_offer_msg(id_1);
    //     handle(&mut deps, mock_env("nft_owner_1", &[]), msg).unwrap();
    //     let id_2 = 101;
    //     let msg = valid_sample_offer_msg(id_2);
    //     handle(&mut deps, mock_env("nft_owner_1", &[]), msg).unwrap();

    //     let msg = QueryMsg::Offers {};
    //     let res = query(&deps, msg).unwrap();
    //     let offers: OffersResponse = from_binary(&res).unwrap();
    //     assert_eq!(vec![id_1, id_2], offers.ids)
    // }
}
