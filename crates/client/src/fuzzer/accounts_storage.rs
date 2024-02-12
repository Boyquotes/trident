use std::collections::HashMap;

use solana_sdk::{pubkey::Pubkey, signature::Keypair};

use crate::{data_builder::FuzzClient, AccountId};

pub struct PdaStore {
    pub pubkey: Pubkey,
    pub seeds: Vec<Vec<u8>>,
}
impl PdaStore {
    pub fn pubkey(&self) -> Pubkey {
        self.pubkey
    }
}

pub struct TokenStore {
    pub pubkey: Pubkey,
}

pub struct MintStore {
    pub pubkey: Pubkey,
}

pub struct ProgramStore {
    pub pubkey: u8,
}

pub struct AccountsStorage<T> {
    accounts: HashMap<AccountId, T>,
    _max_accounts: u8,
}

impl<T> AccountsStorage<T> {
    pub fn new(max_accounts: u8) -> Self {
        let accounts: HashMap<AccountId, T> = HashMap::new();
        Self {
            accounts,
            _max_accounts: max_accounts,
        }
    }

    /// Gets a reference to the account with the given account ID
    pub fn get(&self, account_id: AccountId) -> Option<&T> {
        self.accounts.get(&account_id)
    }

    /// Returns a mutable reference to the underlying HashMap that stores accounts with IDs as keys
    pub fn storage(&mut self) -> &mut HashMap<AccountId, T> {
        &mut self.accounts
    }
}

impl<T> Default for AccountsStorage<T> {
    fn default() -> Self {
        Self::new(2)
    }
}
// TODO Add an easy way to limit the number of created accounts
impl AccountsStorage<Keypair> {
    pub fn get_or_create_account(
        &mut self,
        account_id: AccountId,
        client: &mut impl FuzzClient,
        lamports: u64,
    ) -> Keypair {
        let key = self
            .accounts
            .entry(account_id)
            .or_insert_with(|| client.set_account(lamports));
        key.insecure_clone()
    }
    pub fn get_or_create_data_account(
        &mut self,
        account_id: AccountId,
        client: &mut impl FuzzClient,
        lamports: u64,
        space: usize,
    ) -> Keypair {
        if self.accounts.get(&account_id).is_none() {
            let key = client.set_data_account(lamports, space);
            self.accounts.insert(account_id, key);
        }
        let key = self.accounts.get(&account_id).unwrap();
        key.insecure_clone()
    }
}

impl AccountsStorage<TokenStore> {
    #[allow(clippy::too_many_arguments)]
    pub fn get_or_create_account(
        &mut self,
        account_id: AccountId,
        client: &mut impl FuzzClient,
        mint: Pubkey,
        owner: Pubkey,
        amount: u64,
        delegate: Option<Pubkey>,
        is_native: Option<u64>,
        delegated_amount: u64,
        close_authority: Option<Pubkey>,
    ) -> Option<Pubkey> {
        if self.accounts.get(&account_id).is_none() {
            let key = client.set_token_account(
                mint,
                owner,
                amount,
                delegate,
                is_native,
                delegated_amount,
                close_authority,
            );
            self.accounts.insert(account_id, TokenStore { pubkey: key });
        }
        Some(self.accounts.get(&account_id).unwrap().pubkey)
    }
}

impl AccountsStorage<MintStore> {
    pub fn get_or_create_account(
        &mut self,
        account_id: AccountId,
        client: &mut impl FuzzClient,
        decimals: u8,
        owner: &Pubkey,
        freeze_authority: Option<Pubkey>,
    ) -> Option<Pubkey> {
        if self.accounts.get(&account_id).is_none() {
            let key = client.set_mint_account(decimals, owner, freeze_authority);
            self.accounts.insert(account_id, MintStore { pubkey: key });
        }
        Some(self.accounts.get(&account_id).unwrap().pubkey)
    }
}

impl AccountsStorage<PdaStore> {
    pub fn get_or_create_account(
        &mut self,
        account_id: AccountId,
        client: &mut impl FuzzClient,
        seeds: &[&[u8]],
        program_id: &Pubkey,
    ) -> Option<&PdaStore> {
        if self.accounts.get(&account_id).is_none() {
            let pda_store = client.set_pda_account(seeds, program_id).unwrap();
            self.accounts.insert(account_id, pda_store);
        }
        self.accounts.get(&account_id)
    }
    pub fn get_or_create_data_account(
        &mut self,
        account_id: AccountId,
        client: &mut impl FuzzClient,
        seeds: &[&[u8]],
        program_id: &Pubkey,
        space: usize,
    ) -> Option<&PdaStore> {
        if self.accounts.get(&account_id).is_none() {
            let pda_store = client
                .set_pda_data_account(seeds, program_id, space)
                .unwrap();
            self.accounts.insert(account_id, pda_store);
        }
        self.accounts.get(&account_id)
    }
}
