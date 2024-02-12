use anchor_lang::prelude::*;

declare_id!("5WfCwQJCCNTbHwQsXKt1hYZpmbYLGidM3hehap8tk6gc");

#[program]
pub mod hello_world {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, index: u8) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        escrow.index = index;
        Ok(())
    }
    pub fn update_escrow(ctx: Context<UpdateEscrow>, index: u8) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        escrow.index = index;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        space = 8+1,
        seeds = [b"escrow_seed"],
        bump,
    )]
    pub escrow: Account<'info, Escrow>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateEscrow<'info> {
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds=[b"escrow_seed"],
        bump,
    )]
    pub escrow: Account<'info, Escrow>,
}

#[account]
#[derive(Debug)]
pub struct Escrow {
    pub index: u8,
}
