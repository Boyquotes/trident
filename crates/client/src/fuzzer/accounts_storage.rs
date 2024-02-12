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
    pub max_accounts: u8,
}

impl<T> AccountsStorage<T> {
    pub fn new(max_accounts: u8) -> Self {
        let accounts: HashMap<AccountId, T> = HashMap::new();
        Self {
            accounts,
            max_accounts,
        }
    }

    pub fn get(&self, account_id: AccountId) -> Option<&T> {
        self.accounts.get(&account_id)
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
        let key = self
            .accounts
            .entry(account_id)
            .or_insert_with(|| client.set_data_account(lamports, space));
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
        let key = self.accounts.entry(account_id).or_insert_with(|| {
            let key = client.set_token_account(
                mint,
                owner,
                amount,
                delegate,
                is_native,
                delegated_amount,
                close_authority,
            );
            TokenStore { pubkey: key }
        });
        Some(key.pubkey)
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
        let key = self.accounts.entry(account_id).or_insert_with(|| {
            let key = client.set_mint_account(decimals, owner, freeze_authority);
            MintStore { pubkey: key }
        });
        Some(key.pubkey)
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

        // self.accounts.get(&account_id)
        // let key = self
        //     .accounts
        //     .entry(account_id)
        //     .or_insert(client.set_pda_account(seeds, program_id).unwrap());
        // Some(key)
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

        // if self.accounts.get(&account_id).is_none() {
        //     eprintln!("Is None");
        // } else {
        //     let x = self.accounts.get(&account_id).unwrap();
        //     let acc = client.get_account(&x.pubkey).unwrap();
        //     eprintln!("BEFORE {:#?}", acc);
        // }
        // let key = self.accounts.entry(account_id).or_insert(
        //     client
        //         .set_pda_data_account(seeds, program_id, space)
        //         .unwrap(),
        // );

        // let acc = client.get_account(&key.pubkey).unwrap();
        // eprintln!("AFTER {:#?}", acc);
        // Some(key)
    }
}
