use std::slice::from_raw_parts_mut;

use anchor_lang::{prelude::*, solana_program::{system_program, program_memory::sol_memset}};
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};

use crate::{state::Escrow, VestingError};

pub fn _withdraw_unlocked(ctx: Context<WithdrawUnlocked>) -> Result<()> {
    let escrow = &mut ctx.accounts.escrow;

    let current_time = Clock::get()?.unix_timestamp as u64;
    let unlocked_amount = escrow
        .amount_unlocked(current_time)
        .ok_or(VestingError::InvalidAmount)?;

    let bump = ctx.bumps.escrow_pda_authority;
    let seeds = &[b"ESCROW_PDA_AUTHORITY".as_ref(), &[bump]];

    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.escrow_token_account.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: ctx.accounts.escrow_pda_authority.to_account_info(),
            },
        )
        .with_signer(&[&seeds[..]]),
        unlocked_amount,
    )?;

    escrow.withdrawal += unlocked_amount;

    // close(
    //     escrow.to_account_info(),
    //     ctx.accounts.recipient.to_account_info(),
    // )
    // .unwrap();

    Ok(())
}

#[derive(Accounts)]
pub struct WithdrawUnlocked<'info> {
    #[account(mut)]
    pub recipient: Signer<'info>,

    #[account(mut,
        token::mint = mint,
        token::authority = recipient
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        has_one = recipient,
        close = recipient,
        seeds = [escrow.recipient.key().as_ref(),b"ESCROW_SEED"],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = escrow_pda_authority  // only the program has the authority as this is a PDA
    )]
    pub escrow_token_account: Account<'info, TokenAccount>,

    /// CHECK: we do not read or write to this account
    #[account(
        seeds = [b"ESCROW_PDA_AUTHORITY"],
        bump
    )]
    pub escrow_pda_authority: AccountInfo<'info>,

    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn close<'info>(info: AccountInfo<'info>, sol_destination: AccountInfo<'info>) -> Result<()> {
    // Transfer tokens from the account to the sol_destination.
    let dest_starting_lamports = sol_destination.lamports();
    **sol_destination.lamports.borrow_mut() =
        dest_starting_lamports.checked_add(info.lamports()).unwrap();
    **info.lamports.borrow_mut() = 0;

    info.assign(&system_program::ID);
    // info.realloc(0, false).map_err(Into::into)
    realloc(&info, 0, false).map_err(Into::into)

    // Ok(())
}

pub fn realloc<'info>(selfX: &AccountInfo<'info>, new_len: usize, zero_init: bool) -> Result<()> {
        let mut data = selfX.try_borrow_mut_data()?;
        let old_len = data.len();

        // Return early if length hasn't changed
        if new_len == old_len {
            return Ok(());
        }

        // Return early if the length increase from the original serialized data
        // length is too large and would result in an out of bounds allocation.
        let original_data_len = unsafe { selfX.original_data_len() };
        if new_len.saturating_sub(original_data_len) > 10240 {
            return Err(ProgramError::InvalidRealloc.into());
        }

        // realloc
        unsafe {
            let data_ptr = data.as_mut_ptr();

            // // First set new length in the serialized data
            *(data_ptr.offset(-8) as *mut u64) = new_len as u64;

            // // Then recreate the local slice with the new length
            *data = from_raw_parts_mut(data_ptr, new_len)
        }

        if zero_init {
            let len_increase = new_len.saturating_sub(old_len);
            if len_increase > 0 {
                sol_memset(&mut data[old_len..], 0, len_increase);
            }
        }

        Ok(())
    }
