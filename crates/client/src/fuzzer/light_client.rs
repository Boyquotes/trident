use std::cell::RefCell;
use std::collections::HashMap;

use solana_program::{instruction::AccountMeta, program_pack::Pack, rent::Rent};
use solana_sdk::account::AccountSharedData;
use solana_sdk::clock::Epoch;
use solana_sdk::{
    account::Account, account_info::AccountInfo, instruction::Instruction, program_option::COption,
    pubkey::Pubkey, signature::Keypair, signer::Signer,
};
use spl_token::state::Mint;

use crate::accounts_storage::PdaStore;
use crate::data_builder::FuzzClient;
use crate::error::*;
use crate::program_stubs::test_syscall_stubs;

pub type ProgramEntry = for<'info> fn(
    program_id: &Pubkey,
    accounts: &'info [AccountInfo<'info>],
    instruction_data: &[u8],
) -> anchor_lang::solana_program::entrypoint::ProgramResult;

#[derive(Clone, Debug)]

pub struct TridentAccount {
    pub lamports: u64,
    pub data: Vec<u8>,
    pub owner: Pubkey,
    pub executable: bool,
    pub rent_epoch: Epoch,
}

#[allow(clippy::too_many_arguments)]
impl TridentAccount {
    pub fn new(lamports: u64, space: usize, owner: &Pubkey) -> Self {
        Self {
            lamports,
            data: vec![0u8; space],
            owner: *owner,
            executable: false,
            rent_epoch: Epoch::default(),
        }
    }
    pub fn set_data_from_slice(&mut self, new_data: &[u8]) {
        self.data.copy_from_slice(new_data);
    }
    pub fn realloc(&mut self, newsize: usize) {
        self.data.resize(newsize, 0);
    }
}

impl From<AccountSharedData> for TridentAccount {
    fn from(value: AccountSharedData) -> Self {
        let account = Account::from(value);
        TridentAccount {
            lamports: account.lamports,
            data: account.data,
            owner: account.owner,
            executable: account.executable,
            rent_epoch: account.rent_epoch,
        }
    }
}

impl std::default::Default for TridentAccount {
    fn default() -> Self {
        TridentAccount::new(0, 0, &solana_sdk::system_program::ID)
    }
}

pub struct LighClient {
    pub entry: ProgramEntry,
    pub account_storage: RefCell<HashMap<Pubkey, TridentAccount>>,
}

impl LighClient {
    pub fn new(entry: ProgramEntry, program_id: Pubkey) -> Result<Self, FuzzClientError> {
        let mut new_client = Self {
            entry,
            account_storage: RefCell::new(HashMap::default()),
        };
        new_client.add_program(solana_sdk::system_program::ID);
        new_client.add_program(anchor_spl::token::ID);

        test_syscall_stubs(program_id);

        Ok(new_client)
    }
    pub fn clean_ctx(&mut self) -> Result<(), FuzzClientError> {
        self.account_storage = RefCell::new(HashMap::default());

        self.add_program(solana_sdk::system_program::ID);
        self.add_program(anchor_spl::token::ID);

        Ok(())
    }
    pub fn get_temporary_accounts(
        &self,
        metas: &[AccountMeta],
    ) -> Vec<(AccountMeta, TridentAccount)> {
        let result: Vec<_> = metas
            .iter()
            .map(|m| {
                (
                    m.clone(),
                    self.account_storage
                        .borrow()
                        .get(&m.pubkey)
                        .unwrap()
                        .clone(),
                )
            })
            .collect();
        result
    }
    pub fn add_program(&mut self, program_id: Pubkey) {
        let program = TridentAccount {
            executable: true,
            ..Default::default()
        };
        self.account_storage
            .borrow_mut()
            .insert(program_id, program);
    }
}

impl FuzzClient for LighClient {
    fn get_rent(&mut self) -> Result<Rent, FuzzClientError> {
        Ok(solana_sdk::rent::Rent::default())
    }
    fn set_account_custom(&mut self, address: &Pubkey, account: &AccountSharedData) {
        self.account_storage
            .borrow_mut()
            .insert(*address, account.to_owned().into());
    }

    fn set_account(&mut self, lamports: u64) -> solana_sdk::signature::Keypair {
        let new_account = Keypair::new();

        let new_account_info = TridentAccount::new(lamports, 0, &solana_sdk::system_program::ID);

        self.account_storage
            .borrow_mut()
            .insert(new_account.pubkey(), new_account_info);
        new_account
    }
    fn set_data_account(&mut self, lamports: u64, space: usize) -> Keypair {
        let new_account = Keypair::new();

        let new_account_info =
            TridentAccount::new(lamports, space, &solana_sdk::system_program::ID);

        self.account_storage
            .borrow_mut()
            .insert(new_account.pubkey(), new_account_info);
        new_account
    }
    fn set_pda_account(
        &mut self,
        seeds: &[&[u8]],
        program_id: &Pubkey,
    ) -> std::option::Option<PdaStore> {
        if let Some((key, _)) = Pubkey::try_find_program_address(seeds, program_id) {
            let empty_account = TridentAccount::default();
            self.account_storage.borrow_mut().insert(key, empty_account);
            let seeds_vec: Vec<_> = seeds.iter().map(|&s| s.to_vec()).collect();
            Some(PdaStore {
                pubkey: key,
                seeds: seeds_vec,
            })
        } else {
            None
        }
    }
    fn set_pda_data_account(
        &mut self,
        seeds: &[&[u8]],
        program_id: &Pubkey,
        space: usize,
    ) -> Option<PdaStore> {
        if let Some((key, _)) = Pubkey::try_find_program_address(seeds, program_id) {
            let allocated_account = TridentAccount::new(0, space, &solana_sdk::system_program::ID);
            self.account_storage
                .borrow_mut()
                .insert(key, allocated_account);
            let seeds_vec: Vec<_> = seeds.iter().map(|&s| s.to_vec()).collect();
            Some(PdaStore {
                pubkey: key,
                seeds: seeds_vec,
            })
        } else {
            None
        }
    }

    fn set_token_account(
        &mut self,
        mint: anchor_client::anchor_lang::prelude::Pubkey,
        owner: anchor_client::anchor_lang::prelude::Pubkey,
        amount: u64,
        delegate: Option<anchor_client::anchor_lang::prelude::Pubkey>,
        is_native: Option<u64>,
        delegated_amount: u64,
        close_authority: Option<anchor_client::anchor_lang::prelude::Pubkey>,
    ) -> anchor_client::anchor_lang::prelude::Pubkey {
        let token_account_key = Keypair::new().pubkey();

        let delegate = match delegate {
            Some(a) => COption::Some(a),
            _ => COption::None,
        };

        let is_native = match is_native {
            Some(a) => COption::Some(a),
            _ => COption::None,
        };

        let close_authority = match close_authority {
            Some(a) => COption::Some(a),
            _ => COption::None,
        };

        let r = Rent::default();
        let lamports = r.minimum_balance(spl_token::state::Account::LEN);

        let mut account =
            TridentAccount::new(lamports, spl_token::state::Account::LEN, &spl_token::id());

        let token_account = spl_token::state::Account {
            mint,
            owner,
            amount,
            delegate,
            state: spl_token::state::AccountState::Initialized,
            is_native,
            delegated_amount,
            close_authority,
        };

        let mut data = vec![0u8; spl_token::state::Account::LEN];
        spl_token::state::Account::pack(token_account, &mut data[..]).unwrap();
        account.set_data_from_slice(&data);

        self.account_storage
            .borrow_mut()
            .insert(token_account_key, account);

        token_account_key
    }

    fn set_mint_account(
        &mut self,
        decimals: u8,
        owner: &anchor_client::anchor_lang::prelude::Pubkey,
        freeze_authority: Option<anchor_client::anchor_lang::prelude::Pubkey>,
    ) -> anchor_client::anchor_lang::prelude::Pubkey {
        let mint_account_key = Keypair::new().pubkey();

        let authority = match freeze_authority {
            Some(a) => COption::Some(a),
            _ => COption::None,
        };

        let r = Rent::default();
        let lamports = r.minimum_balance(Mint::LEN);

        let mut account = TridentAccount::new(lamports, Mint::LEN, &spl_token::id());

        let mint = Mint {
            is_initialized: true,
            mint_authority: COption::Some(*owner),
            freeze_authority: authority,
            decimals,
            ..Default::default()
        };

        let mut data = vec![0u8; Mint::LEN];
        Mint::pack(mint, &mut data[..]).unwrap();
        account.set_data_from_slice(&data);
        // self.ctx.set_account(&mint_account.pubkey(), &account);
        self.account_storage
            .borrow_mut()
            .insert(mint_account_key, account);

        mint_account_key
    }

    fn payer(&self) -> solana_sdk::signature::Keypair {
        todo!()
    }

    fn get_account(
        &mut self,
        key: &anchor_client::anchor_lang::prelude::Pubkey,
    ) -> Result<Option<solana_sdk::account::Account>, FuzzClientError> {
        let storage = self.account_storage.borrow();
        let account = storage.get(key).ok_or(FuzzClientError::CannotGetAccounts)?;

        let account = Account {
            lamports: account.lamports,
            data: account.data.clone(),
            owner: account.owner,
            executable: account.executable,
            rent_epoch: account.rent_epoch,
        };
        Ok(Some(account.clone()))
    }

    fn get_accounts(
        &mut self,
        metas: &[anchor_client::anchor_lang::prelude::AccountMeta],
    ) -> Result<Vec<Option<Account>>, FuzzClientErrorWithOrigin> {
        let result: Vec<_> = metas
            .iter()
            .map(|m| {
                self.get_account(&m.pubkey)
                    .map_err(|e| e.with_origin(Origin::Account(m.pubkey)))
            })
            .collect();
        result.into_iter().collect()
    }

    fn get_last_blockhash(&self) -> solana_sdk::hash::Hash {
        todo!()
    }

    fn process_instruction(&mut self, instruction: Instruction) -> Result<(), FuzzClientError> {
        let mut account_infos = vec![];

        let mut temporary_account_storage = self.get_temporary_accounts(&instruction.accounts);

        for (account_meta, account_data) in temporary_account_storage.iter_mut() {
            let account_info = AccountInfo::new(
                &account_meta.pubkey,
                account_meta.is_signer,
                account_meta.is_writable,
                &mut account_data.lamports,
                &mut account_data.data,
                &account_data.owner,
                account_data.executable,
                account_data.rent_epoch,
            );
            account_infos.push(account_info);
        }

        let result = (self.entry)(&instruction.program_id, &account_infos, &instruction.data);

        // TODO acoount close trigger free() invalid pointer crash, if the account is not closed it works
        // you can try it with fuzz_example3
        match result {
            Ok(_) => {
                for account in account_infos.iter() {
                    if account.is_writable {
                        let mut storage = self.account_storage.borrow_mut();
                        let account_data = storage.get_mut(account.key).unwrap();
                        if is_closed(account) {
                            account_data.lamports = account.lamports.borrow_mut().to_owned();
                        } else {
                            account_data.data = account.data.borrow_mut().to_vec();
                            account_data.lamports = account.lamports.borrow_mut().to_owned();
                            account_data.owner = account.owner.to_owned();
                        }
                    }
                }
                Ok(())
            }
            Err(_e) => Err(FuzzClientError::Custom(10)),
        }
    }
}

// mimic anchor_lang
pub fn is_closed(info: &AccountInfo) -> bool {
    info.owner == &solana_system_program::id() && info.data_is_empty()
}
