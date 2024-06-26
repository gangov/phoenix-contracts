use curve::{Curve, SaturatingLinear};
use soroban_sdk::{
    contracttype, log, panic_with_error, Address, ConversionError, Env, String, TryFromVal, Val,
};

use crate::error::ContractError;

impl TryFromVal<Env, DataKey> for Val {
    type Error = ConversionError;

    fn try_from_val(_env: &Env, v: &DataKey) -> Result<Self, Self::Error> {
        Ok((*v as u32).into())
    }
}

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DataKey {
    Admin = 1,
    Config = 2,
    Minter = 3,
    Whitelist = 4,
    VestingTokenInfo = 5,
    MaxVestingComplexity = 6,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VestingTokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u32,
    pub address: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VestingBalance {
    pub rcpt_address: Address,
    pub distribution_info: DistributionInfo,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinterInfo {
    pub address: Address,
    pub mint_capacity: u128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistributionInfo {
    pub start_timestamp: u64,
    pub end_timestamp: u64,
    pub amount: u128, // this is fine. this will be constant for historical data checking
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VestingInfo {
    pub balance: u128, // This is the value that we will update during transfer msgs
    pub distribution_info: DistributionInfo,
}

impl MinterInfo {
    pub fn get_curve(&self) -> Curve {
        Curve::Constant(self.mint_capacity)
    }
}

impl DistributionInfo {
    pub fn get_curve(&self) -> Curve {
        Curve::SaturatingLinear(SaturatingLinear {
            min_x: self.start_timestamp,
            min_y: self.amount,
            max_x: self.end_timestamp,
            max_y: 0u128,
        })
    }
}

pub fn save_admin(env: &Env, admin: &Address) {
    env.storage().persistent().set(&DataKey::Admin, admin);
}

pub fn get_admin(env: &Env) -> Address {
    env.storage()
        .persistent()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| {
            log!(&env, "Vesting: Get admin: Critical error - No admin found");
            panic_with_error!(env, ContractError::NoAdminFound);
        })
}

pub fn save_vesting(env: &Env, address: &Address, vesting_info: &VestingInfo) {
    env.storage().persistent().set(address, vesting_info);
}

pub fn get_vesting(env: &Env, address: &Address) -> VestingInfo {
    env.storage().persistent().get(address).unwrap_or_else(|| {
        log!(&env, "Vesting: Get vesting schedule: Critical error - No vesting schedule found for the given address");
        panic_with_error!(env, ContractError::VestingNotFoundForAddress);
    })
}

// TODO: uncomment when needed
// pub fn get_allowances(env: &Env, owner_spender: &(Address, Address)) -> i128 {
//     env.storage().persistent().get(owner_spender).unwrap_or_else(|| {
//             log!(&env, "Vesting: Get allowance: Critical error - No allowance found for the given address pair");
//             panic_with_error!(env, ContractError::AllowanceNotFoundForGivenPair);
//         })
// }

// pub fn save_allowances(env: &Env, owner_spender: &(Address, Address), amount: i128) {
//     env.storage().persistent().set(owner_spender, &amount);
// }

pub fn save_minter(env: &Env, minter: &MinterInfo) {
    env.storage().persistent().set(&DataKey::Minter, minter);
}

pub fn get_minter(env: &Env) -> Option<MinterInfo> {
    env.storage().persistent().get(&DataKey::Minter)
}

pub fn save_token_info(env: &Env, token_info: &VestingTokenInfo) {
    env.storage()
        .persistent()
        .set(&DataKey::VestingTokenInfo, token_info);
}

pub fn get_token_info(env: &Env) -> VestingTokenInfo {
    env.storage()
        .persistent()
        .get(&DataKey::VestingTokenInfo)
        .unwrap_or_else(|| {
            log!(
                &env,
                "Vesting: Get token info: Critical error - No token info found"
            );
            panic_with_error!(env, ContractError::NoTokenInfoFound);
        })
}

pub fn save_max_vesting_complexity(env: &Env, max_vesting_complexity: &u32) {
    env.storage()
        .persistent()
        .set(&DataKey::MaxVestingComplexity, max_vesting_complexity);
}
