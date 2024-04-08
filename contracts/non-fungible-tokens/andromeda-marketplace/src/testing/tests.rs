use andromeda_non_fungible_tokens::marketplace::{
    Cw721HookMsg, ExecuteMsg, InstantiateMsg, Status,
};
use andromeda_std::{
    ado_base::modules::Module,
    amp::addresses::AndrAddr,
    common::{
        encode_binary,
        expiration::{expiration_from_milliseconds, MILLISECONDS_TO_NANOSECONDS_RATIO},
        reply::ReplyId,
        Milliseconds,
    },
    error::ContractError,
    os::economics::ExecuteMsg as EconomicsExecuteMsg,
};
use cosmwasm_std::{
    coin, coins,
    testing::{mock_env, mock_info},
    to_json_binary, Addr, BankMsg, CosmosMsg, Deps, DepsMut, Env, Response, SubMsg, Uint128,
    WasmMsg,
};
use cw721::{Cw721ExecuteMsg, Cw721ReceiveMsg};
use cw_utils::Expiration;

use super::mock_querier::MOCK_KERNEL_CONTRACT;
use crate::{
    contract::{execute, instantiate},
    state::{sale_infos, SaleInfo, TokenSaleState, TOKEN_SALE_STATE},
    testing::mock_querier::{
        mock_dependencies_custom, MOCK_RATES_CONTRACT, MOCK_TOKEN_ADDR, MOCK_TOKEN_OWNER,
        MOCK_UNCLAIMED_TOKEN, RATES,
    },
};

fn start_sale(deps: DepsMut) {
    let hook_msg = Cw721HookMsg::StartSale {
        coin_denom: "uusd".to_string(),
        price: Uint128::new(100),
        start_time: None,
        duration: None,
        recipient: None,
    };
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: MOCK_TOKEN_OWNER.to_owned(),
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        msg: encode_binary(&hook_msg).unwrap(),
    });
    let env = mock_env();

    let info = mock_info(MOCK_TOKEN_ADDR, &[]);
    let _res = execute(deps, env, info, msg).unwrap();
}

fn start_sale_future_start(deps: DepsMut, env: Env) {
    let current_time = env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;
    let hook_msg = Cw721HookMsg::StartSale {
        coin_denom: "uusd".to_string(),
        price: Uint128::new(100),
        // Add one to the current time to have it set in the future
        start_time: Some(Milliseconds(current_time + 1)),
        duration: None,
        recipient: None,
    };
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: MOCK_TOKEN_OWNER.to_owned(),
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        msg: encode_binary(&hook_msg).unwrap(),
    });
    let env = mock_env();

    let info = mock_info(MOCK_TOKEN_ADDR, &[]);
    let _res = execute(deps, env, info, msg).unwrap();
}

fn start_sale_future_start_with_duration(deps: DepsMut, env: Env) {
    let current_time = env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;
    let hook_msg = Cw721HookMsg::StartSale {
        coin_denom: "uusd".to_string(),
        price: Uint128::new(100),
        // Add one to the current time to have it set in the future
        start_time: Some(Milliseconds(current_time + 1)),
        // Add duration, the end time's expiration will be current time + duration
        duration: Some(Milliseconds(1)),
        recipient: None,
    };
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: MOCK_TOKEN_OWNER.to_owned(),
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        msg: encode_binary(&hook_msg).unwrap(),
    });
    let env = mock_env();

    let info = mock_info(MOCK_TOKEN_ADDR, &[]);
    let _res = execute(deps, env, info, msg).unwrap();
}

fn init(deps: DepsMut, modules: Option<Vec<Module>>) -> Response {
    let msg = InstantiateMsg {
        owner: None,
        modules,
        kernel_address: MOCK_KERNEL_CONTRACT.to_string(),
    };

    let info = mock_info("owner", &[]);
    instantiate(deps, mock_env(), info, msg).unwrap()
}

fn assert_sale_created(deps: Deps, env: Env) {
    let current_time = env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;
    let start_time_expiration =
        expiration_from_milliseconds(Milliseconds(current_time + 1)).unwrap();
    assert_eq!(
        TokenSaleState {
            coin_denom: "uusd".to_string(),
            sale_id: 1u128.into(),
            owner: MOCK_TOKEN_OWNER.to_string(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_owned(),
            status: Status::Open,
            price: Uint128::new(100),
            // start sale function has start_time set as None, so it defaults to the current time
            start_time: start_time_expiration,
            end_time: Expiration::Never {},
            recipient: None,
        },
        TOKEN_SALE_STATE.load(deps.storage, 1u128).unwrap()
    );

    assert_eq!(
        SaleInfo {
            sale_ids: vec![Uint128::from(1u128)],
            token_address: MOCK_TOKEN_ADDR.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        },
        sale_infos()
            .load(
                deps.storage,
                &(MOCK_UNCLAIMED_TOKEN.to_owned() + MOCK_TOKEN_ADDR)
            )
            .unwrap()
    );
}

fn assert_sale_created_future_start(deps: Deps, env: Env) {
    let current_time = env.block.time.nanos() / MILLISECONDS_TO_NANOSECONDS_RATIO;
    // Add one to the current time to have it set in the future
    let start_time_expiration =
        expiration_from_milliseconds(Milliseconds(current_time + 1)).unwrap();
    assert_eq!(
        TokenSaleState {
            coin_denom: "uusd".to_string(),
            sale_id: 1u128.into(),
            owner: MOCK_TOKEN_OWNER.to_string(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
            token_address: MOCK_TOKEN_ADDR.to_owned(),
            status: Status::Open,
            price: Uint128::new(100),
            start_time: start_time_expiration,
            end_time: Expiration::Never {},
            recipient: None,
        },
        TOKEN_SALE_STATE.load(deps.storage, 1u128).unwrap()
    );

    assert_eq!(
        SaleInfo {
            sale_ids: vec![Uint128::from(1u128)],
            token_address: MOCK_TOKEN_ADDR.to_owned(),
            token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        },
        sale_infos()
            .load(
                deps.storage,
                &(MOCK_UNCLAIMED_TOKEN.to_owned() + MOCK_TOKEN_ADDR)
            )
            .unwrap()
    );
}

#[test]
fn test_sale_instantiate() {
    let mut deps = mock_dependencies_custom(&[]);
    let res = init(deps.as_mut(), None);
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_sale_instantiate_future_start() {
    let mut deps = mock_dependencies_custom(&[]);
    let res = init(deps.as_mut(), None);
    assert_eq!(0, res.messages.len());

    start_sale_future_start(deps.as_mut(), mock_env());
    assert_sale_created_future_start(deps.as_ref(), mock_env());
}

#[test]
fn test_execute_buy_non_existing_sale() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);
    let env = mock_env();
    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_string(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };
    let info = mock_info("buyer", &coins(100, "uusd"));
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::SaleDoesNotExist {}, res.unwrap_err());
}

#[test]
fn test_execute_buy_sale_not_open_already_bought() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();
    let _res = init(deps.as_mut(), None);

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref(), env.clone());

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    // Add one second so that the start_time expires
    env.block.time = env.block.time.plus_seconds(1);
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::SaleExecuted {})
}

#[test]
fn test_execute_buy_sale_not_open_cancelled() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let _res = init(deps.as_mut(), None);

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref(), env.clone());

    let msg = ExecuteMsg::CancelSale {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info(MOCK_TOKEN_OWNER, &[]);
    let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };
    let info = mock_info("sender", &coins(100, "uusd".to_string()));
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::SaleCancelled {})
}

#[test]
fn test_execute_buy_token_owner_cannot_buy() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();

    let _res = init(deps.as_mut(), None);

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref(), env.clone());

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };
    // Add one second so that the start_time expires
    env.block.time = env.block.time.plus_seconds(1);

    let info = mock_info(MOCK_TOKEN_OWNER, &coins(100, "uusd".to_string()));
    let res = execute(deps.as_mut(), env, info, msg);
    assert_eq!(ContractError::TokenOwnerCannotBuy {}, res.unwrap_err());
}

#[test]
fn test_execute_buy_invalid_coins_sent() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();

    let _res = init(deps.as_mut(), None);

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref(), env.clone());

    let error = ContractError::InvalidFunds {
        msg: "Sales ensure! exactly one coin to be sent.".to_string(),
    };
    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_string(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    // No coins sent
    let info = mock_info("sender", &[]);
    // Add one second so that the start_time expires
    env.block.time = env.block.time.plus_seconds(1);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(error, res.unwrap_err());

    // Multiple coins sent
    let info = mock_info("sender", &[coin(100, "uusd"), coin(100, "uluna")]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(error, res.unwrap_err());

    // Invalid denom sent
    let info = mock_info("sender", &[coin(100, "uluna")]);
    let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
    assert_eq!(
        ContractError::InvalidFunds {
            msg: "No uusd assets are provided to sale".to_string(),
        },
        res.unwrap_err()
    );

    // Correct denom but empty
    let info = mock_info("sender", &[coin(0, "uusd")]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert!(matches!(res, ContractError::InvalidFunds { .. }));
}

#[test]
fn test_execute_buy_works() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();

    let _res = init(deps.as_mut(), None);

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref(), env.clone());

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("someone", &coins(100, "uusd".to_string()));
    // Add one second so that the start_time expires
    env.block.time = env.block.time.plus_seconds(1);
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();
}

#[test]
fn test_execute_buy_future_start() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let _res = init(deps.as_mut(), None);

    start_sale_future_start(deps.as_mut(), mock_env());
    assert_sale_created_future_start(deps.as_ref(), mock_env());

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("someone", &coins(100, "uusd".to_string()));
    // The start time is ahead of the current block time, so it should return a Sale Not Started error.
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::SaleNotOpen {})
}

#[test]
fn test_execute_buy_sale_expired() {
    let mut deps = mock_dependencies_custom(&[]);
    let mut env = mock_env();

    let _res = init(deps.as_mut(), None);

    start_sale_future_start_with_duration(deps.as_mut(), mock_env());

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("someone", &coins(100, "uusd".to_string()));
    // Forward block time so that the end time expires
    env.block.time = env.block.time.plus_days(100);

    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::SaleExpired {})
}

#[test]
fn test_execute_update_sale_unauthorized() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let _res = init(deps.as_mut(), None);

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref(), env.clone());

    let msg = ExecuteMsg::UpdateSale {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
        price: Uint128::new(11),
        coin_denom: "juno".to_string(),
        recipient: None,
    };

    let info = mock_info("someone", &[]);
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {})
}

#[test]
fn test_execute_update_sale_invalid_price() {
    let mut deps = mock_dependencies_custom(&[]);
    let env = mock_env();

    let _res = init(deps.as_mut(), None);

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref(), env.clone());

    let msg = ExecuteMsg::UpdateSale {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
        price: Uint128::zero(),
        coin_denom: "juno".to_string(),
        recipient: None,
    };

    let info = mock_info("owner", &[]);
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::InvalidZeroAmount {})
}

#[test]
fn test_execute_start_sale_invalid_price() {
    let mut deps = mock_dependencies_custom(&[]);
    let _res = init(deps.as_mut(), None);

    let hook_msg = Cw721HookMsg::StartSale {
        coin_denom: "uusd".to_string(),
        price: Uint128::zero(),
        start_time: None,
        duration: None,
        recipient: None,
    };
    let msg = ExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: MOCK_TOKEN_OWNER.to_owned(),
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        msg: encode_binary(&hook_msg).unwrap(),
    });
    let env = mock_env();

    let info = mock_info(MOCK_TOKEN_ADDR, &[]);
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(err, ContractError::InvalidZeroAmount {})
}

#[test]
fn test_execute_buy_with_tax_and_royalty_insufficient_funds() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        name: Some(RATES.to_owned()),
        address: AndrAddr::from_string(MOCK_RATES_CONTRACT.to_owned()),
        is_mutable: false,
    }];
    let _res = init(deps.as_mut(), Some(modules));

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref(), mock_env());

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };
    let mut env = mock_env();
    // Add one second so that the start_time expires
    env.block.time = env.block.time.plus_seconds(1);
    let info = mock_info("someone", &coins(100, "uusd".to_string()));
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert!(matches!(err, ContractError::InvalidFunds { .. }));
}

#[test]
fn execute_buy_with_tax_and_royalty_too_many_funds() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        name: Some(RATES.to_owned()),
        address: AndrAddr::from_string(MOCK_RATES_CONTRACT.to_owned()),
        is_mutable: false,
    }];
    let _res = init(deps.as_mut(), Some(modules));

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref(), mock_env());

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };
    let mut env = mock_env();
    // Add one second so that the start_time expires
    env.block.time = env.block.time.plus_seconds(1);

    let info = mock_info("someone", &coins(200, "uusd".to_string()));
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert!(matches!(err, ContractError::InvalidFunds { .. }));
}

#[test]
fn test_execute_buy_with_tax_and_royalty_works() {
    let mut deps = mock_dependencies_custom(&[]);
    let modules = vec![Module {
        name: Some(RATES.to_owned()),
        address: AndrAddr::from_string(MOCK_RATES_CONTRACT.to_owned()),
        is_mutable: false,
    }];
    let _res = init(deps.as_mut(), Some(modules));

    start_sale(deps.as_mut());
    assert_sale_created(deps.as_ref(), mock_env());

    let msg = ExecuteMsg::Buy {
        token_id: MOCK_UNCLAIMED_TOKEN.to_owned(),
        token_address: MOCK_TOKEN_ADDR.to_string(),
    };

    let info = mock_info("someone", &coins(150, "uusd".to_string()));
    let mut env = mock_env();
    // Add one second so that the start_time expires
    env.block.time = env.block.time.plus_seconds(1);

    let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();
    let expected: Vec<SubMsg<_>> = vec![
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "royalty_recipient".to_string(),
            amount: vec![coin(10, "uusd")],
        })),
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "tax_recipient".to_string(),
            amount: vec![coin(50, "uusd")],
        })),
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: MOCK_TOKEN_ADDR.to_string(),
            msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: info.sender.to_string(),
                token_id: MOCK_UNCLAIMED_TOKEN.to_string(),
            })
            .unwrap(),
            funds: vec![],
        })),
        SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "owner".to_string(),
            amount: vec![coin(90, "uusd")],
        })),
        SubMsg::reply_on_error(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "economics_contract".to_string(),
                msg: to_json_binary(&EconomicsExecuteMsg::PayFee {
                    payee: Addr::unchecked("someone"),
                    action: "Buy".to_string(),
                })
                .unwrap(),
                funds: vec![],
            }),
            ReplyId::PayFee.repr(),
        ),
    ];
    assert_eq!(res.messages, expected)
}
