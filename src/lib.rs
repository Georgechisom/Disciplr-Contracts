#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, Address, BytesN, Env, Symbol,
};

#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VaultStatus {
    Active = 0,
    Completed = 1,
    Failed = 2,
    Cancelled = 3,
}

#[contracttype]
#[derive(Clone)]
pub struct ProductivityVault {
    pub creator: Address,
    pub amount: i128,
    pub start_timestamp: u64,
    pub end_timestamp: u64,
    pub milestone_hash: BytesN<32>,
    pub verifier: Option<Address>,
    pub success_destination: Address,
    pub failure_destination: Address,
    pub status: VaultStatus,
}

#[contract]
pub struct DisciplrVault;

#[contractimpl]
impl DisciplrVault {
    /// Create a new productivity vault. Caller must have approved USDC transfer to this contract.
    pub fn create_vault(
        env: Env,
        creator: Address,
        amount: i128,
        start_timestamp: u64,
        end_timestamp: u64,
        milestone_hash: BytesN<32>,
        verifier: Option<Address>,
        success_destination: Address,
        failure_destination: Address,
    ) -> u32 {
        creator.require_auth();
        // TODO: pull USDC from creator to this contract
        // For now, just store vault metadata (storage key pattern would be used in full impl)
        let vault = ProductivityVault {
            creator: creator.clone(),
            amount,
            start_timestamp,
            end_timestamp,
            milestone_hash,
            verifier,
            success_destination,
            failure_destination,
            status: VaultStatus::Active,
        };
        let vault_id = 0u32; // placeholder; real impl would allocate id and persist
        env.events().publish(
            (Symbol::new(&env, "vault_created"), vault_id),
            vault,
        );
        vault_id
    }

    /// Verifier (or authorized party) validates milestone completion.
    pub fn validate_milestone(env: Env, vault_id: u32) -> bool {
        // TODO: check vault exists, status is Active, caller is verifier, timestamp < end
        // TODO: transfer USDC to success_destination, set status Completed
        env.events().publish(
            (Symbol::new(&env, "milestone_validated"), vault_id),
            (),
        );
        true
    }

    /// Release funds to success destination (called after validation or by deadline logic).
    pub fn release_funds(_env: Env, _vault_id: u32) -> bool {
        // TODO: require status Active, transfer to success_destination, set Completed
        true
    }

    /// Redirect funds to failure destination (e.g. after deadline without validation).
    pub fn redirect_funds(_env: Env, _vault_id: u32) -> bool {
        // TODO: require status Active and past end_timestamp, transfer to failure_destination, set Failed
        true
    }

    /// Cancel vault and return funds to creator (if allowed by rules).
    /// Only Active vaults can be cancelled.
    pub fn cancel_vault(env: Env, vault_id: u32, creator: Address) -> bool {
        creator.require_auth();
        
        // Get vault state
        let vault_opt = Self::get_vault_state(env.clone(), vault_id);
        
        if let Some(vault) = vault_opt {
            // Verify caller is the creator
            if vault.creator != creator {
                panic!("Only vault creator can cancel");
            }
            
            // Only Active vaults can be cancelled
            if vault.status != VaultStatus::Active {
                panic!("Only Active vaults can be cancelled");
            }
            
            // TODO: return USDC to creator, set status to Cancelled
            env.events().publish(
                (Symbol::new(&env, "vault_cancelled"), vault_id),
                (),
            );
            true
        } else {
            panic!("Vault not found");
        }
    }

    /// Return current vault state for a given vault id.
    pub fn get_vault_state(env: Env, vault_id: u32) -> Option<ProductivityVault> {
        env.storage().instance().get(&vault_id)
    }
}

// Test helper function outside of contractimpl
#[cfg(test)]
fn set_vault_in_storage(env: &Env, contract_id: &Address, vault_id: u32, vault: ProductivityVault) {
    env.as_contract(contract_id, || {
        env.storage().instance().set(&vault_id, &vault);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    fn create_test_vault(env: &Env, status: VaultStatus) -> (u32, Address, ProductivityVault) {
        let creator = Address::generate(env);
        let verifier = Address::generate(env);
        let success_dest = Address::generate(env);
        let failure_dest = Address::generate(env);
        let milestone_hash = BytesN::from_array(env, &[0u8; 32]);
        
        let vault = ProductivityVault {
            creator: creator.clone(),
            amount: 1000,
            start_timestamp: 1000,
            end_timestamp: 2000,
            milestone_hash,
            verifier: Some(verifier),
            success_destination: success_dest,
            failure_destination: failure_dest,
            status,
        };
        
        let vault_id = 1u32;
        (vault_id, creator, vault)
    }

    #[test]
    #[should_panic(expected = "Only Active vaults can be cancelled")]
    fn test_cancel_vault_when_completed_fails() {
        let env = Env::default();
        let contract_id = env.register(DisciplrVault, ());
        let client = DisciplrVaultClient::new(&env, &contract_id);
        
        // Create a vault with Completed status
        let (vault_id, creator, vault) = create_test_vault(&env, VaultStatus::Completed);
        
        // Store the vault in contract storage
        set_vault_in_storage(&env, &contract_id, vault_id, vault);
        
        // Mock auth for creator
        env.mock_all_auths();
        
        // Attempt to cancel - should panic
        client.cancel_vault(&vault_id, &creator);
    }

    #[test]
    #[should_panic(expected = "Only Active vaults can be cancelled")]
    fn test_cancel_vault_when_failed_fails() {
        let env = Env::default();
        let contract_id = env.register(DisciplrVault, ());
        let client = DisciplrVaultClient::new(&env, &contract_id);
        
        // Create a vault with Failed status
        let (vault_id, creator, vault) = create_test_vault(&env, VaultStatus::Failed);
        
        // Store the vault in contract storage
        set_vault_in_storage(&env, &contract_id, vault_id, vault);
        
        // Mock auth for creator
        env.mock_all_auths();
        
        // Attempt to cancel - should panic
        client.cancel_vault(&vault_id, &creator);
    }

    #[test]
    fn test_cancel_vault_when_active_succeeds() {
        let env = Env::default();
        let contract_id = env.register(DisciplrVault, ());
        let client = DisciplrVaultClient::new(&env, &contract_id);
        
        // Create a vault with Active status
        let (vault_id, creator, vault) = create_test_vault(&env, VaultStatus::Active);
        
        // Store the vault in contract storage
        set_vault_in_storage(&env, &contract_id, vault_id, vault);
        
        // Mock auth for creator
        env.mock_all_auths();
        
        // Cancel should succeed
        let result = client.cancel_vault(&vault_id, &creator);
        assert!(result, "Expected cancel_vault to succeed for Active vault");
    }

    #[test]
    #[should_panic(expected = "Only Active vaults can be cancelled")]
    fn test_cancel_vault_when_cancelled_fails() {
        let env = Env::default();
        let contract_id = env.register(DisciplrVault, ());
        let client = DisciplrVaultClient::new(&env, &contract_id);
        
        // Create a vault with Cancelled status
        let (vault_id, _creator, vault) = create_test_vault(&env, VaultStatus::Cancelled);
        
        // Store the vault in contract storage
        set_vault_in_storage(&env, &contract_id, vault_id, vault.clone());
        
        // Mock auth for creator
        env.mock_all_auths();
        
        // Attempt to cancel - should panic
        client.cancel_vault(&vault_id, &vault.creator);
    }

    #[test]
    #[should_panic(expected = "Only vault creator can cancel")]
    fn test_cancel_vault_non_creator_fails() {
        let env = Env::default();
        let contract_id = env.register(DisciplrVault, ());
        let client = DisciplrVaultClient::new(&env, &contract_id);
        
        // Create a vault with Active status
        let (vault_id, _creator, vault) = create_test_vault(&env, VaultStatus::Active);
        
        // Store the vault in contract storage
        set_vault_in_storage(&env, &contract_id, vault_id, vault);
        
        // Try to cancel with a different address
        let non_creator = Address::generate(&env);
        env.mock_all_auths();
        
        // Attempt to cancel - should panic
        client.cancel_vault(&vault_id, &non_creator);
    }

    #[test]
    #[should_panic(expected = "Vault not found")]
    fn test_cancel_vault_nonexistent_fails() {
        let env = Env::default();
        let contract_id = env.register(DisciplrVault, ());
        let client = DisciplrVaultClient::new(&env, &contract_id);
        
        let creator = Address::generate(&env);
        let vault_id = 999u32;
        
        env.mock_all_auths();
        
        // Attempt to cancel non-existent vault - should panic
        client.cancel_vault(&vault_id, &creator);
    }
}
