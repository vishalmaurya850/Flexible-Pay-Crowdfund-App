use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct VersionResponse {
    pub version: String,
}
