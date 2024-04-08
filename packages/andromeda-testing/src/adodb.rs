use crate::{mock::MockApp, mock_ado, mock_contract::ExecuteResult, MockADO, MockContract};
use andromeda_adodb::mock::*;
use andromeda_std::os::adodb::{ActionFee, ExecuteMsg, QueryMsg};
use cosmwasm_std::Addr;
use cw_multi_test::Executor;

pub struct MockADODB(Addr);
mock_ado!(MockADODB, ExecuteMsg, QueryMsg);

impl MockADODB {
    pub fn instantiate(
        app: &mut MockApp,
        code_id: u64,
        sender: Addr,
        owner: Option<String>,
        kernel_address: String,
    ) -> Self {
        let msg = mock_adodb_instantiate_msg(kernel_address, owner);
        let res = app.instantiate_contract(
            code_id,
            sender.clone(),
            &msg,
            &[],
            "ADO DB",
            Some(sender.to_string()),
        );

        Self(res.unwrap())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn execute_publish(
        &self,
        app: &mut MockApp,
        sender: Addr,
        code_id: u64,
        ado_type: impl Into<String>,
        version: impl Into<String>,
        publisher: Option<String>,
        action_fees: Option<Vec<ActionFee>>,
    ) -> ExecuteResult {
        let msg = mock_publish(code_id, ado_type, version, publisher, action_fees);

        self.execute(app, &msg, sender, &[])
    }

    pub fn query_code_id(&self, app: &mut MockApp, key: impl Into<String>) -> u64 {
        let msg = mock_get_code_id_msg(key.into());
        let res: u64 = self.query(app, msg);

        res
    }
}
