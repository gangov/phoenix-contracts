use super::setup::{
    deploy_factory_contract, install_lp_contract, install_multihop_wasm, install_stake_wasm,
    install_token_wasm, lp_contract,
};
use crate::contract::{Factory, FactoryClient};
use phoenix::utils::{LiquidityPoolInitInfo, StakeInitInfo, TokenInitInfo};

use soroban_sdk::{
    testutils::{arbitrary::std, Address as _},
    vec, Address, Env, String,
};

#[test]
fn factory_successfully_inits_itself() {
    let env = Env::default();
    let admin = Address::generate(&env);

    let factory = deploy_factory_contract(&env, Some(admin.clone()));

    assert_eq!(factory.get_admin(), admin);
}

#[test]
fn factory_successfully_inits_multihop() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);

    let factory = deploy_factory_contract(&env, Some(admin.clone()));

    let multihop_address = factory.get_config().multihop_address;

    assert!(multihop_address.to_string().len() != 0);
}

#[test]
fn factory_successfully_inits_lp() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let mut token1_admin = Address::generate(&env);
    let mut token2_admin = Address::generate(&env);
    let user = Address::generate(&env);

    let mut token1 = Address::generate(&env);
    let mut token2 = Address::generate(&env);

    env.mock_all_auths();
    env.budget().reset_unlimited();

    if token2 < token1 {
        std::mem::swap(&mut token1, &mut token2);
        std::mem::swap(&mut token1_admin, &mut token2_admin);
    }

    let factory = deploy_factory_contract(&env, Some(admin.clone()));
    assert_eq!(factory.get_admin(), admin);

    let token_init_info = TokenInitInfo {
        token_a: token1,
        token_b: token2,
    };
    let stake_init_info = StakeInitInfo {
        min_bond: 10i128,
        min_reward: 5i128,
        manager: Address::generate(&env),
        max_complexity: 10u32,
    };

    let lp_init_info = LiquidityPoolInitInfo {
        admin: admin.clone(),
        fee_recipient: user.clone(),
        max_allowed_slippage_bps: 5_000,
        max_allowed_spread_bps: 500,
        swap_fee_bps: 0,
        max_referral_bps: 5_000,
        token_init_info: token_init_info.clone(),
        stake_init_info,
    };

    factory.create_liquidity_pool(
        &admin,
        &lp_init_info,
        &String::from_str(&env, "Pool"),
        &String::from_str(&env, "PHO/BTC"),
    );
    let lp_contract_addr = factory.query_pools().get(0).unwrap();

    let first_lp_contract = lp_contract::Client::new(&env, &lp_contract_addr);
    let share_token_address = first_lp_contract.query_share_token_address();
    let stake_token_address = first_lp_contract.query_stake_contract_address();

    assert_eq!(
        first_lp_contract.query_config(),
        lp_contract::Config {
            fee_recipient: user,
            max_allowed_slippage_bps: 5_000,
            max_allowed_spread_bps: 500,
            max_referral_bps: 5_000,
            pool_type: lp_contract::PairType::Xyk,
            share_token: share_token_address,
            stake_contract: stake_token_address,
            token_a: token_init_info.token_a,
            token_b: token_init_info.token_b,
            total_fee_bps: 0,
        }
    );
}

#[test]
#[should_panic(
    expected = "Factory: Create Liquidity Pool: You are not authorized to create liquidity pool!"
)]
fn factory_fails_to_init_lp_when_authorized_address_not_present() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let mut token1_admin = Address::generate(&env);
    let mut token2_admin = Address::generate(&env);
    let user = Address::generate(&env);

    let mut token1 = Address::generate(&env);
    let mut token2 = Address::generate(&env);

    env.mock_all_auths();
    env.budget().reset_unlimited();

    if token2 < token1 {
        std::mem::swap(&mut token1, &mut token2);
        std::mem::swap(&mut token1_admin, &mut token2_admin);
    }

    let factory = deploy_factory_contract(&env, Some(admin.clone()));
    assert_eq!(factory.get_admin(), admin);

    let token_init_info = TokenInitInfo {
        token_a: token1,
        token_b: token2,
    };
    let stake_init_info = StakeInitInfo {
        min_bond: 10i128,
        min_reward: 5i128,
        manager: Address::generate(&env),
        max_complexity: 10u32,
    };

    let lp_init_info = LiquidityPoolInitInfo {
        admin,
        fee_recipient: user.clone(),
        max_allowed_slippage_bps: 5_000,
        max_allowed_spread_bps: 500,
        swap_fee_bps: 0,
        max_referral_bps: 5_000,
        token_init_info: token_init_info.clone(),
        stake_init_info,
    };

    let unauthorized_addr = Address::generate(&env);

    factory.create_liquidity_pool(
        &unauthorized_addr,
        &lp_init_info,
        &String::from_str(&env, "Pool"),
        &String::from_str(&env, "PHO/BTC"),
    );
}

#[should_panic(
    expected = "Factory: Initialize: there must be at least one whitelisted account able to create liquidity pools."
)]
#[test]
fn factory_fails_to_init_lp_when_no_whitelisted_accounts() {
    let env = Env::default();
    let admin = Address::generate(&env);

    env.mock_all_auths();
    env.budget().reset_unlimited();

    let factory = FactoryClient::new(&env, &env.register_contract(None, Factory {}));
    let multihop_wasm_hash = install_multihop_wasm(&env);
    let whitelisted_accounts = vec![&env];

    let lp_wasm_hash = install_lp_contract(&env);
    let stake_wasm_hash = install_stake_wasm(&env);
    let token_wasm_hash = install_token_wasm(&env);

    factory.initialize(
        &admin,
        &multihop_wasm_hash,
        &lp_wasm_hash,
        &stake_wasm_hash,
        &token_wasm_hash,
        &whitelisted_accounts,
        &10u32,
    );
}

#[test]
fn successfully_updates_new_list_of_whitelisted_accounts() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let first_wl_addr = Address::generate(&env);
    let second_wl_addr = Address::generate(&env);

    let factory = deploy_factory_contract(&env, admin.clone());

    let to_add = vec![&env, first_wl_addr.clone(), second_wl_addr.clone()];
    factory.update_whitelisted_accounts(&admin.clone(), &to_add, &vec![&env]);
    // query for first whitelisted address
    let config = factory.get_config();

    assert!(config.whitelisted_accounts.contains(first_wl_addr.clone()));

    let to_remove = vec![&env, admin.clone()];

    factory.update_whitelisted_accounts(&admin, &vec![&env], &to_remove);

    let config = factory.get_config();

    assert!(config.whitelisted_accounts.contains(first_wl_addr));
    assert!(config.whitelisted_accounts.contains(second_wl_addr));
    assert!(config.whitelisted_accounts.len() == 2);
}

#[test]
fn doesn_not_change_whitelisted_accounts_when_removing_non_existent() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);

    let factory = deploy_factory_contract(&env, admin.clone());

    let to_remove = vec![&env, Address::generate(&env)];

    factory.update_whitelisted_accounts(&admin.clone(), &vec![&env], &to_remove);

    let config = factory.get_config();

    assert!(config.whitelisted_accounts.contains(admin));
    assert!(config.whitelisted_accounts.len() == 1);
}

#[should_panic(expected = "Factory: Update whitelisted accounts: You are not authorized!")]
#[test]
fn fails_to_update_whitelisted_accounts_when_not_authorized() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let first_wl_addr = Address::generate(&env);
    let second_wl_addr = Address::generate(&env);

    let factory = deploy_factory_contract(&env, admin.clone());

    let to_add = vec![&env, first_wl_addr.clone(), second_wl_addr.clone()];
    factory.update_whitelisted_accounts(&admin.clone(), &to_add, &vec![&env]);

    let to_remove = vec![&env, admin.clone()];

    factory.update_whitelisted_accounts(&Address::generate(&env), &vec![&env], &to_remove);
}

#[test]
fn test_add_vec_with_duplicates_should_be_handled_correctly() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let first_wl_addr = Address::generate(&env);
    let dupe_of_first_wl_addr = first_wl_addr.clone();
    let second_wl_addr = Address::generate(&env);
    let dupe_second_wl_addr = second_wl_addr.clone();

    let factory = deploy_factory_contract(&env, admin.clone());

    let to_add = vec![
        &env,
        first_wl_addr.clone(),
        dupe_of_first_wl_addr.clone(),
        second_wl_addr.clone(),
        dupe_second_wl_addr.clone(),
    ];

    factory.update_whitelisted_accounts(&admin.clone(), &to_add, &vec![&env]);
    let config = factory.get_config();

    assert!(config.whitelisted_accounts.contains(first_wl_addr.clone()));
    assert!(config.whitelisted_accounts.len() == 3);

    let to_remove = vec![&env, admin.clone()];

    factory.update_whitelisted_accounts(&admin, &vec![&env], &to_remove);

    let config = factory.get_config();

    assert!(config.whitelisted_accounts.contains(first_wl_addr));
    assert!(config.whitelisted_accounts.contains(second_wl_addr));
    assert!(config.whitelisted_accounts.len() == 2);
}
