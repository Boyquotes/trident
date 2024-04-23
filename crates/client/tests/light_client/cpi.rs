use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_sdk::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::{ProgramResult, MAX_PERMITTED_DATA_INCREASE},
    instruction::{get_stack_height, AccountMeta, Instruction},
    msg,
    program::invoke,
    pubkey::Pubkey,
    rent::Rent,
    signature::Signer,
    signer::keypair::Keypair,
    system_instruction, system_program,
    sysvar::Sysvar,
    transaction::Transaction,
};

use trident_client::fuzzing::{light_client::*, FuzzClient};

// Process instruction to invoke into another program
// We pass this specific number of accounts in order to test for a reported error.
fn invoker_process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _input: &[u8],
) -> ProgramResult {
    // if we can call `msg!` successfully, then InvokeContext exists as required
    msg!("Processing invoker instruction before CPI");
    let account_info_iter = &mut accounts.iter();
    let invoked_program_info = next_account_info(account_info_iter)?;
    invoke(
        &Instruction::new_with_bincode(
            *invoked_program_info.key,
            &[0],
            vec![AccountMeta::new_readonly(*invoked_program_info.key, false)],
        ),
        &[invoked_program_info.clone()],
    )?;
    msg!("Processing invoker instruction after CPI");
    Ok(())
}

// Process instruction to invoke into another program with duplicates
fn invoker_dupes_process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _input: &[u8],
) -> ProgramResult {
    // if we can call `msg!` successfully, then InvokeContext exists as required
    msg!("Processing invoker instruction before CPI");
    let account_info_iter = &mut accounts.iter();
    let invoked_program_info = next_account_info(account_info_iter)?;
    invoke(
        &Instruction::new_with_bincode(
            *invoked_program_info.key,
            &[0],
            vec![
                AccountMeta::new_readonly(*invoked_program_info.key, false),
                AccountMeta::new_readonly(*invoked_program_info.key, false),
                AccountMeta::new_readonly(*invoked_program_info.key, false),
                AccountMeta::new_readonly(*invoked_program_info.key, false),
            ],
        ),
        &[
            invoked_program_info.clone(),
            invoked_program_info.clone(),
            invoked_program_info.clone(),
            invoked_program_info.clone(),
            invoked_program_info.clone(),
        ],
    )?;
    msg!("Processing invoker instruction after CPI");
    Ok(())
}

// Process instruction to be invoked by another program
#[allow(clippy::unnecessary_wraps)]
fn invoked_process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _input: &[u8],
) -> ProgramResult {
    // if we can call `msg!` successfully, then InvokeContext exists as required
    msg!("Processing invoked instruction");
    Ok(())
}

// Process instruction to invoke into system program to create an account
fn invoke_create_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _input: &[u8],
) -> ProgramResult {
    msg!("Processing instruction before system program CPI instruction");
    let account_info_iter = &mut accounts.iter();
    let payer_info = next_account_info(account_info_iter)?;
    let create_account_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let rent = Rent::get()?;
    let minimum_balance = rent.minimum_balance(MAX_PERMITTED_DATA_INCREASE);
    invoke(
        &system_instruction::create_account(
            payer_info.key,
            create_account_info.key,
            minimum_balance,
            MAX_PERMITTED_DATA_INCREASE as u64,
            program_id,
        ),
        &[
            payer_info.clone(),
            create_account_info.clone(),
            system_program_info.clone(),
        ],
    )?;
    msg!("Processing instruction after system program CPI");
    Ok(())
}

#[tokio::test]
async fn cpi() {
    let invoker_program_id = Pubkey::new_unique();
    let mut client = LightClient::new(invoker_process_instruction, invoker_program_id).unwrap();
    let invoked_program_id = Pubkey::new_unique();
    client.add_program(invoked_program_id, invoked_process_instruction);

    client.init();
    let ix = Instruction::new_with_bincode(
        invoker_program_id,
        &[0],
        vec![AccountMeta::new_readonly(invoked_program_id, false)],
    );

    client.process_instruction(ix).unwrap();
}

#[tokio::test]
async fn cpi_dupes() {
    let invoker_program_id = Pubkey::new_unique();
    let mut client =
        LightClient::new(invoker_dupes_process_instruction, invoker_program_id).unwrap();
    let invoked_program_id = Pubkey::new_unique();
    client.add_program(invoked_program_id, invoked_process_instruction);

    client.init();
    let ix = Instruction::new_with_bincode(
        invoker_program_id,
        &[0],
        vec![
            AccountMeta::new_readonly(invoked_program_id, false),
            AccountMeta::new_readonly(invoked_program_id, false),
            AccountMeta::new_readonly(invoked_program_id, false),
            AccountMeta::new_readonly(invoked_program_id, false),
        ],
    );

    client.process_instruction(ix).unwrap();
}

#[tokio::test]
async fn cpi_create_account() {
    let create_account_program_id = Pubkey::new_unique();
    let mut client = LightClient::new(invoke_create_account, create_account_program_id).unwrap();

    let create_account_keypair = Keypair::new();
    client.init();

    let payer = client.set_account(LAMPORTS_PER_SOL * 10);

    let ix = Instruction::new_with_bincode(
        create_account_program_id,
        &[0],
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(create_account_keypair.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    );

    client.process_instruction(ix).unwrap();
}

// Process instruction to invoke into another program
fn invoker_stack_height(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _input: &[u8],
) -> ProgramResult {
    // if we can call `msg!` successfully, then InvokeContext exists as required
    msg!("Processing invoker instruction before CPI");
    let stack_height = get_stack_height();
    assert_eq!(stack_height, 1);
    let account_info_iter = &mut accounts.iter();
    let invoked_program_info = next_account_info(account_info_iter)?;
    invoke(
        &Instruction::new_with_bytes(*invoked_program_info.key, &[], vec![]),
        &[invoked_program_info.clone()],
    )?;
    msg!("Processing invoker instruction after CPI");
    Ok(())
}

// Process instruction to be invoked by another program
#[allow(clippy::unnecessary_wraps)]
fn invoked_stack_height(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _input: &[u8],
) -> ProgramResult {
    let stack_height = get_stack_height();
    assert_eq!(stack_height, 2);
    Ok(())
}

#[tokio::test]
async fn stack_height() {
    let invoker_stack_height_program_id = Pubkey::new_unique();
    let invoked_stack_height_program_id = Pubkey::new_unique();
    let mut client =
        LightClient::new(invoker_stack_height, invoker_stack_height_program_id).unwrap();
    client.add_program(invoked_stack_height_program_id, invoked_stack_height);

    client.init();
    let ix = Instruction::new_with_bytes(
        invoker_stack_height_program_id,
        &[],
        vec![AccountMeta::new_readonly(
            invoked_stack_height_program_id,
            false,
        )],
    );

    client.process_instruction(ix).unwrap();
}
