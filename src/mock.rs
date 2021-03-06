use cosmwasm_std::testing::{BankQuerier, MockApi, MockStorage};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Coin, Empty, Extern, HumanAddr, Querier, QuerierResult,
    QueryRequest, SystemError, WasmQuery,
};
use std::collections::HashMap;

use crate::msg_cw721::{QueryAnswer, QueryMsg as Cw721QueryMsg};

pub fn mock_dependencies(
    balances: &[(&HumanAddr, &[Coin])],
    owners: Option<HashMap<String, HumanAddr>>,
) -> Extern<MockStorage, MockApi, MockQuerier> {
    Extern {
        storage: MockStorage::default(),
        api: MockApi::new(20),
        querier: MockQuerier::new(balances, owners),
    }
}

pub struct MockQuerier {
    bank: BankQuerier,
    wasm: WasmQuerier,
}

impl MockQuerier {
    pub fn new(
        balances: &[(&HumanAddr, &[Coin])],
        _owners: Option<HashMap<String, HumanAddr>>,
    ) -> Self {
        let owners = match _owners {
            Some(owners) => owners,
            None => HashMap::new(),
        };
        MockQuerier {
            bank: BankQuerier::new(balances),
            wasm: WasmQuerier { owners },
        }
    }
}

impl Querier for MockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

impl MockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Bank(bank_query) => self.bank.query(bank_query),
            QueryRequest::Wasm(msg) => self.wasm.query(msg),
            _ => Err(SystemError::UnsupportedRequest {
                kind: format!("only wasm supporting, request: {:?}", request),
            }),
        }
    }
}

#[derive(Clone, Default)]
pub struct WasmQuerier {
    pub owners: HashMap<String, HumanAddr>,
}

impl WasmQuerier {
    pub fn query(&self, request: &WasmQuery) -> QuerierResult {
        let msg = match request {
            WasmQuery::Smart { msg, .. } => msg,
            _ => {
                return Err(SystemError::UnsupportedRequest {
                    kind: format!("only smart supporting, request: {:?}", request),
                })
            }
        };

        let query: Cw721QueryMsg = from_binary(&msg).unwrap();
        let token_id = match query {
            Cw721QueryMsg::OwnerOf { token_id, .. } => token_id,
        };

        let owner = match self.owners.get(&token_id) {
            Some(owner) => owner,
            None => {
                return Err(SystemError::InvalidRequest {
                    error: format!("Unable to find token info for {}", token_id),
                    request: to_binary(&token_id).unwrap(),
                })
            }
        };

        Ok(to_binary(&QueryAnswer::OwnerOf {
            owner: owner.clone(),
            approvals: vec![],
        }))
    }
}
