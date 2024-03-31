use std::cell::RefCell;
use std::collections::HashMap;
use std::mem::size_of;

use solana_program::entrypoint::{
    deserialize, BPF_ALIGN_OF_U128, MAX_PERMITTED_DATA_INCREASE, NON_DUP_MARKER,
};
use solana_program::instruction::InstructionError;
use solana_program::{instruction::AccountMeta, program_pack::Pack, rent::Rent};
use solana_program_runtime::solana_rbpf::aligned_memory::{AlignedMemory, Pod};
use solana_program_runtime::solana_rbpf::ebpf::HOST_ALIGN;
use solana_program_test_anchor_fix::IndexOfAccount;
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

#[repr(C)]
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
        let mut accounts_ser = vec![];

        let mut temporary_account_storage = self.get_temporary_accounts(&instruction.accounts);

        let mut dedup_ixs: HashMap<Pubkey, u16> = HashMap::new();
        for (i, (account_meta, account_data)) in temporary_account_storage.iter_mut().enumerate() {
            let duplicate_ix = dedup_ixs.insert(account_meta.pubkey, i as u16);
            match duplicate_ix {
                Some(i_orig_ix) => accounts_ser.push(SerializeAccountCustom::Duplicate(i_orig_ix)),
                None => {
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

                    accounts_ser.push(SerializeAccountCustom::Account(i as u16, account_info));
                }
            }
        }

        let mut parameter_bytes = serialize_parameters_aligned_custom2(
            accounts_ser,
            &instruction.data,
            &instruction.program_id,
            true,
        )
        .unwrap();

        let (_program_id, account_infos, _input) =
            unsafe { deserialize(&mut parameter_bytes.as_slice_mut()[0] as *mut u8) };

        let result = (self.entry)(&instruction.program_id, &account_infos, &instruction.data);
        match result {
            Ok(_) => {
                for account in account_infos.iter() {
                    if account.is_writable {
                        let mut storage = self.account_storage.borrow_mut();
                        let account_data = storage.get_mut(account.key).unwrap();
                        // if is_closed(account) {
                        //     // FIXME closed account must have no balance (0 lamports) - why not to remove the account from storage?
                        //     account_data.lamports = account.lamports.borrow_mut().to_owned();
                        //     println!("### ACCOUNT CLOSED...");
                        // } else {
                        account_data.data = account.data.borrow().to_vec();
                        account_data.lamports = account.lamports.borrow().to_owned();
                        account_data.owner = account.owner.to_owned();
                        // TODO check data can be resized
                        // TODO check data can be changed
                        // TODO check lamports sum is constant
                        // }
                    }
                }
                Ok(())
            }
            Err(_e) => Err(FuzzClientError::Custom(10)), // FIXME The ProgramError has to be propagated here
        }
    }
}

// mimic anchor_lang
pub fn is_closed(info: &AccountInfo) -> bool {
    info.owner == &solana_system_program::id() && info.data_is_empty() // FIXME lamports must also be set to 0
}

enum SerializeAccountCustom<'info> {
    Account(IndexOfAccount, AccountInfo<'info>),
    Duplicate(IndexOfAccount),
}

fn serialize_parameters_aligned_custom2(
    accounts: Vec<SerializeAccountCustom>,
    instruction_data: &[u8],
    program_id: &Pubkey,
    copy_account_data: bool,
) -> Result<AlignedMemory<HOST_ALIGN>, InstructionError> {
    // Calculate size in order to alloc once
    let mut size = size_of::<u64>();
    for account in &accounts {
        size += 1; // dup
        match account {
            SerializeAccountCustom::Duplicate(_) => size += 7, // padding to 64-bit aligned
            SerializeAccountCustom::Account(_, account) => {
                let data_len = account.data_len();
                size += size_of::<u8>() // is_signer
                + size_of::<u8>() // is_writable
                + size_of::<u8>() // executable
                + size_of::<u32>() // original_data_len
                + size_of::<Pubkey>()  // key
                + size_of::<Pubkey>() // owner
                + size_of::<u64>()  // lamports
                + size_of::<u64>()  // data len
                + MAX_PERMITTED_DATA_INCREASE
                + size_of::<u64>(); // rent epoch
                if copy_account_data {
                    size += data_len + (data_len as *const u8).align_offset(BPF_ALIGN_OF_U128);
                } else {
                    size += BPF_ALIGN_OF_U128;
                }
            }
        }
    }
    size += size_of::<u64>() // data len
    + instruction_data.len()
    + size_of::<Pubkey>(); // program id;

    let mut s = SerializerCustomLight::new(size, true, copy_account_data);

    // Serialize into the buffer
    s.write::<u64>((accounts.len() as u64).to_le());
    for account in accounts {
        match account {
            SerializeAccountCustom::Account(_, mut borrowed_account) => {
                s.write::<u8>(NON_DUP_MARKER);
                s.write::<u8>(borrowed_account.is_signer as u8);
                s.write::<u8>(borrowed_account.is_writable as u8);
                s.write::<u8>(borrowed_account.executable as u8);
                s.write_all(&[0u8, 0, 0, 0]);
                s.write_all(borrowed_account.key.as_ref());
                s.write_all(borrowed_account.owner.as_ref());
                s.write::<u64>(borrowed_account.lamports().to_le());
                s.write::<u64>((borrowed_account.data_len() as u64).to_le());
                s.write_account(&mut borrowed_account)?;
                s.write::<u64>((borrowed_account.rent_epoch).to_le());
            }
            SerializeAccountCustom::Duplicate(position) => {
                s.write::<u8>(position as u8);
                s.write_all(&[0u8, 0, 0, 0, 0, 0, 0]);
            }
        };
    }
    s.write::<u64>((instruction_data.len() as u64).to_le());
    s.write_all(instruction_data);
    s.write_all(program_id.as_ref());

    let mem = s.finish();
    Ok(mem)
}

struct SerializerCustomLight {
    pub buffer: AlignedMemory<HOST_ALIGN>,
    aligned: bool,
    copy_account_data: bool,
}
impl SerializerCustomLight {
    fn new(size: usize, aligned: bool, copy_account_data: bool) -> SerializerCustomLight {
        SerializerCustomLight {
            buffer: AlignedMemory::with_capacity(size),
            aligned,
            copy_account_data,
        }
    }

    fn fill_write(&mut self, num: usize, value: u8) -> std::io::Result<()> {
        self.buffer.fill_write(num, value)
    }

    pub fn write<T: Pod>(&mut self, value: T) {
        // Safety:
        // in serialize_parameters_(aligned|unaligned) first we compute the
        // required size then we write into the newly allocated buffer. There's
        // no need to check bounds at every write.
        //
        // AlignedMemory::write_unchecked _does_ debug_assert!() that the capacity
        // is enough, so in the unlikely case we introduce a bug in the size
        // computation, tests will abort.
        unsafe {
            self.buffer.write_unchecked(value);
        }
    }

    fn write_all(&mut self, value: &[u8]) {
        // Safety:
        // see write() - the buffer is guaranteed to be large enough
        unsafe {
            self.buffer.write_all_unchecked(value);
        }
    }

    fn write_account(&mut self, account: &mut AccountInfo<'_>) -> Result<(), InstructionError> {
        if self.copy_account_data {
            self.write_all(*account.data.borrow());
        };

        if self.aligned {
            let align_offset = (account.data_len() as *const u8).align_offset(BPF_ALIGN_OF_U128);
            if self.copy_account_data {
                self.fill_write(MAX_PERMITTED_DATA_INCREASE + align_offset, 0)
                    .map_err(|_| InstructionError::InvalidArgument)?;
            } else {
                // The deserialization code is going to align the vm_addr to
                // BPF_ALIGN_OF_U128. Always add one BPF_ALIGN_OF_U128 worth of
                // padding and shift the start of the next region, so that once
                // vm_addr is aligned, the corresponding host_addr is aligned
                // too.
                self.fill_write(MAX_PERMITTED_DATA_INCREASE + BPF_ALIGN_OF_U128, 0)
                    .map_err(|_| InstructionError::InvalidArgument)?;
            }
        }

        Ok(())
    }

    fn finish(self) -> AlignedMemory<HOST_ALIGN> {
        self.buffer
    }
}
