use soroban_sdk::{
    contracttype, log, xdr::ToXdr, Address, Bytes, BytesN, ConversionError, Env, RawVal, Symbol,
    TryFromVal,
};

use crate::{error::ContractError, token_contract};

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    TotalShares = 0,
    ReserveA = 1,
    ReserveB = 2,
}

impl TryFromVal<Env, DataKey> for RawVal {
    type Error = ConversionError;

    fn try_from_val(_env: &Env, v: &DataKey) -> Result<Self, Self::Error> {
        Ok((*v as u32).into())
    }
}

#[contracttype]
#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum PairType {
    Xyk = 0,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Config {
    pub token_a: Address,
    pub token_b: Address,
    pub share_token: Address,
    pub pair_type: PairType,
}
const CONFIG: Symbol = Symbol::short("CONFIG");

pub fn get_config(env: &Env) -> Result<Config, ContractError> {
    match env.storage().get(&CONFIG) {
        Some(config) => config.map_err(|_| ContractError::FailedToLoadFromStorage),
        None => Err(ContractError::ConfigNotSet),
    }
}

pub fn save_config(env: &Env, config: Config) {
    env.storage().set(&CONFIG, &config);
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    /// Address of the asset
    pub address: Address,
    /// The total amount of those tokens in the pool
    pub amount: u128,
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

pub mod utils {
    use super::*;

    pub fn deploy_token_contract(
        e: &Env,
        token_wasm_hash: &BytesN<32>,
        token_a: &Address,
        token_b: &Address,
    ) -> Address {
        let mut salt = Bytes::new(e);
        salt.append(&token_a.to_xdr(e));
        salt.append(&token_b.to_xdr(e));
        let salt = e.crypto().sha256(&salt);
        e.deployer()
            .with_current_contract(&salt)
            .deploy(token_wasm_hash)
    }

    pub fn save_total_shares(e: &Env, amount: u128) {
        e.storage().set(&DataKey::TotalShares, &amount)
    }

    pub fn save_pool_balance_a(e: &Env, amount: u128) {
        e.storage().set(&DataKey::ReserveA, &amount)
    }

    pub fn save_pool_balance_b(e: &Env, amount: u128) {
        e.storage().set(&DataKey::ReserveB, &amount)
    }

    pub fn mint_shares(
        e: &Env,
        share_token: Address,
        to: Address,
        amount: u128,
    ) -> Result<(), ContractError> {
        let total = get_total_shares(e)?;

        token_contract::Client::new(e, &share_token).mint(&to, &(amount as i128));

        save_total_shares(e, total + amount);
        Ok(())
    }

    // queries
    pub fn get_total_shares(e: &Env) -> Result<u128, ContractError> {
        e.storage()
            .get_unchecked(&DataKey::TotalShares)
            .map_err(|_| ContractError::FailedToLoadFromStorage)
    }
    pub fn get_pool_balance_a(e: &Env) -> Result<u128, ContractError> {
        e.storage()
            .get_unchecked(&DataKey::ReserveA)
            .map_err(|_| ContractError::FailedToLoadFromStorage)
    }

    pub fn get_pool_balance_b(e: &Env) -> Result<u128, ContractError> {
        e.storage()
            .get_unchecked(&DataKey::ReserveB)
            .map_err(|_| ContractError::FailedToLoadFromStorage)
    }

    pub fn get_balance(e: &Env, contract: &Address) -> i128 {
        token_contract::Client::new(e, contract).balance(&e.current_contract_address())
    }

    pub fn get_deposit_amounts(
        env: &Env,
        desired_a: u128,
        min_a: u128,
        desired_b: u128,
        min_b: u128,
        pool_balance_a: u128,
        pool_balance_b: u128,
    ) -> Result<(u128, u128), ContractError> {
        if pool_balance_a == 0 && pool_balance_b == 0 {
            return Ok((desired_a, desired_b));
        }

        // determines the amount of asset B proportionally based on the desired amount of asset A
        let amount_b = desired_a * pool_balance_b / pool_balance_a;
        if amount_b <= desired_b {
            if amount_b < min_b {
                log!(
                    env,
                    "Deposit amount for asset B ({}) is less than the minimum requirement ({})",
                    amount_b,
                    min_b
                );
                return Err(ContractError::DepositAmountBLessThenMin);
            }
            Ok((desired_a, amount_b))
        } else {
            // as above
            let amount_a = desired_b * pool_balance_a / pool_balance_b;
            if amount_a > desired_a || desired_a < min_a {
                log!(env, "Deposit amount for asset A ({}) is invalid. Either it exceeds the desired amount ({}) or falls below the minimum requirement ({})", amount_a, desired_a, min_a);
                return Err(ContractError::DepositAmountAExceedsOrBelowMin);
            }
            Ok((amount_a, desired_b))
        }
    }
}