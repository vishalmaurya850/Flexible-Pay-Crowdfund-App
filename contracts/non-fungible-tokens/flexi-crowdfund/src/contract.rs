use crate::state::{
    get_available_tokens, Purchase, AVAILABLE_TOKENS, CONFIG, NUMBER_OF_TOKENS_AVAILABLE,
    PURCHASES, SALE_CONDUCTED, STATE,
};
use andromeda_sdk::{ Address, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use andromeda_std::{Cw721, Cw721ExecuteMsg, Cw721QueryMsg, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
    pub price: Coin,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Contribute {},
    Withdraw {},
    Mint {
        token_id: String,
        token_uri: String,
        extension: Extension,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Extension {
    publisher: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StartSaleMsg {
    pub expiration: Expiration,
    pub price: Coin,
    pub min_tokens_sold: Uint128,
    pub max_amount_per_wallet: Option<u32>,
    pub recipient: String,
}

#[multiversx_sc::message]
pub fn start_sale(&self, msg: StartSaleMsg) -> StdResult<()> {
    if self.sale_started() {
        return Err(StdError::generic_err("Sale has already been started"));
    }

    if msg.expiration.is_expired() {
        return Err(StdError::generic_err("Expiration time must be in the future"));
    }

    if msg.price.amount <= 0 {
        return Err(StdError::generic_err("Price must be greater than zero"));
    }

    if msg.min_tokens_sold <= 0 {
        return Err(StdError::generic_err("Minimum number of tokens to be sold must be greater than zero"));
    }

    if msg.max_amount_per_wallet <= 0 {
        return Err(StdError::generic_err("Default token amount is 1"));
    }

    self.sale_params().set(&SaleParams {
        expiration: msg.expiration,
        price: msg.price,
        min_tokens_sold: msg.min_tokens_sold,
        recipient: msg.recipient,
    })?;

    self.sale_started().set(&true)?;

    Ok(())
}

pub fn instantiate(deps: DepsMut,env: Env,info: MessageInfo,msg: InstantiateMsg,) -> StdResult<Response> {
    let owner = deps.api.canonical_address(&msg.owner)?;
    Ok(Response::new()
       .add_attribute("owner", owner)
       .add_attribute("price", msg.price))
}

pub fn execute(deps: DepsMut,env: Env,info: MessageInfo,msg: ExecuteMsg,) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Contribute {} => contribute(deps, info),
        ExecuteMsg::Withdraw {} => withdraw(deps, info),
        ExecuteMsg::Mint {
            token_id,
            token_uri,
            extension,
        } => mint(deps, info, token_id, token_uri, extension),
    }
}

fn contribute(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    // Implement the contribute logic here
    Ok(Response::new())
}

fn withdraw(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    // Implement the withdraw logic here
    Ok(Response::new())
}

fn mint(
    deps: DepsMut,
    info: MessageInfo,
    token_id: String,
    token_uri: String,
    extension: Extension,
) -> StdResult<Response> {
    // Implement the mint logic here
    Ok(Response::new())
}
use andromeda_non_fungible_tokens::{
    crowdfund::{Config, CrowdfundMintMsg, ExecuteMsg, InstantiateMsg, QueryMsg, State},
    cw721::{ExecuteMsg as Cw721ExecuteMsg, MintMsg, QueryMsg as Cw721QueryMsg},
};
use andromeda_std::{
    ado_base::ownership::OwnershipMessage,
    amp::{messages::AMPPkt, recipient::Recipient, AndrAddr},
    common::{
        actions::call_action,
        expiration::{expiration_from_milliseconds, get_and_validate_start_time},
        Milliseconds,
    },
};
use andromeda_std::{ado_contract::ADOContract, common::context::ExecuteContext};

use andromeda_std::{
    ado_base::{hooks::AndromedaHook, InstantiateMsg as BaseInstantiateMsg, MigrateMsg},
    common::{deduct_funds, encode_binary, merge_sub_msgs, rates::get_tax_amount, Funds},
    error::ContractError,
};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coins, ensure, has_coins, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Order, QuerierWrapper, QueryRequest, Reply, Response, StdError, Storage, SubMsg, Uint128,
    WasmMsg, WasmQuery,
};
use cw721::{ContractInfoResponse, TokensResponse};
use cw_utils::nonpayable;
use std::cmp;

const MAX_LIMIT: u32 = 100;
const DEFAULT_LIMIT: u32 = 50;
pub(crate) const MAX_MINT_LIMIT: u32 = 100;
const CONTRACT_NAME: &str = "crates.io:andromeda-crowdfund";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    CONFIG.save(
        deps.storage,
        &Config {
            token_address: msg.token_address,
            can_mint_after_sale: msg.can_mint_after_sale,
        },
    )?;
    SALE_CONDUCTED.save(deps.storage, &false)?;
    NUMBER_OF_TOKENS_AVAILABLE.save(deps.storage, &Uint128::zero())?;
    let inst_resp = ADOContract::default().instantiate(
        deps.storage,
        env,
        deps.api,
        &deps.querier,
        info,
        BaseInstantiateMsg {
            ado_type: CONTRACT_NAME.to_string(),
            ado_version: CONTRACT_VERSION.to_string(),
            kernel_address: msg.kernel_address,
            owner: msg.owner,
        },
    )?;
    let owner = ADOContract::default().owner(deps.storage)?;
    let mod_resp =
        ADOContract::default().register_modules(owner.as_str(), deps.storage, msg.modules)?;

    Ok(inst_resp
        .add_attributes(mod_resp.attributes)
        .add_submessages(mod_resp.messages))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        return Err(ContractError::Std(StdError::generic_err(
            msg.result.unwrap_err(),
        )));
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let ctx = ExecuteContext::new(deps, info, env);

    match msg {
        ExecuteMsg::AMPReceive(pkt) => {
            ADOContract::default().execute_amp_receive(ctx, pkt, handle_execute)
        }
        _ => handle_execute(ctx, msg),
    }
}

pub fn handle_execute(mut ctx: ExecuteContext, msg: ExecuteMsg) -> Result<Response, ContractError> {
    let contract = ADOContract::default();
    let action_response = call_action(
        &mut ctx.deps,
        &ctx.info,
        &ctx.env,
        &ctx.amp_ctx,
        msg.as_ref(),
    )?;
    if !matches!(msg, ExecuteMsg::UpdateAppContract { .. })
        && !matches!(
            msg,
            ExecuteMsg::Ownership(OwnershipMessage::UpdateOwner { .. })
        )
    {
        contract.module_hook::<Response>(
            &ctx.deps.as_ref(),
            AndromedaHook::OnExecute {
                sender: ctx.info.sender.to_string(),
                payload: encode_binary(&msg)?,
            },
        )?;
    }
    let res = match msg {
        ExecuteMsg::Mint(mint_msgs) => execute_mint(ctx, mint_msgs),
        ExecuteMsg::StartSale {
            start_time,
            end_time,
            price,
            min_tokens_sold,
            max_amount_per_wallet,
            recipient,
        } => execute_start_sale(
            ctx,
            start_time,
            end_time,
            price,
            min_tokens_sold,
            max_amount_per_wallet,
            recipient,
        ),
        ExecuteMsg::Purchase { number_of_tokens } => execute_purchase(ctx, number_of_tokens),
        ExecuteMsg::PurchaseByTokenId { token_id } => execute_purchase_by_token_id(ctx, token_id),
        ExecuteMsg::ClaimRefund {} => execute_claim_refund(ctx),
        ExecuteMsg::EndSale { limit } => execute_end_sale(ctx, limit),
        ExecuteMsg::UpdateTokenContract { address } => execute_update_token_contract(ctx, address),
        _ => ADOContract::default().execute(ctx, msg),
    }?;
    Ok(res
        .add_submessages(action_response.messages)
        .add_attributes(action_response.attributes)
        .add_events(action_response.events))
}

fn execute_mint(
    ctx: ExecuteContext,
    mint_msgs: Vec<CrowdfundMintMsg>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;
    nonpayable(&info)?;

    ensure!(
        mint_msgs.len() <= MAX_MINT_LIMIT as usize,
        ContractError::TooManyMintMessages {
            limit: MAX_MINT_LIMIT,
        }
    );
    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    // Can only mint when no sale is ongoing.
    ensure!(
        STATE.may_load(deps.storage)?.is_none(),
        ContractError::SaleStarted {}
    );
    let sale_conducted = SALE_CONDUCTED.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    ensure!(
        config.can_mint_after_sale || !sale_conducted,
        ContractError::CannotMintAfterSaleConducted {}
    );

    let token_contract = config.token_address;
    let crowdfund_contract = env.contract.address.to_string();
    let resolved_path = token_contract.get_raw_address(&deps.as_ref())?;

    let mut resp = Response::new();
    for mint_msg in mint_msgs {
        let mint_resp = mint(
            deps.storage,
            &crowdfund_contract,
            resolved_path.to_string(),
            mint_msg,
        )?;
        resp = resp
            .add_attributes(mint_resp.attributes)
            .add_submessages(mint_resp.messages);
    }

    Ok(resp)
}

fn mint(
    storage: &mut dyn Storage,
    crowdfund_contract: &str,
    token_contract: String,
    mint_msg: CrowdfundMintMsg,
) -> Result<Response, ContractError> {
    let mint_msg: MintMsg = MintMsg {
        token_id: mint_msg.token_id,
        owner: mint_msg
            .owner
            .unwrap_or_else(|| crowdfund_contract.to_owned()),
        token_uri: mint_msg.token_uri,
        extension: mint_msg.extension,
    };
    // We allow for owners other than the contract, incase the creator wants to set aside a few
    // tokens for some other use, say airdrop, team allocation, etc.  Only those which have the
    // contract as the owner will be available to sell.
    if mint_msg.owner == crowdfund_contract {
        // Mark token as available to purchase in next sale.
        AVAILABLE_TOKENS.save(storage, &mint_msg.token_id, &true)?;
        let current_number = NUMBER_OF_TOKENS_AVAILABLE.load(storage)?;
        NUMBER_OF_TOKENS_AVAILABLE.save(storage, &(current_number + Uint128::new(1)))?;
    }
    Ok(Response::new()
        .add_attribute("action", "mint")
        .add_message(WasmMsg::Execute {
            contract_addr: token_contract,
            msg: encode_binary(&Cw721ExecuteMsg::Mint {
                token_id: mint_msg.token_id,
                owner: mint_msg.owner,
                token_uri: mint_msg.token_uri,
                extension: mint_msg.extension,
            })?,
            funds: vec![],
        }))
}

fn execute_update_token_contract(
    ctx: ExecuteContext,
    address: AndrAddr,
) -> Result<Response, ContractError> {
    let ExecuteContext { deps, info, .. } = ctx;
    nonpayable(&info)?;

    let contract = ADOContract::default();
    ensure!(
        contract.is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    // Ensure no tokens have been minted already
    let num_tokens = NUMBER_OF_TOKENS_AVAILABLE
        .load(deps.storage)
        .unwrap_or(Uint128::zero());
    ensure!(num_tokens.is_zero(), ContractError::Unauthorized {});

    // Will error if not a valid path
    let addr = address.get_raw_address(&deps.as_ref())?;
    let query = Cw721QueryMsg::ContractInfo {};

    // Check contract is a valid CW721 contract
    let res: Result<ContractInfoResponse, StdError> = deps.querier.query_wasm_smart(addr, &query);
    ensure!(res.is_ok(), ContractError::Unauthorized {});

    CONFIG.update(deps.storage, |mut config| {
        config.token_address = address;
        Ok::<_, ContractError>(config)
    })?;
    Ok(Response::new().add_attribute("action", "update_token_contract"))
}

#[allow(clippy::too_many_arguments)]
fn execute_start_sale(
    ctx: ExecuteContext,
    start_time: Option<Milliseconds>,
    end_time: Milliseconds,
    price: Coin,
    min_tokens_sold: Uint128,
    max_amount_per_wallet: Option<u32>,
    recipient: Recipient,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;
    recipient.validate(&deps.as_ref())?;
    nonpayable(&info)?;
    let ado_contract = ADOContract::default();

    // Validate recipient
    ado_contract.validate_andr_addresses(&deps.as_ref(), vec![recipient.address.clone()])?;
    ensure!(
        ADOContract::default().is_contract_owner(deps.storage, info.sender.as_str())?,
        ContractError::Unauthorized {}
    );
    // If start time wasn't provided, it will be set as the current_time
    let (start_expiration, _current_time) = get_and_validate_start_time(&env, start_time)?;

    let end_expiration = expiration_from_milliseconds(end_time)?;

    ensure!(
        end_expiration > start_expiration,
        ContractError::StartTimeAfterEndTime {}
    );

    SALE_CONDUCTED.save(deps.storage, &true)?;
    let state = STATE.may_load(deps.storage)?;
    ensure!(state.is_none(), ContractError::SaleStarted {});
    let max_amount_per_wallet = max_amount_per_wallet.unwrap_or(1u32);

    // This is to prevent cloning price.
    let price_str = price.to_string();
    STATE.save(
        deps.storage,
        &State {
            end_time: end_expiration,
            price,
            min_tokens_sold,
            max_amount_per_wallet,
            amount_sold: Uint128::zero(),
            amount_to_send: Uint128::zero(),
            amount_transferred: Uint128::zero(),
            recipient,
        },
    )?;

    SALE_CONDUCTED.save(deps.storage, &true)?;

    Ok(Response::new()
        .add_attribute("action", "start_sale")
        .add_attribute("start_time", start_expiration.to_string())
        .add_attribute("end_time", end_expiration.to_string())
        .add_attribute("price", price_str)
        .add_attribute("min_tokens_sold", min_tokens_sold)
        .add_attribute("max_amount_per_wallet", max_amount_per_wallet.to_string()))
}

fn execute_purchase_by_token_id(
    ctx: ExecuteContext,
    token_id: String,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        mut deps,
        info,
        env,
        ..
    } = ctx;
    let sender = info.sender.to_string();
    let state = STATE.may_load(deps.storage)?;

    // CHECK :: That there is an ongoing sale.
    ensure!(state.is_some(), ContractError::NoOngoingSale {});

    let mut state = state.unwrap();
    ensure!(
        !state.end_time.is_expired(&env.block),
        ContractError::NoOngoingSale {}
    );

    let mut purchases = PURCHASES
        .may_load(deps.storage, &sender)?
        .unwrap_or_default();

    ensure!(
        AVAILABLE_TOKENS.has(deps.storage, &token_id),
        ContractError::TokenNotAvailable {}
    );

    let max_possible = state.max_amount_per_wallet - purchases.len() as u32;

    // CHECK :: The user is able to purchase these without going over the limit.
    ensure!(max_possible > 0, ContractError::PurchaseLimitReached {});

    purchase_tokens(
        &mut deps,
        vec![token_id.clone()],
        &info,
        &mut state,
        &mut purchases,
    )?;

    STATE.save(deps.storage, &state)?;
    PURCHASES.save(deps.storage, &sender, &purchases)?;

    Ok(Response::new()
        .add_attribute("action", "purchase")
        .add_attribute("token_id", token_id))
}

fn execute_purchase(
    ctx: ExecuteContext,
    number_of_tokens: Option<u32>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        mut deps,
        info,
        env,
        ..
    } = ctx;
    let sender = info.sender.to_string();
    let state = STATE.may_load(deps.storage)?;

    // CHECK :: That there is an ongoing sale.
    ensure!(state.is_some(), ContractError::NoOngoingSale {});

    let mut state = state.unwrap();
    ensure!(
        !state.end_time.is_expired(&env.block),
        ContractError::NoOngoingSale {}
    );

    let mut purchases = PURCHASES
        .may_load(deps.storage, &sender)?
        .unwrap_or_default();

    let max_possible = state.max_amount_per_wallet - purchases.len() as u32;

    // CHECK :: The user is able to purchase these without going over the limit.
    ensure!(max_possible > 0, ContractError::PurchaseLimitReached {});

    let number_of_tokens_wanted =
        number_of_tokens.map_or(max_possible, |n| cmp::min(n, max_possible));

    // The number of token ids here is equal to min(number_of_tokens_wanted, num_tokens_left).
    let token_ids = get_available_tokens(deps.storage, None, Some(number_of_tokens_wanted))?;

    let number_of_tokens_purchased = token_ids.len();

    let required_payment =
        purchase_tokens(&mut deps, token_ids, &info, &mut state, &mut purchases)?;

    PURCHASES.save(deps.storage, &sender, &purchases)?;
    STATE.save(deps.storage, &state)?;

    // Refund user if they sent more. This can happen near the end of the sale when they weren't
    // able to get the amount that they wanted.
    let mut funds = info.funds;
    deduct_funds(&mut funds, &required_payment)?;

    // If any funds were remaining after deduction, send refund.
    let resp = if has_coins(&funds, &Coin::new(1, state.price.denom)) {
        Response::new().add_message(BankMsg::Send {
            to_address: sender,
            amount: funds,
        })
    } else {
        Response::new()
    };

    Ok(resp
        .add_attribute("action", "purchase")
        .add_attribute(
            "number_of_tokens_wanted",
            number_of_tokens_wanted.to_string(),
        )
        .add_attribute(
            "number_of_tokens_purchased",
            number_of_tokens_purchased.to_string(),
        ))
}

fn purchase_tokens(
    deps: &mut DepsMut,
    token_ids: Vec<String>,
    info: &MessageInfo,
    state: &mut State,
    purchases: &mut Vec<Purchase>,
) -> Result<Coin, ContractError> {
    // CHECK :: There are any tokens left to purchase.
    ensure!(!token_ids.is_empty(), ContractError::AllTokensPurchased {});

    let number_of_tokens_purchased = token_ids.len();

    // CHECK :: The user has sent enough funds to cover the base fee (without any taxes).
    let total_cost = Coin::new(
        state.price.amount.u128() * number_of_tokens_purchased as u128,
        state.price.denom.clone(),
    );
    ensure!(
        has_coins(&info.funds, &total_cost),
        ContractError::InsufficientFunds {}
    );

    let mut total_tax_amount = Uint128::zero();

    // This is the same for each token, so we only need to do it once.
    let (msgs, _events, remainder) = ADOContract::default().on_funds_transfer(
        &deps.as_ref(),
        info.sender.to_string(),
        Funds::Native(state.price.clone()),
        encode_binary(&"")?,
    )?;

    let mut current_number = NUMBER_OF_TOKENS_AVAILABLE.load(deps.storage)?;
    for token_id in token_ids {
        let remaining_amount = remainder.try_get_coin()?;

        let tax_amount = get_tax_amount(&msgs, state.price.amount, remaining_amount.amount);

        let purchase = Purchase {
            token_id: token_id.clone(),
            tax_amount,
            msgs: msgs.clone(),
            purchaser: info.sender.to_string(),
        };
        total_tax_amount = total_tax_amount.checked_add(tax_amount)?;

        state.amount_to_send = state.amount_to_send.checked_add(remaining_amount.amount)?;
        state.amount_sold = state.amount_sold.checked_add(Uint128::one())?;

        purchases.push(purchase);

        AVAILABLE_TOKENS.remove(deps.storage, &token_id);
        current_number = current_number.checked_sub(Uint128::one())?;
    }
    NUMBER_OF_TOKENS_AVAILABLE.save(deps.storage, &current_number)?;

    // CHECK :: User has sent enough to cover taxes.
    let required_payment = Coin {
        denom: state.price.denom.clone(),
        amount: state
            .price
            .amount
            .checked_mul(Uint128::from(number_of_tokens_purchased as u128))?
            .checked_add(total_tax_amount)?,
    };
    ensure!(
        has_coins(&info.funds, &required_payment),
        ContractError::InsufficientFunds {}
    );
    Ok(required_payment)
}

fn execute_claim_refund(ctx: ExecuteContext) -> Result<Response, ContractError> {
    let ExecuteContext {
        deps, info, env, ..
    } = ctx;
    nonpayable(&info)?;

    let state = STATE.may_load(deps.storage)?;
    ensure!(state.is_some(), ContractError::NoOngoingSale {});
    let state = state.unwrap();
    ensure!(
        state.end_time.is_expired(&env.block),
        ContractError::SaleNotEnded {}
    );
    ensure!(
        state.amount_sold < state.min_tokens_sold,
        ContractError::MinSalesExceeded {}
    );

    let purchases = PURCHASES.may_load(deps.storage, info.sender.as_str())?;
    ensure!(purchases.is_some(), ContractError::NoPurchases {});
    let purchases = purchases.unwrap();
    let refund_msg = process_refund(deps.storage, &purchases, &state.price);
    let mut resp = Response::new();
    if let Some(refund_msg) = refund_msg {
        resp = resp.add_message(refund_msg);
    }

    Ok(resp.add_attribute("action", "claim_refund"))
}

fn execute_end_sale(ctx: ExecuteContext, limit: Option<u32>) -> Result<Response, ContractError> {
    let ExecuteContext {
        mut deps,
        info,
        env,
        amp_ctx,
    } = ctx;
    nonpayable(&info)?;

    let state = STATE.may_load(deps.storage)?;
    ensure!(state.is_some(), ContractError::NoOngoingSale {});
    let state = state.unwrap();
    let number_of_tokens_available = NUMBER_OF_TOKENS_AVAILABLE.load(deps.storage)?;
    // In case the minimum sold tokens threshold is met, it has to be the owner who calls the function
    let contract = ADOContract::default();
    let has_minimum_sold = state.min_tokens_sold <= state.amount_sold;
    let is_owner = contract.is_contract_owner(deps.storage, info.sender.as_str())?;

    ensure!(
        // If all tokens have been sold the sale can be ended too.
        state.end_time.is_expired(&env.block)
            || number_of_tokens_available.is_zero()
            || (has_minimum_sold && is_owner),
        ContractError::SaleNotEnded {}
    );
    if state.amount_sold < state.min_tokens_sold {
        issue_refunds_and_burn_tokens(&mut deps, env, limit)
    } else {
        transfer_tokens_and_send_funds(
            ExecuteContext {
                deps,
                info,
                env,
                amp_ctx,
            },
            limit,
        )
    }
}

fn issue_refunds_and_burn_tokens(
    deps: &mut DepsMut,
    env: Env,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    ensure!(limit > 0, ContractError::LimitMustNotBeZero {});
    let mut refund_msgs: Vec<CosmosMsg> = vec![];
    // Issue refunds for `limit` number of users.
    let purchases: Vec<Vec<Purchase>> = PURCHASES
        .range(deps.storage, None, None, Order::Ascending)
        .take(limit)
        .flatten()
        .map(|(_v, p)| p)
        .collect();
    for purchase_vec in purchases.iter() {
        let refund_msg = process_refund(deps.storage, purchase_vec, &state.price);
        if let Some(refund_msg) = refund_msg {
            refund_msgs.push(refund_msg);
        }
    }

    // Burn `limit` number of tokens
    let burn_msgs = get_burn_messages(deps, env.contract.address.to_string(), limit)?;

    if burn_msgs.is_empty() && purchases.is_empty() {
        // When all tokens have been burned and all purchases have been refunded, the sale is over.
        clear_state(deps.storage)?;
    }

    Ok(Response::new()
        .add_attribute("action", "issue_refunds_and_burn_tokens")
        .add_messages(refund_msgs)
        .add_messages(burn_msgs))
}

fn transfer_tokens_and_send_funds(
    ctx: ExecuteContext,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let ExecuteContext {
        mut deps,
        info,
        env,
        ..
    } = ctx;
    let mut state = STATE.load(deps.storage)?;
    let mut resp = Response::new();
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    ensure!(limit > 0, ContractError::LimitMustNotBeZero {});
    // Send the funds if they haven't been sent yet and if all of the tokens have been transferred.
    let mut pkt = match ctx.amp_ctx {
        Some(pkt) => pkt,
        None => AMPPkt::new(info.sender, env.contract.address.clone(), vec![]),
    };

    if state.amount_transferred == state.amount_sold {
        if state.amount_to_send > Uint128::zero() {
            let funds = vec![Coin {
                denom: state.price.denom.clone(),
                amount: state.amount_to_send,
            }];
            match state.recipient.msg {
                None => {
                    resp = resp.add_submessage(
                        state.recipient.generate_direct_msg(&deps.as_ref(), funds)?,
                    );
                }
                Some(_) => {
                    let amp_message = state
                        .recipient
                        .generate_amp_msg(&deps.as_ref(), Some(funds))
                        .unwrap();
                    pkt = pkt.add_message(amp_message);
                    let kernel_address = ADOContract::default().get_kernel_address(deps.storage)?;
                    let sub_msg = pkt.to_sub_msg(
                        kernel_address,
                        Some(coins(
                            state.amount_to_send.u128(),
                            state.price.denom.clone(),
                        )),
                        1,
                    )?;
                    resp = resp.add_submessage(sub_msg);
                }
            }
            state.amount_to_send = Uint128::zero();
            STATE.save(deps.storage, &state)?;
        }
        // Once all purchased tokens have been transferred, begin burning `limit` number of tokens
        // that were not purchased.
        let burn_msgs = get_burn_messages(&mut deps, env.contract.address.to_string(), limit)?;

        if burn_msgs.is_empty() {
            // When burn messages are empty, we have finished the sale, which is represented by
            // having no State.
            clear_state(deps.storage)?;
        } else {
            resp = resp.add_messages(burn_msgs);
        }
        // If we are here then there are no purchases to process so we can exit.
        return Ok(resp.add_attribute("action", "transfer_tokens_and_send_funds"));
    }
    let mut purchases: Vec<Purchase> = PURCHASES
        .range(deps.storage, None, None, Order::Ascending)
        .flatten()
        // Flatten Vec<Vec<Purchase>> into Vec<Purchase>.
        .flat_map(|(_v, p)| p)
        // Take one extra in order to compare what the next purchaser would be to check if some
        // purchases will be left over.
        .take(limit + 1)
        .collect();

    let config = CONFIG.load(deps.storage)?;
    let mut rate_messages: Vec<SubMsg> = vec![];
    let mut transfer_msgs: Vec<CosmosMsg> = vec![];

    let last_purchaser = if purchases.len() == 1 {
        purchases[0].purchaser.clone()
    } else {
        purchases[purchases.len() - 2].purchaser.clone()
    };
    // This subtraction is no problem as we will always have at least one purchase.
    let subsequent_purchase = &purchases[purchases.len() - 1];
    // If this is false, then there are some purchases that we will need to leave for the next
    // round. Otherwise, we are able to process all of the purchases for the last purchaser and we
    // can remove their entry from the map entirely.
    let remove_last_purchaser = last_purchaser != subsequent_purchase.purchaser;

    let mut number_of_last_purchases_removed = 0;
    // If we took an extra element, we remove it. Otherwise limit + 1 was more than was necessary
    // so we need to remove all of the purchases from the map.
    if limit + 1 == purchases.len() {
        // This is an O(1) operation from looking at the source code.
        purchases.pop();
    }

    // Resolve the token contract address from the VFS
    let token_contract_address = config.token_address.get_raw_address(&deps.as_ref())?;
    for purchase in purchases.into_iter() {
        let purchaser = purchase.purchaser;
        let should_remove = purchaser != last_purchaser || remove_last_purchaser;
        if should_remove && PURCHASES.has(deps.storage, &purchaser) {
            PURCHASES.remove(deps.storage, &purchaser);
        } else if purchaser == last_purchaser {
            // Keep track of the number of purchases removed from the last purchaser to remove them
            // at the end, if not all of them were removed.
            number_of_last_purchases_removed += 1;
        }
        rate_messages.extend(purchase.msgs);
        transfer_msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_contract_address.to_string(),
            msg: encode_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: AndrAddr::from_string(purchaser),
                token_id: purchase.token_id,
            })?,
            funds: vec![],
        }));
        state.amount_transferred = state.amount_transferred.checked_add(Uint128::one())?;
    }
    // If the last purchaser wasn't removed, remove the subset of purchases that were processed.
    if PURCHASES.has(deps.storage, &last_purchaser) {
        let last_purchases = PURCHASES.load(deps.storage, &last_purchaser)?;
        PURCHASES.save(
            deps.storage,
            &last_purchaser,
            &last_purchases[number_of_last_purchases_removed..].to_vec(),
        )?;
    }
    STATE.save(deps.storage, &state)?;

    Ok(resp
        .add_attribute("action", "transfer_tokens_and_send_funds")
        .add_messages(transfer_msgs)
        .add_submessages(merge_sub_msgs(rate_messages)))
}

/// Processes a vector of purchases for the SAME user by merging all funds into a single BankMsg.
/// The given purchaser is then removed from `PURCHASES`.
///
/// ## Arguments
/// * `storage`  - Mutable reference to Storage
/// * `purchase` - Vector of purchases for the same user to issue a refund message for.
/// * `price`    - The price of a token
///
/// Returns an `Option<CosmosMsg>` which is `None` when the amount to refund is zero.
fn process_refund(
    storage: &mut dyn Storage,
    purchases: &[Purchase],
    price: &Coin,
) -> Option<CosmosMsg> {
    let purchaser = purchases[0].purchaser.clone();
    // Remove each entry as they get processed.
    PURCHASES.remove(storage, &purchaser);
    // Reduce a user's purchases into one message. While the tax paid on each item should
    // be the same, it is not guaranteed given that the rates module is mutable during the
    // sale.
    let amount = purchases
        .iter()
        // This represents the total amount of funds they sent for each purchase.
        .map(|p| p.tax_amount + price.amount)
        // Adds up all of the purchases.
        .reduce(|accum, item| accum + item)
        .unwrap_or_else(Uint128::zero);

    if amount > Uint128::zero() {
        Some(CosmosMsg::Bank(BankMsg::Send {
            to_address: purchaser,
            amount: vec![Coin {
                denom: price.denom.clone(),
                amount,
            }],
        }))
    } else {
        None
    }
}

fn get_burn_messages(
    deps: &mut DepsMut,
    address: String,
    limit: usize,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let token_address = config.token_address.get_raw_address(&deps.as_ref())?;
    let tokens_to_burn = query_tokens(&deps.querier, token_address.to_string(), address, limit)?;

    tokens_to_burn
        .into_iter()
        .map(|token_id| {
            // Any token that is burnable has been added to this map, and so must be removed.
            AVAILABLE_TOKENS.remove(deps.storage, &token_id);
            Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_address.to_string(),
                funds: vec![],
                msg: encode_binary(&Cw721ExecuteMsg::Burn { token_id })?,
            }))
        })
        .collect()
}

fn clear_state(storage: &mut dyn Storage) -> Result<(), ContractError> {
    STATE.remove(storage);
    NUMBER_OF_TOKENS_AVAILABLE.save(storage, &Uint128::zero())?;

    Ok(())
}

fn query_tokens(
    querier: &QuerierWrapper,
    token_address: String,
    owner: String,
    limit: usize,
) -> Result<Vec<String>, ContractError> {
    let res: TokensResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: token_address,
        msg: encode_binary(&Cw721QueryMsg::Tokens {
            owner,
            start_after: None,
            limit: Some(limit as u32),
        })?,
    }))?;
    Ok(res.tokens)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::State {} => encode_binary(&query_state(deps)?),
        QueryMsg::Config {} => encode_binary(&query_config(deps)?),
        QueryMsg::AvailableTokens { start_after, limit } => {
            encode_binary(&query_available_tokens(deps, start_after, limit)?)
        }
        QueryMsg::IsTokenAvailable { id } => encode_binary(&query_is_token_available(deps, id)),
        _ => ADOContract::default().query(deps, env, msg),
    }
}

fn query_state(deps: Deps) -> Result<State, ContractError> {
    Ok(STATE.load(deps.storage)?)
}

fn query_config(deps: Deps) -> Result<Config, ContractError> {
    Ok(CONFIG.load(deps.storage)?)
}

fn query_available_tokens(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<String>, ContractError> {
    get_available_tokens(deps.storage, start_after, limit)
}

fn query_is_token_available(deps: Deps, id: String) -> bool {
    AVAILABLE_TOKENS.has(deps.storage, &id)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ADOContract::default().migrate(deps, CONTRACT_NAME, CONTRACT_VERSION)
}
