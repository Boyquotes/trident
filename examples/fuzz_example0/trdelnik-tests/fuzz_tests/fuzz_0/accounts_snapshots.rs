use fuzz_example0::Counter;
use trdelnik_client::anchor_lang::solana_program::instruction::AccountMeta;
use trdelnik_client::anchor_lang::{self, prelude::*};
use trdelnik_client::fuzzing::{get_account_infos_option, FuzzingError};

/// Type representing an account used during fuzzing. An [`FuzzAccOption`] can be
/// either `None` meaning the account does not exist, `Typed` meaning that the account
/// was successfully deserialized into the expected data type or `Raw` meaning the
/// account exists but the deserialization failed.
///
/// One example when it happens is during account initialization where a user submits
/// an existing arbitrary account and Anchor sets the appropriate data and owner fields. Before
/// the initialization, the account cannot be deserialized using `try_from` method because it
/// verifies the correct owner and data structure.
pub enum FuzzAccOption<'a, T> {
    /// Uninitialized account
    None,
    /// Deserialized account to type `T`
    Typed(T),
    /// Raw account as `AccountInfo`
    Raw(AccountInfo<'a>),
}

impl<'info, T> From<std::result::Result<Option<T>, AccountInfo<'info>>>
    for FuzzAccOption<'info, T>
{
    fn from(value: std::result::Result<Option<T>, AccountInfo<'info>>) -> Self {
        match value {
            Ok(val) => match val {
                Some(acc) => FuzzAccOption::Typed(acc),
                None => FuzzAccOption::None,
            },
            Err(acc) => FuzzAccOption::Raw(acc),
        }
    }
}

pub struct InitializeSnapshot<'info> {
    pub counter: FuzzAccOption<'info, Account<'info, Counter>>,
    pub user: Option<Signer<'info>>,
    pub system_program: Option<Program<'info, System>>,
}
pub struct UpdateSnapshot<'info> {
    pub counter: Option<Account<'info, Counter>>,
    pub authority: Option<Signer<'info>>,
}
impl<'info> InitializeSnapshot<'info> {
    pub fn deserialize_option(
        metas: &'info [AccountMeta],
        accounts: &'info mut [Option<trdelnik_client::solana_sdk::account::Account>],
    ) -> core::result::Result<Self, FuzzingError> {
        let accounts = get_account_infos_option(accounts, metas)
            .map_err(|_| FuzzingError::CannotGetAccounts)?;
        let mut accounts_iter = accounts.into_iter();
        // let counter: Option<anchor_lang::accounts::account::Account<Counter>> = accounts_iter
        //     .next()
        //     .ok_or(FuzzingError::NotEnoughAccounts)?
        //     .map(|acc| anchor_lang::accounts::account::Account::try_from(&acc))
        //     .transpose()
        //     .unwrap_or(None);

        let counter: FuzzAccOption<'info, anchor_lang::accounts::account::Account<Counter>> =
            accounts_iter
                .next()
                .ok_or(FuzzingError::NotEnoughAccounts)?
                .map(|acc| anchor_lang::accounts::account::Account::try_from(&acc).or(Err(acc)))
                .transpose()
                .into();

        let user: Option<Signer<'_>> = accounts_iter
            .next()
            .ok_or(FuzzingError::NotEnoughAccounts)?
            .map(|acc| anchor_lang::accounts::signer::Signer::try_from(&acc))
            .transpose()
            .map_err(|_| FuzzingError::CannotDeserializeAccount)?;
        let system_program: Option<anchor_lang::accounts::program::Program<System>> = accounts_iter
            .next()
            .ok_or(FuzzingError::NotEnoughAccounts)?
            .map(|acc| anchor_lang::accounts::program::Program::try_from(&acc))
            .transpose()
            .map_err(|_| FuzzingError::CannotDeserializeAccount)?;
        Ok(Self {
            counter,
            user,
            system_program,
        })
    }
}
impl<'info> UpdateSnapshot<'info> {
    pub fn deserialize_option(
        metas: &'info [AccountMeta],
        accounts: &'info mut [Option<trdelnik_client::solana_sdk::account::Account>],
    ) -> core::result::Result<Self, FuzzingError> {
        let accounts = get_account_infos_option(accounts, metas)
            .map_err(|_| FuzzingError::CannotGetAccounts)?;
        let mut accounts_iter = accounts.into_iter();
        let counter: Option<anchor_lang::accounts::account::Account<Counter>> = accounts_iter
            .next()
            .ok_or(FuzzingError::NotEnoughAccounts)?
            .map(|acc| anchor_lang::accounts::account::Account::try_from(&acc))
            .transpose()
            .unwrap_or(None);
        let authority: Option<Signer<'_>> = accounts_iter
            .next()
            .ok_or(FuzzingError::NotEnoughAccounts)?
            .map(|acc| anchor_lang::accounts::signer::Signer::try_from(&acc))
            .transpose()
            .map_err(|_| FuzzingError::CannotDeserializeAccount)?;
        Ok(Self { counter, authority })
    }
}
