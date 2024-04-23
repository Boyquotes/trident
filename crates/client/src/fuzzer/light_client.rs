use std::cell::RefCell;
use std::collections::HashMap;
use std::mem::{size_of, transmute};
use std::rc::Rc;
use std::slice::from_raw_parts_mut;

use solana_program::entrypoint::{
    ProcessInstruction, ProgramResult, BPF_ALIGN_OF_U128, MAX_PERMITTED_DATA_INCREASE,
    NON_DUP_MARKER,
};
use solana_program::instruction::InstructionError;
use solana_program::sysvar::rent;
use solana_program::{program_pack::Pack, rent::Rent};
use solana_program_runtime::solana_rbpf::aligned_memory::{AlignedMemory, Pod};
use solana_program_runtime::solana_rbpf::ebpf::HOST_ALIGN;
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

pub type ProgramEntry = for<'info, 'b> fn(
    program_id: &Pubkey,
    accounts: &'info [AccountInfo<'b>],
    instruction_data: &[u8],
) -> ProgramResult;

#[repr(C)]
#[derive(Clone, Debug, Default)]

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

thread_local! {
    /// Static pointer to LightClient so that we can access it from system program stubs
    pub static LIGHT_CLIENT: RefCell<Option<usize>> = RefCell::new(None);
}

/// Sets the pointer to LightClient. This is necessary in order to access LightClient using 'get_light_client' method
fn set_light_client(new: &LightClient) {
    LIGHT_CLIENT.with(|client| unsafe { client.replace(Some(transmute::<_, usize>(new))) });
}

/// Gets the pointer to LightClient
pub(crate) fn get_light_client<'a>() -> &'a LightClient {
    let ptr = LIGHT_CLIENT.with(|client| match *client.borrow() {
        Some(val) => val,
        None => panic!("Light client not set! Maybe you have to call init() first."),
    });
    unsafe { transmute::<usize, &LightClient>(ptr) }
}

pub struct LightClient {
    pub account_storage: HashMap<Pubkey, TridentAccount>,
    pub programs: HashMap<Pubkey, ProcessInstruction>,
}

impl LightClient {
    /// Create new LightClient instance with a program entrypoint. Call the `init()` method before using the client.
    // TODO make the same order of parameters as in add_program
    pub fn new(entry: ProgramEntry, program_id: Pubkey) -> Result<Self, FuzzClientError> {
        let mut new_client = Self {
            account_storage: HashMap::new(),
            programs: HashMap::new(),
        };
        new_client.add_system_program();
        new_client.add_program(program_id, entry);
        new_client.add_program(
            anchor_spl::token::ID,
            spl_token::processor::Processor::process,
        );
        new_client.add_program(
            anchor_spl::token_2022::ID,
            spl_token_2022::processor::Processor::process,
        );
        new_client.add_program(
            anchor_spl::associated_token::ID,
            spl_associated_token_account::processor::process_instruction,
        );

        // TODO support also other sysvars
        new_client.add_rent()?;

        test_syscall_stubs(program_id);

        Ok(new_client)
    }

    /// Initializes the LightClient before usage.
    pub fn init(&self) {
        set_light_client(self);
    }

    fn add_rent(&mut self) -> Result<(), FuzzClientError> {
        let rent = Rent::default();
        let size = size_of::<Rent>();
        let mut data = vec![0; size];
        bincode::serialize_into(&mut data[..], &rent)
            .map_err(|e| FuzzClientError::ClientInitError(e))?;

        let lamports = rent.minimum_balance(data.len());

        let mut account = TridentAccount::new(lamports, size, &solana_program::sysvar::id());

        account.set_data_from_slice(&data[..]);
        self.account_storage.insert(rent::id(), account);
        Ok(())
    }

    fn add_system_program(&mut self) {
        let rent = Rent::default().minimum_balance(0).max(1);
        let program = TridentAccount {
            executable: true,
            lamports: rent,
            ..Default::default()
        };
        self.account_storage
            .insert(solana_sdk::system_program::ID, program);
    }

    /// Add new arbitrary program to the client.
    ///
    /// - `program_id` is the address of your program
    /// - `process_function` is the closure that will be called to enter the program and process instructions
    pub fn add_program(
        &mut self,
        program_id: Pubkey,
        process_function: solana_sdk::entrypoint::ProcessInstruction,
    ) {
        self.programs.insert(program_id, process_function);

        let rent = Rent::default().minimum_balance(0).max(1);
        let program = TridentAccount {
            executable: true,
            lamports: rent,
            ..Default::default()
        };
        self.account_storage.insert(program_id, program);
    }
}

impl FuzzClient for LightClient {
    fn get_rent(&mut self) -> Result<Rent, FuzzClientError> {
        Ok(solana_sdk::rent::Rent::default())
    }
    fn set_account_custom(&mut self, address: &Pubkey, account: &AccountSharedData) {
        self.account_storage
            .insert(*address, account.to_owned().into());
    }

    fn set_account(&mut self, lamports: u64) -> solana_sdk::signature::Keypair {
        let new_account = Keypair::new();

        let new_account_info = TridentAccount::new(lamports, 0, &solana_sdk::system_program::ID);

        self.account_storage
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
            self.account_storage.insert(key, empty_account);
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

        self.account_storage.insert(token_account_key, account);

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
        self.account_storage.insert(mint_account_key, account);

        mint_account_key
    }

    fn payer(&self) -> solana_sdk::signature::Keypair {
        todo!()
    }

    fn get_account(
        &mut self,
        key: &anchor_client::anchor_lang::prelude::Pubkey,
    ) -> Result<Option<solana_sdk::account::Account>, FuzzClientError> {
        let storage = &self.account_storage; //.borrow();
        match storage.get(key) {
            Some(account) => Ok(Some(Account {
                lamports: account.lamports,
                data: account.data.clone(),
                owner: account.owner,
                executable: account.executable,
                rent_epoch: account.rent_epoch,
            })),
            None => Ok(None),
        }
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
        let mut instruction = instruction;
        let mut dedup_ixs: HashMap<Pubkey, usize> =
            HashMap::with_capacity(instruction.accounts.len());

        for i in 0..instruction.accounts.len() {
            match dedup_ixs.get(&instruction.accounts[i].pubkey) {
                // the account is already in the HashMap, so it is a duplicate
                Some(&reference_index) => {
                    //  if the duplicate account is signer or writable, make sure also the referrence account is set accordingly
                    if instruction.accounts[i].is_signer {
                        instruction.accounts[reference_index].is_signer = true;
                    }
                    if instruction.accounts[i].is_writable {
                        instruction.accounts[reference_index].is_writable = true;
                    }
                }
                // the account is not yet in the HashMap, so it is not a duplicate
                None => {
                    dedup_ixs.insert(instruction.accounts[i].pubkey, i);
                }
            };
        }

        // We expect duplicate accounts will be only minority so we return references to all accounts
        let account_refs = &instruction
            .accounts
            .iter()
            .map(|m| self.account_storage.get(&m.pubkey))
            .collect::<Vec<_>>();
        let duplicate_accounts = instruction.accounts.len() - dedup_ixs.len();
        let mut size = size_of::<u64>(); // number of accounts
        size += 8 * duplicate_accounts; // Duplicate accounts are represented by 1 byte duplicate flag plus 7 padding bytes to 64-aligned.
        for &acc in account_refs.iter() {
            let data_len = match acc {
                Some(acc) => acc.data.len(),
                None => 0,
            };
            // This block is 64-bit aligned
            size += size_of::<u8>() // duplicate flag
                + size_of::<u8>() // is_signer
                + size_of::<u8>() // is_writable
                + size_of::<u8>() // executable
                + size_of::<u32>() // original_data_len
                + size_of::<Pubkey>()  // key
                + size_of::<Pubkey>() // owner
                + size_of::<u64>()  // lamports
                + size_of::<u64>()  // data len
                + MAX_PERMITTED_DATA_INCREASE
                + size_of::<u64>(); // rent epoch
            size += data_len + (data_len as *const u8).align_offset(BPF_ALIGN_OF_U128);
        }

        let mut s = SerializerCustomLight::new(size);

        // Serialize into the buffer
        s.write::<u64>((instruction.accounts.len() as u64).to_le());
        for (i, (account_meta, account)) in
            instruction.accounts.iter().zip(account_refs).enumerate()
        {
            // We can unwrap, it should never be None.
            let position = dedup_ixs.get(&account_meta.pubkey).unwrap();

            if i == *position {
                // first occurence of the account
                match account {
                    Some(account) => {
                        s.write::<u8>(NON_DUP_MARKER);
                        s.write::<u8>(account_meta.is_signer as u8);
                        s.write::<u8>(account_meta.is_writable as u8);
                        s.write::<u8>(account.executable as u8);
                        s.write_all(&[0u8, 0, 0, 0]);
                        s.write_all(account_meta.pubkey.as_ref());
                        s.write_all(account.owner.as_ref());
                        s.write::<u64>(account.lamports.to_le());
                        s.write::<u64>((account.data.len() as u64).to_le());
                        s.write_account_custom(account).unwrap();
                        s.write::<u64>((account.rent_epoch).to_le());
                    }
                    None => {
                        let account = TridentAccount::default();
                        s.write::<u8>(NON_DUP_MARKER);
                        s.write::<u8>(account_meta.is_signer as u8);
                        s.write::<u8>(account_meta.is_writable as u8);
                        s.write::<u8>(account.executable as u8);
                        s.write_all(&[0u8, 0, 0, 0]);
                        s.write_all(account_meta.pubkey.as_ref());
                        s.write_all(account.owner.as_ref());
                        s.write::<u64>(account.lamports.to_le());
                        s.write::<u64>((account.data.len() as u64).to_le());
                        s.write_account_custom(&account).unwrap();
                        s.write::<u64>((account.rent_epoch).to_le());
                    }
                };
            } else {
                // it is a duplicate
                s.write::<u8>(*position as u8);
                s.write_all(&[0u8, 0, 0, 0, 0, 0, 0]);
            }
        }

        let mut parameter_bytes = s.finish();

        let account_infos =
            unsafe { deserialize_custom(&mut parameter_bytes.as_slice_mut()[0] as *mut u8) };

        match self.programs.get(&instruction.program_id) {
            Some(entrypoint) => {
                (entrypoint)(&instruction.program_id, &account_infos, &instruction.data)
                    .map_err(FuzzClientError::ProgramError)?
            }
            None if instruction.program_id == solana_system_program::id() => {
                solana_program::program::invoke(&instruction, &account_infos)
                    .map_err(FuzzClientError::ProgramError)?
            }
            None => Err(FuzzClientError::ProgramNotFound(instruction.program_id))?,
        };

        // let result = (self.entry)(&instruction.program_id, &account_infos, &instruction.data);
        for account in account_infos.iter() {
            if account.is_writable {
                if is_closed(account) {
                    // TODO if we remove the account, what about AccountStorage?
                    self.account_storage.remove(account.key);
                } else {
                    let account_data = self.account_storage.entry(*account.key).or_default();
                    account_data.data = account.data.borrow().to_vec();
                    account_data.lamports = account.lamports.borrow().to_owned();
                    account_data.owner = account.owner.to_owned();
                    // TODO check data can be resized
                    // TODO check lamports sum is constant
                }
            }
        }
        Ok(())
    }
}

/// Deserialize the input arguments
///
/// The integer arithmetic in this method is safe when called on a buffer that was
/// serialized by Trident. Use with buffers serialized otherwise is unsupported and
/// done at one's own risk.
///
/// # Safety
pub(crate) unsafe fn deserialize_custom<'a>(input: *mut u8) -> Vec<AccountInfo<'a>> {
    let mut offset: usize = 0;

    // Number of accounts present

    #[allow(clippy::cast_ptr_alignment)]
    let num_accounts = *(input.add(offset) as *const u64) as usize;
    offset += size_of::<u64>();

    // Account Infos

    let mut accounts = Vec::with_capacity(num_accounts);
    for _ in 0..num_accounts {
        let dup_info = *(input.add(offset) as *const u8);
        offset += size_of::<u8>();
        if dup_info == NON_DUP_MARKER {
            #[allow(clippy::cast_ptr_alignment)]
            let is_signer = *(input.add(offset) as *const u8) != 0;
            offset += size_of::<u8>();

            #[allow(clippy::cast_ptr_alignment)]
            let is_writable = *(input.add(offset) as *const u8) != 0;
            offset += size_of::<u8>();

            #[allow(clippy::cast_ptr_alignment)]
            let executable = *(input.add(offset) as *const u8) != 0;
            offset += size_of::<u8>();

            // The original data length is stored here because these 4 bytes were
            // originally only used for padding and served as a good location to
            // track the original size of the account data in a compatible way.
            let original_data_len_offset = offset;
            offset += size_of::<u32>();

            let key: &Pubkey = &*(input.add(offset) as *const Pubkey);
            offset += size_of::<Pubkey>();

            let owner: &Pubkey = &*(input.add(offset) as *const Pubkey);
            offset += size_of::<Pubkey>();

            #[allow(clippy::cast_ptr_alignment)]
            let lamports = Rc::new(RefCell::new(&mut *(input.add(offset) as *mut u64)));
            offset += size_of::<u64>();

            #[allow(clippy::cast_ptr_alignment)]
            let data_len = *(input.add(offset) as *const u64) as usize;
            offset += size_of::<u64>();

            // Store the original data length for detecting invalid reallocations and
            // requires that MAX_PERMITTED_DATA_LENGTH fits in a u32
            *(input.add(original_data_len_offset) as *mut u32) = data_len as u32;

            let data = Rc::new(RefCell::new({
                from_raw_parts_mut(input.add(offset), data_len)
            }));
            offset += data_len + MAX_PERMITTED_DATA_INCREASE;
            offset += (offset as *const u8).align_offset(BPF_ALIGN_OF_U128); // padding

            #[allow(clippy::cast_ptr_alignment)]
            let rent_epoch = *(input.add(offset) as *const u64);
            offset += size_of::<u64>();

            accounts.push(AccountInfo {
                key,
                is_signer,
                is_writable,
                lamports,
                data,
                owner,
                executable,
                rent_epoch,
            });
        } else {
            offset += 7; // padding

            // Duplicate account, clone the original
            accounts.push(accounts[dup_info as usize].clone());
        }
    }

    accounts
}

pub fn is_closed(info: &AccountInfo) -> bool {
    info.owner == &solana_system_program::id() && info.data_is_empty() && info.lamports() == 0
}

struct SerializerCustomLight {
    buffer: AlignedMemory<HOST_ALIGN>,
}

impl SerializerCustomLight {
    fn new(size: usize) -> SerializerCustomLight {
        SerializerCustomLight {
            buffer: AlignedMemory::with_capacity(size),
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

    fn write_account_custom(&mut self, account: &TridentAccount) -> Result<(), InstructionError> {
        self.write_all(&account.data);
        let align_offset = (account.data.len() as *const u8).align_offset(BPF_ALIGN_OF_U128);
        self.fill_write(MAX_PERMITTED_DATA_INCREASE + align_offset, 0)
            .map_err(|_| InstructionError::InvalidArgument)?;

        Ok(())
    }

    fn finish(self) -> AlignedMemory<HOST_ALIGN> {
        self.buffer
    }
}
