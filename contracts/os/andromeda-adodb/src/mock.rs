#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]

use crate::contract::{execute, instantiate, query};
use andromeda_std::os::adodb::{ActionFee, ExecuteMsg, InstantiateMsg, QueryMsg};
use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_andromeda_adodb() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_adodb_instantiate_msg(
    kernel_address: impl Into<String>,
    owner: Option<String>,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address: kernel_address.into(),
        owner,
    }
}

pub fn mock_publish(
    code_id: u64,
    ado_type: impl Into<String>,
    version: impl Into<String>,
    publisher: Option<String>,
    action_fees: Option<Vec<ActionFee>>,
) -> ExecuteMsg {
    ExecuteMsg::Publish {
        code_id,
        ado_type: ado_type.into(),
        version: version.into(),
        publisher,
        action_fees,
    }
}

pub fn mock_unpublish(ado_type: impl Into<String>, version: impl Into<String>) -> ExecuteMsg {
    ExecuteMsg::Unpublish {
        ado_type: ado_type.into(),
        version: version.into(),
    }
}

/// Used to generate a Code ID query message
pub fn mock_get_code_id_msg(code_id_key: String) -> QueryMsg {
    QueryMsg::CodeId { key: code_id_key }
}
