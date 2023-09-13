use crate::error::ContractError;
use soroban_sdk::{contracttype, Address, ConversionError, Env, TryFromVal, Val, Vec};

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Admin = 1,
    Config = 2,
    LpVec = 3,
}

impl TryFromVal<Env, DataKey> for Val {
    type Error = ConversionError;

    fn try_from_val(_env: &Env, v: &DataKey) -> Result<Self, Self::Error> {
        Ok((*v as u32).into())
    }
}

#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum PairType {
    Xyk = 0,
}
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub token_a: Address,
    pub token_b: Address,
    pub share_token: Address,
    pub stake_contract: Address,
    pub pair_type: PairType,
    /// The total fees (in bps) charged by a pair of this type.
    /// In relation to the returned amount of tokens
    pub total_fee_bps: i64,
    pub fee_recipient: Address,
    /// The maximum amount of slippage (in bps) that is tolerated during providing liquidity
    pub max_allowed_slippage_bps: i64,
    /// The maximum amount of spread (in bps) that is tolerated during swap
    pub max_allowed_spread_bps: i64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    /// Address of the asset
    pub address: Address,
    /// The total amount of those tokens in the pool
    pub amount: i128,
}

/// This struct is used to return a query result with the total amount of LP tokens and assets in a specific pool.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PoolResponse {
    /// The asset A in the pool together with asset amounts
    pub asset_a: Asset,
    /// The asset B in the pool together with asset amounts
    pub asset_b: Asset,
    /// The total amount of LP tokens currently issued
    pub asset_lp_share: Asset,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiquidityPoolInfo {
    pub pool_response: PoolResponse,
    pub total_fee_bps: i64,
}

pub fn save_admin(env: &Env, address: Address) {
    env.storage().instance().set(&DataKey::Admin, &address);
}

pub fn get_admin(env: &Env) -> Result<Address, ContractError> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(ContractError::FailedToGetAdminAddrFromStorage)
}

pub fn get_lp_vec(env: &Env) -> Result<Vec<Address>, ContractError> {
    env.storage()
        .instance()
        .get(&DataKey::LpVec)
        .ok_or(ContractError::LiquidityPoolVectorNotFound)
}

pub fn save_lp_vec(env: &Env, lp_info: Vec<Address>) {
    env.storage().instance().set(&DataKey::LpVec, &lp_info);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "HostError: Error(Context, MissingValue)")]
    fn test_get_admin_should_panic_when_no_admin_saved() {
        let env = Env::default();

        get_admin(&env).unwrap();
    }

    #[test]
    #[should_panic(expected = "HostError: Error(Context, MissingValue)")]
    fn test_get_lp_vec_should_panic_when_no_vec_saved() {
        let env = Env::default();

        get_lp_vec(&env).unwrap();
    }
}
