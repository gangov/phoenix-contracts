use soroban_sdk::{contract, contractimpl, contractmeta, log, panic_with_error, Address, Env, Vec};

use curve::Curve;

use crate::storage::{
    get_admin, get_token_info, save_max_vesting_complexity, save_token_info, DistributionInfo,
};
use crate::utils::{create_vesting_accounts, verify_vesting_and_update_balances};
use crate::{
    error::ContractError,
    storage::{
        get_minter, get_vesting, save_admin, save_minter, MinterInfo, VestingBalance,
        VestingTokenInfo,
    },
    token_contract,
};

// Metadata that is added on to the WASM custom section
contractmeta!(
    key = "Description",
    val = "Phoenix Protocol Token Vesting Contract"
);
#[contract]
pub struct Vesting;

pub trait VestingTrait {
    fn initialize(
        env: Env,
        admin: Address,
        vesting_token: VestingTokenInfo,
        vesting_balances: Vec<VestingBalance>,
        minter_info: Option<MinterInfo>,
        max_vesting_complexity: u32,
    );

    fn transfer_token(env: Env, sender: Address, recipient: Address, amount: i128);

    fn claim(env: Env, sender: Address);

    fn burn(env: Env, sender: Address, amount: u128);

    fn mint(env: Env, sender: Address, amount: i128);

    // TODO: we will need these in the future, not needed for the most basic implementation right now
    // TODO: replace the tuple `owner_spender: (Address, Address)` with how it is in `send_to_contract_from`
    // fn increase_allowance(env: Env, owner_spender: (Address, Address), amount: i128);

    // fn decrease_allowance(env: Env, owner_spender: (Address, Address), amount: i128);

    // fn transfer_from(
    //     env: Env,
    //     owner_spender: (Address, Address),
    //     to: Address,
    //     amount: i128,
    // ) -> Result<(), ContractError>;

    // fn burn_from(
    //     env: Env,
    //     sender: Address,
    //     owner: Address,
    //     amount: i128,
    // ) -> Result<(), ContractError>;

    // fn send_to_contract_from(
    //     env: Env,
    //     sender: Address,
    //     owner: Address,
    //     contract: Address,
    //     amount: i128,
    // ) -> Result<(), ContractError>;

    fn update_minter(env: Env, sender: Address, new_minter: Address);

    fn update_minter_capacity(env: Env, sender: Address, new_capacity: u128);

    fn query_balance(env: Env, address: Address) -> i128;

    fn query_distribution_info(env: Env, address: Address) -> DistributionInfo;

    fn query_token_info(env: Env) -> VestingTokenInfo;

    fn query_minter(env: Env) -> MinterInfo;

    fn query_vesting_contract_balance(env: Env) -> i128;

    fn query_available_to_claim(env: Env, address: Address) -> i128;
}

#[contractimpl]
impl VestingTrait for Vesting {
    fn initialize(
        env: Env,
        admin: Address,
        vesting_token: VestingTokenInfo,
        vesting_balances: Vec<VestingBalance>,
        minter_info: Option<MinterInfo>,
        max_vesting_complexity: u32,
    ) {
        admin.require_auth();

        save_admin(&env, &admin);

        if vesting_balances.is_empty() {
            log!(
                &env,
                "Vesting: Initialize: At least one vesting schedule must be provided."
            );
            panic_with_error!(env, ContractError::MissingBalance);
        }

        let total_vested_amount =
            create_vesting_accounts(&env, max_vesting_complexity, vesting_balances);

        // check if the admin has enough tokens to start the vesting contract
        let token_client = token_contract::Client::new(&env, &vesting_token.address);

        if token_client.balance(&admin) < total_vested_amount as i128 {
            log!(
                &env,
                "Vesting: Initialize: Admin does not have enough tokens to start the vesting contract"
            );
            panic_with_error!(env, ContractError::NoEnoughtTokensToStart);
        }

        token_client.transfer(
            &admin,
            &env.current_contract_address(),
            &(total_vested_amount as i128),
        );

        if let Some(minter) = minter_info {
            let input_curve = Curve::Constant(minter.mint_capacity);

            let capacity = input_curve.value(env.ledger().timestamp());

            if total_vested_amount > capacity {
                log!(
                    &env,
                    "Vesting: Initialize: total vested amount over the capacity"
                );
                panic_with_error!(env, ContractError::TotalVestedOverCapacity);
            }
            save_minter(&env, &minter);
        }

        let token_info = VestingTokenInfo {
            name: vesting_token.name,
            symbol: vesting_token.symbol,
            decimals: vesting_token.decimals,
            address: vesting_token.address,
        };

        save_token_info(&env, &token_info);
        save_max_vesting_complexity(&env, &max_vesting_complexity);

        env.events()
            .publish(("Initialize", "Vesting contract with admin: "), admin);
    }

    fn transfer_token(env: Env, sender: Address, recipient: Address, amount: i128) {
        sender.require_auth();

        if amount <= 0 {
            log!(&env, "Vesting: Transfer token: Invalid transfer amount");
            panic_with_error!(env, ContractError::InvalidTransferAmount);
        }

        let token_client = token_contract::Client::new(&env, &get_token_info(&env).address);

        verify_vesting_and_update_balances(&env, &sender, amount as u128);
        token_client.transfer(&env.current_contract_address(), &recipient, &amount);

        env.events().publish(
            (
                "Transfer token",
                "Transfering tokens between accounts: from: {}, to:{}, amount: {}",
            ),
            (sender, recipient, amount),
        );
    }

    fn claim(env: Env, sender: Address) {
        sender.require_auth();

        let available_to_claim = Self::query_available_to_claim(env.clone(), sender.clone());

        if available_to_claim <= 0 {
            log!(&env, "Vesting: Claim: No tokens available to claim");
            panic_with_error!(env, ContractError::NeverFullyVested);
        }

        let token_client = token_contract::Client::new(&env, &get_token_info(&env).address);

        verify_vesting_and_update_balances(&env, &sender, available_to_claim as u128);

        token_client.transfer(
            &env.current_contract_address(),
            &sender,
            &(available_to_claim),
        );

        env.events()
            .publish(("Claim", "Claimed tokens: "), available_to_claim);
    }

    fn burn(env: Env, sender: Address, amount: u128) {
        sender.require_auth();

        if amount == 0 {
            log!(&env, "Vesting: Burn: Invalid burn amount");
            panic_with_error!(env, ContractError::InvalidBurnAmount);
        }

        let token_client = token_contract::Client::new(&env, &get_token_info(&env).address);

        token_client.burn(&sender, &(amount as i128));

        env.events().publish(("Burn", "Burned from: "), sender);
        env.events().publish(("Burn", "Burned tokens: "), amount);
    }

    fn mint(env: Env, sender: Address, amount: i128) {
        sender.require_auth();

        if amount <= 0 {
            log!(&env, "Vesting: Mint: Invalid mint amount");
            panic_with_error!(env, ContractError::InvalidMintAmount);
        }

        // check if minter is set
        let minter = if let Some(minter) = get_minter(&env) {
            minter
        } else {
            log!(&env, "Vesting: Mint: Minter not found");
            panic_with_error!(env, ContractError::MinterNotFound);
        };

        // check if sender is minter
        if sender != minter.address {
            log!(&env, "Vesting: Mint: Not authorized to mint");
            panic_with_error!(env, ContractError::NotAuthorized);
        }

        // check if minter has enough to mint
        let minter_remainder = get_minter(&env)
            .map_or(0, |m| m.mint_capacity)
            .checked_sub(amount as u128)
            .unwrap_or_else(|| {
                log!(
                    &env,
                    "Vesting: Mint: Minter does not have enough capacity to mint"
                );
                panic_with_error!(env, ContractError::NotEnoughCapacity);
            });

        // mint to recipient
        let token_client = token_contract::Client::new(&env, &get_token_info(&env).address);
        token_client.mint(&env.current_contract_address(), &amount);

        // we update the minter
        save_minter(
            &env,
            &MinterInfo {
                address: minter.address,
                mint_capacity: minter_remainder,
            },
        );

        env.events().publish(("Mint", "sender: "), sender);
        env.events().publish(("Mint", "Minted tokens: "), amount);
    }

    // TODO: we will need these in the future, not needed for the most basic implementation right now
    // fn increase_allowance(env: Env, owner_spender: (Address, Address), amount: i128) {
    //     owner_spender.0.require_auth();

    //     if amount <= 0 {
    //         log!(&env, "Vesting: Increase allowance: Invalid amount");
    //         panic_with_error!(env, ContractError::InvalidAllowanceAmount);
    //     }

    //     let allowance = get_allowances(&env, &owner_spender)
    //         .checked_add(amount)
    //         .unwrap_or_else(|| {
    //             log!(
    //                 &env,
    //                 "Vesting: Increase allowance: Critical error - allowance cannot be negative"
    //             );
    //             panic_with_error!(env, ContractError::Std);
    //         });

    //     save_allowances(&env, &owner_spender, allowance);

    //     env.events().publish(
    //         (
    //             "Increase allowance",
    //             "Increased allowance between accounts: from: {}, to: {}, increase: {}",
    //         ),
    //         (owner_spender.0, owner_spender.1, amount),
    //     );
    // }

    // fn decrease_allowance(env: Env, owner_spender: (Address, Address), amount: i128) {
    //     owner_spender.0.require_auth();

    //     if amount <= 0 {
    //         log!(&env, "Vesting: Decrease allowance: Invalid amount");
    //         panic_with_error!(env, ContractError::InvalidAllowanceAmount);
    //     }

    //     let allowance = get_allowances(&env, &owner_spender)
    //         .checked_sub(amount)
    //         .unwrap_or_else(|| {
    //             log!(
    //                 &env,
    //                 "Vesting: Decrease allowance: Critical error - allowance cannot be negative"
    //             );
    //             panic_with_error!(env, ContractError::Std);
    //         });

    //     save_allowances(&env, &owner_spender, allowance);

    //     env.events().publish(
    //         (
    //             "Decrease allowance",
    //             "Decreased allowance between accounts: from: {}, to: {}, decrease: {}",
    //         ),
    //         (owner_spender.0, owner_spender.1, amount),
    //     );
    // }

    // fn transfer_from(
    //     env: Env,
    //     owner_spender: (Address, Address),
    //     to: Address,
    //     amount: i128,
    // ) -> Result<(), ContractError> {
    //     let owner = owner_spender.0.clone();
    //     let spender = owner_spender.1.clone();
    //     spender.require_auth();

    //     if amount <= 0 {
    //         log!(&env, "Vesting: Transfer from: Invalid transfer amount");
    //         panic_with_error!(env, ContractError::InvalidTransferAmount);
    //     }

    //     // todo deduct_allowances
    //     let allowance = get_allowances(&env, &owner_spender);
    //     if allowance < amount {
    //         log!(&env, "Vesting: Transfer from: Not enough allowance");
    //         panic_with_error!(env, ContractError::NotEnoughBalance);
    //     }
    //     let new_allowance = allowance.checked_sub(amount).unwrap_or_else(|| {
    //         log!(
    //             &env,
    //             "Vesting: Transfer from: Critical error - allowance cannot be negative"
    //         );
    //         panic_with_error!(env, ContractError::Std);
    //     });

    //     verify_vesting_and_transfer_tokens(&env, &owner, &to, amount)?;

    //     save_allowances(&env, &owner_spender, new_allowance);

    //     env.events().publish(
    //         (
    //             "Transfer from",
    //             "Transfering tokens between accounts: from: {}, to: {}, amount: {}",
    //         ),
    //         (owner, to, amount),
    //     );

    //     Ok(())
    // }

    // fn burn_from(
    //     env: Env,
    //     sender: Address,
    //     owner: Address,
    //     amount: i128,
    // ) -> Result<(), ContractError> {
    //     sender.require_auth();

    //     if amount <= 0 {
    //         log!(&env, "Vesting: Burn from: Invalid burn amount");
    //         panic_with_error!(env, ContractError::InvalidBurnAmount);
    //     }

    //     let allowance = get_allowances(&env, &(owner.clone(), sender.clone()));
    //     if allowance < amount {
    //         log!(&env, "Vesting: Burn from: Not enough allowance");
    //         panic_with_error!(env, ContractError::NotEnoughBalance);
    //     }

    //     let new_allowance = allowance.checked_sub(amount).unwrap_or_else(|| {
    //         log!(
    //             &env,
    //             "Vesting: Burn from: Critical error - allowance cannot be negative"
    //         );
    //         panic_with_error!(env, ContractError::Std);
    //     });

    //     let total_supply = get_vesting_total_supply(&env)
    //         .checked_sub(amount)
    //         .unwrap_or_else(|| {
    //             log!(
    //                 &env,
    //                 "Vesting: Burn from: Critical error - total supply cannot be negative"
    //             );
    //             panic_with_error!(env, ContractError::Std);
    //         });

    //     update_vesting_total_supply(&env, total_supply);

    //     let token_client = token_contract::Client::new(&env, &get_config(&env).token_info.address);
    //     token_client.burn(&owner, &amount);

    //     save_allowances(&env, &(owner, sender), new_allowance);

    //     env.events()
    //         .publish(("Burn from", "Burned tokens: "), amount);

    //     Ok(())
    // }

    // fn send_to_contract_from(
    //     env: Env,
    //     sender: Address,
    //     owner: Address,
    //     contract: Address,
    //     amount: i128,
    // ) -> Result<(), ContractError> {
    //     sender.require_auth();
    //     if amount <= 0 {
    //         log!(&env, "Vesting: Send to contract from: Invalid amount");
    //         panic_with_error!(env, ContractError::InvalidTransferAmount);
    //     }
    //     //used to verify that the sender is authorized by the owner
    //     let _ = get_allowances(&env, &(owner.clone(), sender.clone()));

    //     let token_client = token_contract::Client::new(&env, &get_config(&env).token_info.address);
    //     token_client.transfer(&owner, &contract, &amount);

    //     env.events().publish(
    //         (
    //             "Send to contract from",
    //             "Sent tokens to contract from account: from: {}, to: {}, amount: {}",
    //         ),
    //         (owner, contract, amount),
    //     );

    //     Ok(())
    // }

    fn update_minter(env: Env, sender: Address, new_minter: Address) {
        let current_minter = get_minter(&env);

        let is_authorized = if let Some(current_minter) = current_minter.clone() {
            sender == current_minter.address
        } else {
            sender == get_admin(&env)
        };

        if !is_authorized {
            log!(
                env,
                "Vesting: Update minter: Not authorized to update minter"
            );
            panic_with_error!(env, ContractError::NotAuthorized);
        }

        let mint_capacity = current_minter.map_or(0, |m| m.mint_capacity);
        save_minter(
            &env,
            &MinterInfo {
                address: new_minter.clone(),
                mint_capacity,
            },
        );

        env.events()
            .publish(("Update minter", "Updated minter to: "), new_minter);
    }

    fn update_minter_capacity(env: Env, sender: Address, new_capacity: u128) {
        if sender != get_admin(&env) {
            log!(
                &env,
                "Vesting: Update minter capacity: Only contract's admin can update the minter's capacity"
            );
            panic_with_error!(env, ContractError::NotAuthorized);
        }

        if let Some(minter) = get_minter(&env) {
            save_minter(
                &env,
                &MinterInfo {
                    address: minter.address,
                    mint_capacity: new_capacity,
                },
            );
        } else {
            log!(&env, "Vesting: Update Minter Capacity: Minter not found");
            panic_with_error!(env, ContractError::MinterNotFound);
        };

        env.events().publish(
            ("Update minter capacity", "Updated minter capacity to: "),
            new_capacity,
        );
    }

    fn query_balance(env: Env, address: Address) -> i128 {
        token_contract::Client::new(&env, &get_token_info(&env).address).balance(&address)
    }

    fn query_distribution_info(env: Env, address: Address) -> DistributionInfo {
        get_vesting(&env, &address).distribution_info
    }

    fn query_token_info(env: Env) -> VestingTokenInfo {
        get_token_info(&env)
    }

    fn query_minter(env: Env) -> MinterInfo {
        if let Some(minter) = get_minter(&env) {
            minter
        } else {
            log!(&env, "Vesting: Query Minter: Minter not found");
            panic_with_error!(env, ContractError::MinterNotFound);
        }
    }

    fn query_vesting_contract_balance(env: Env) -> i128 {
        let token_address = get_token_info(&env).address;
        token_contract::Client::new(&env, &token_address).balance(&env.current_contract_address())
    }

    fn query_available_to_claim(env: Env, address: Address) -> i128 {
        let vesting_info = get_vesting(&env, &address);
        let vested = vesting_info
            .distribution_info
            .get_curve()
            .value(env.ledger().timestamp());

        let sender_balance = vesting_info.balance;
        let sender_liquid = sender_balance
            .checked_sub(vested)
            .unwrap_or_else(|| panic_with_error!(env, ContractError::NotEnoughBalance));

        sender_liquid as i128
    }
}
