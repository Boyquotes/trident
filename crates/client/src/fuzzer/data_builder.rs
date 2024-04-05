use anchor_client::anchor_lang::solana_program::account_info::{Account as Acc, AccountInfo};
use anchor_client::anchor_lang::solana_program::hash::Hash;
use anchor_lang::prelude::Rent;
use arbitrary::Arbitrary;
use arbitrary::Unstructured;
use solana_sdk::account::{Account, AccountSharedData};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::transaction::VersionedTransaction;
use std::cell::RefCell;
use std::error::Error;
use std::fmt::Display;

use crate::accounts_storage::PdaStore;
use crate::error::*;

pub struct FuzzData<T, U> {
    pub pre_ixs: Vec<T>,
    pub ixs: Vec<T>,
    pub post_ixs: Vec<T>,
    pub accounts: RefCell<U>,
}

pub struct FuzzDataIterator<'a, T> {
    pre_ixs_iter: std::slice::Iter<'a, T>,
    ixs_iter: std::slice::Iter<'a, T>,
    post_ixs_iter: std::slice::Iter<'a, T>,
}

impl<T, U> FuzzData<T, U> {
    pub fn iter(&self) -> FuzzDataIterator<'_, T> {
        FuzzDataIterator {
            pre_ixs_iter: self.pre_ixs.iter(),
            ixs_iter: self.ixs.iter(),
            post_ixs_iter: self.post_ixs.iter(),
        }
    }
}

impl<'a, T> Iterator for FuzzDataIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.pre_ixs_iter
            .next()
            .or_else(|| self.ixs_iter.next())
            .or_else(|| self.post_ixs_iter.next())
    }
}

impl<T, U> FuzzData<T, U>
where
    T: FuzzTestExecutor<U> + Display,
{
    pub fn run_with_runtime(
        &self,
        program_id: Pubkey,
        client: &mut impl FuzzClient,
    ) -> core::result::Result<(), Box<dyn Error + 'static>> {
        // solana_logger::setup_with_default("off");
        // #[cfg(fuzzing_debug)]
        // solana_logger::setup_with_default(
        //     "solana_rbpf::vm=debug,\
        //         solana_runtime::message_processor=debug,\
        //         solana_runtime::system_instruction_processor=trace,\
        //         solana_program_test=info,\
        //         fuzz_target=info",
        // );

        #[cfg(fuzzing_debug)]
        {
            eprintln!("Instructions sequence:");
            for ix in self.iter() {
                eprintln!("{}", ix);
            }
            eprintln!("------ End of Instructions sequence ------ ");
        }

        for fuzz_ix in &mut self.iter() {
            #[cfg(fuzzing_debug)]
            eprintln!("Currently processing: {}", fuzz_ix);

            fuzz_ix.run_fuzzer(program_id, &self.accounts, client)?;
        }
        Ok(())
    }
}

pub trait FuzzTestExecutor<T> {
    fn run_fuzzer(
        &self,
        program_id: Pubkey,
        accounts: &RefCell<T>,
        client: &mut impl FuzzClient,
    ) -> core::result::Result<(), Box<dyn Error + 'static>>;
}

#[allow(unused_variables)]
pub trait FuzzDataBuilder<T: for<'a> Arbitrary<'a>> {
    /// The instruction(s) executed as first, can be used for initialization.
    fn pre_ixs(u: &mut Unstructured) -> arbitrary::Result<Vec<T>> {
        Ok(vec![])
    }

    /// The main instructions for fuzzing.
    fn ixs(u: &mut Unstructured) -> arbitrary::Result<Vec<T>> {
        let v = <Vec<T>>::arbitrary(u)?;
        // Return always a vector with at least one element, othewise return error.
        if v.is_empty() {
            return Err(arbitrary::Error::NotEnoughData);
        }
        Ok(v)
    }

    /// The instuction(s) executed as last.
    fn post_ixs(u: &mut Unstructured) -> arbitrary::Result<Vec<T>> {
        Ok(vec![])
    }
}

pub trait IxOps<'info> {
    type IxData;
    type IxAccounts;
    type IxSnapshot;
    // TODO maybe generate the From trait and return Ok(self.data.into())
    fn get_data(
        &self,
        client: &mut impl FuzzClient,
        fuzz_accounts: &mut Self::IxAccounts,
    ) -> Result<Self::IxData, FuzzingError>;

    fn get_accounts(
        &self,
        client: &mut impl FuzzClient,
        fuzz_accounts: &mut Self::IxAccounts,
    ) -> Result<(Vec<Keypair>, Vec<AccountMeta>), FuzzingError>;

    // TODO implement better error with source and description
    #[allow(unused_variables)]
    fn check(
        &self,
        pre_ix: Self::IxSnapshot,
        post_ix: Self::IxSnapshot,
        ix_data: Self::IxData,
    ) -> Result<(), FuzzingError> {
        Ok(())
    }
}

pub trait FuzzDeserialize<'info> {
    type Ix;
    // TODO return also remaining accounts

    fn deserialize_option(
        &self,
        accounts: &'info mut [Option<AccountInfo<'info>>],
    ) -> Result<Self::Ix, FuzzingError>;
}

/// A trait providing methods to read and write (manipulate) accounts
pub trait FuzzClient {
    /// Create or overwrite a custom account, subverting normal runtime checks.
    fn set_account_custom(&mut self, address: &Pubkey, account: &AccountSharedData);

    /// Create an empty account and add lamports to it
    fn set_account(&mut self, lamports: u64) -> Keypair;

    fn set_pda_account(&mut self, seeds: &[&[u8]], program_id: &Pubkey) -> Option<PdaStore>;

    /// Create an SPL token account
    #[allow(clippy::too_many_arguments)]
    fn set_token_account(
        &mut self,
        mint: Pubkey,
        owner: Pubkey,
        amount: u64,
        delegate: Option<Pubkey>,
        is_native: Option<u64>,
        delegated_amount: u64,
        close_authority: Option<Pubkey>,
    ) -> Pubkey;

    /// Create an SPL mint account
    fn set_mint_account(
        &mut self,
        decimals: u8,
        owner: &Pubkey,
        freeze_authority: Option<Pubkey>,
    ) -> Pubkey;

    /// Get the Keypair of the client's payer account
    fn payer(&self) -> Keypair;

    /// Get the account at the given address
    fn get_account(&mut self, key: &Pubkey) -> Result<Option<Account>, FuzzClientError>;

    // TODO remove from this trait and place it elsewhere
    /// Get accounts based on the supplied meta information
    fn get_accounts(
        &mut self,
        metas: &[AccountMeta],
    ) -> Result<Vec<Option<Account>>, FuzzClientErrorWithOrigin>;

    /// Get last blockhash
    fn get_last_blockhash(&self) -> Hash;

    /// Get the cluster rent
    fn get_rent(&mut self) -> Result<Rent, FuzzClientError>;

    /// Send a transaction and return until the transaction has been finalized or rejected.
    fn process_transaction(
        &mut self,
        _transaction: impl Into<VersionedTransaction>,
    ) -> Result<(), FuzzClientError> {
        Err(FuzzClientError::NotImplemnted)
    }
    fn process_instruction(&mut self, _instruction: Instruction) -> Result<(), FuzzClientError> {
        Err(FuzzClientError::NotImplemnted)
    }
}

#[macro_export]
macro_rules! fuzz_trident {
    ($ix:ident: $ix_dty:ident , |$buf:ident: $dty:ident| $body:block) => {
        fuzz(|$buf| {
            let mut $buf: FuzzData<$ix_dty, _> = {
                use arbitrary::Unstructured;

                let mut buf = Unstructured::new($buf);
                if let Ok(fuzz_data) = build_ix_fuzz_data($dty {}, &mut buf) {
                    fuzz_data
                } else {
                    return;
                }
            };
            $body
        });
    };
}
/// Prints the details of a given account in a pretty-printed format.
///
/// This macro takes a single argument, which is an expression referring to the account
/// you want to print. The account data structure must implement or derive the [`Debug`]
/// trait for this macro to work, as it relies on `std::fmt::Debug` for formatting.
///
/// # Examples
///
/// ```rust,ignore
/// use trident_client::fuzzing::show_account;
///
/// #[derive(Debug)]
/// #[account]
/// struct Escrow {
///     recipeint: Pubkey,
///     id: u32,
///     balance: f64,
///     name: String,
/// }
///
/// fn check(
///     &self,
///     pre_ix: Self::IxSnapshot,
///     post_ix: Self::IxSnapshot,
///     ix_data: Self::IxData,
/// ) -> Result<(), FuzzingError> {
///     if let Some(escrow) = pre_ix.escrow{
///         show_account!(escrow);
///     }
/// }
/// ```
///
/// # Requirements
///
/// The `account` passed to `show_account!` must implement or derive the [`Debug`] trait.
/// Attempting to use this macro with a type that does not meet this requirement will
/// result in a compilation error.
#[macro_export]
macro_rules! show_account {
    ($account:expr) => {
        eprintln!("{:#?}", $account);
    };
}

pub fn build_ix_fuzz_data<U: for<'a> Arbitrary<'a>, T: FuzzDataBuilder<U>, V: Default>(
    _data_builder: T,
    u: &mut arbitrary::Unstructured,
) -> arbitrary::Result<FuzzData<U, V>> {
    Ok(FuzzData {
        pre_ixs: T::pre_ixs(u)?,
        ixs: T::ixs(u)?,
        post_ixs: T::post_ixs(u)?,
        accounts: RefCell::new(V::default()),
    })
}

/// Creates `AccountInfo`s from `Accounts` and corresponding `AccountMeta` slices.
pub fn get_account_infos_option<'info>(
    accounts: &'info mut [Option<Account>],
    metas: &'info [AccountMeta],
) -> Result<Vec<Option<AccountInfo<'info>>>, FuzzingError> {
    let iter = accounts.iter_mut().zip(metas);
    let r = iter
        .map(|(account, meta)| {
            if let Some(account) = account {
                let (lamports, data, owner, executable, rent_epoch) = account.get();
                Some(AccountInfo::new(
                    &meta.pubkey,
                    meta.is_signer,
                    meta.is_writable,
                    lamports,
                    data,
                    owner,
                    executable,
                    rent_epoch,
                ))
            } else {
                None
            }
        })
        .collect();

    Ok(r)
}
