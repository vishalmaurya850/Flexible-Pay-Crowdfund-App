use super::ADOContract;
use crate::{ado_base::modules::Module, error::ContractError};
use cosmwasm_std::{ensure, DepsMut, MessageInfo, Response, Storage, Uint64};

impl<'a> ADOContract<'a> {
    /// Registers all modules within the ADO contract
    pub fn instantiate_modules(
        &self,
        storage: &mut dyn Storage,
        modules: Vec<Module>,
    ) -> Result<Response, ContractError> {
        let mut resp = Response::default();
        for module in modules {
            let idx = self.register_module(storage, &module)?;
            resp = resp.add_attribute("module_idx", idx.to_string());
        }
        Ok(resp)
    }

    /// An execute wrapper for `fn register_module`. The parameters are "extracted" from `DepsMut` to be able to
    /// execute this in a loop without cloning.
    pub(crate) fn execute_register_module(
        &self,
        storage: &mut dyn Storage,
        sender: &str,
        module: Module,
        should_validate: bool,
    ) -> Result<Response, ContractError> {
        ensure!(
            self.is_owner_or_operator(storage, sender)?,
            ContractError::Unauthorized {}
        );
        let resp = Response::default();
        let idx = self.register_module(storage, &module)?;
        if should_validate {
            self.validate_modules(&self.load_modules(storage)?)?;
        }
        Ok(resp
            .add_attribute("action", "register_module")
            .add_attribute("module_idx", idx.to_string()))
    }

    /// An execute wrapper for `fn alter_module`.
    pub(crate) fn execute_alter_module(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        module_idx: Uint64,
        module: Module,
    ) -> Result<Response, ContractError> {
        let addr = info.sender.as_str();
        ensure!(
            self.is_owner_or_operator(deps.storage, addr)?,
            ContractError::Unauthorized {}
        );
        self.alter_module(deps.storage, module_idx, &module)?;
        self.validate_modules(&self.load_modules(deps.storage)?)?;
        Ok(Response::default()
            .add_attribute("action", "alter_module")
            .add_attribute("module_idx", module_idx))
    }

    /// A wrapper for `fn deregister_module`.
    pub(crate) fn execute_deregister_module(
        &self,
        deps: DepsMut,
        info: MessageInfo,
        module_idx: Uint64,
    ) -> Result<Response, ContractError> {
        let addr = info.sender.as_str();
        ensure!(
            self.is_owner_or_operator(deps.storage, addr)?,
            ContractError::Unauthorized {}
        );
        self.deregister_module(deps.storage, module_idx)?;
        Ok(Response::default()
            .add_attribute("action", "deregister_module")
            .add_attribute("module_idx", module_idx))
    }
}
