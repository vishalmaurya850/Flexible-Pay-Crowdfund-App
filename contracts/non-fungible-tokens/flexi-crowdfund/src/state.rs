use andromeda_non_fungible_tokens::crowdfund::{Config, State};
use andromeda_std::error::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Order, Storage, SubMsg, Uint128};
use cw_storage_plus::{Bound, Item, Map};

/// The config.
pub const CONFIG: Item<Config> = Item::new("config");

/// The number of tokens available for sale.
pub const NUMBER_OF_TOKENS_AVAILABLE: Item<Uint128> = Item::new("number_of_tokens_available");

/// Sale started if and only if STATE.may_load is Some and !duration.is_expired()
pub const STATE: Item<State> = Item::new("state");

/// Relates buyer address to vector of purchases.
pub const PURCHASES: Map<&str, Vec<Purchase>> = Map::new("buyers");

/// Contains token ids that have not been purchased.
pub const AVAILABLE_TOKENS: Map<&str, bool> = Map::new("available_tokens");

/// Is set to true when at least one sale has been conducted. This is used to disallow minting if
/// config.can_mint_after_sale is false.
pub const SALE_CONDUCTED: Item<bool> = Item::new("sale_conducted");

#[cw_serde]
pub struct Purchase {
    /// The token id being purchased.
    pub token_id: String,
    /// Amount of tax paid.
    pub tax_amount: Uint128,
    /// sub messages for sending funds for rates.
    pub msgs: Vec<SubMsg>,
    /// The purchaser of the token.
    pub purchaser: String,
}

const MAX_LIMIT: u32 = 50;
const DEFAULT_LIMIT: u32 = 20;
pub(crate) fn get_available_tokens(
    storage: &dyn Storage,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<String>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.as_deref().map(Bound::exclusive);
    let tokens: Result<Vec<String>, ContractError> = AVAILABLE_TOKENS
        .keys(storage, start, None, Order::Ascending)
        .take(limit)
        .map(|token| Ok(token?))
        .collect();
    tokens
}
