use serde::{Deserialize, Serialize};
use solana_program::{
    clock::UnixTimestamp, entrypoint::ProgramResult, slot_history::Slot, stake_history::Epoch,
};
use solana_sdk::{
    account_info::AccountInfo, instruction::Instruction, pubkey::Pubkey,
    system_instruction::SystemInstruction,
};
use solana_sdk_macro::CloneZeroed;

#[allow(deprecated)]
use solana_sdk::program_stubs;

struct TestSyscallStubs {
    pub caller: Pubkey,
}

impl program_stubs::SyscallStubs for TestSyscallStubs {
    fn sol_log(&self, message: &str) {
        println!("{message}"); // FIXME maybe eprintln?
    }
    fn sol_get_rent_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }
    fn sol_log_compute_units(&self) {
        self.sol_log("SyscallStubs: sol_log_compute_units() not available");
    }
    fn sol_invoke_signed(
        &self,
        instruction: &Instruction,
        account_infos: &[AccountInfo],
        signers_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        if instruction.program_id == solana_program::system_program::ID {
            // let entry = solana_system_program::system_processor::Entrypoint::rust();
            // let ctx = InvokeContext{};
            let instruction =
                solana_sdk::program_utils::limited_deserialize(&instruction.data).unwrap();
            // TODO It may be correct to implement this as
            // https://github.com/solana-labs/solana/blob/master/programs/system/src/system_processor.rs
            match instruction {
                SystemInstruction::CreateAccount {
                    lamports,
                    space: _,
                    owner,
                } => {
                    self.sol_log("Processing CreateAccount");
                    subtract_lamports(&account_infos[0], lamports);
                    add_lamports(&account_infos[1], lamports);
                    assign(&account_infos[1], &owner);
                    Ok(())
                }
                SystemInstruction::CreateAccountWithSeed {
                    base: _,
                    seed: _,
                    lamports: _,
                    space: _,
                    owner: _,
                } => {
                    self.sol_log("Processing CreateAccountWithSeed");
                    Ok(())
                }
                SystemInstruction::Assign { owner } => {
                    // for now we will implement this manually but check if the logic cannot be actually
                    // reused
                    // https://github.com/solana-labs/solana/blob/e4064023bf7936ced97b0d4de22137742324983d/programs/system/src/system_processor.rs#L300
                    self.sol_log("Processing Assign");
                    assign(&account_infos[0], &owner);
                    Ok(())
                }
                SystemInstruction::Transfer { lamports } => {
                    self.sol_log("Processing Transfer");
                    subtract_lamports(&account_infos[0], lamports);
                    add_lamports(&account_infos[1], lamports);
                    Ok(())
                }
                SystemInstruction::TransferWithSeed {
                    lamports: _,
                    from_seed: _,
                    from_owner: _,
                } => {
                    self.sol_log("Processing TransferWithSeed");
                    Ok(())
                }
                SystemInstruction::AdvanceNonceAccount => {
                    self.sol_log("Processing AdvanceNonceAccount");
                    Ok(())
                }
                SystemInstruction::WithdrawNonceAccount(_lamports) => {
                    self.sol_log("Processing WithdrawNonceAccount");
                    Ok(())
                }
                SystemInstruction::InitializeNonceAccount(_authorized) => {
                    self.sol_log("Processing InitializeNonceAccount");
                    Ok(())
                }
                SystemInstruction::AuthorizeNonceAccount(_nonce_authority) => {
                    self.sol_log("Processing AuthorizeNonceAccount");
                    Ok(())
                }
                SystemInstruction::UpgradeNonceAccount => {
                    self.sol_log("Processing UpgradeNonceAccount");
                    Ok(())
                }
                SystemInstruction::Allocate { space: _ } => {
                    self.sol_log("Processing Allocate");
                    Ok(())
                }
                SystemInstruction::AllocateWithSeed {
                    base: _,
                    seed: _,
                    space: _,
                    owner: _,
                } => {
                    self.sol_log("Processing AllocateWithSeed");
                    Ok(())
                }
                SystemInstruction::AssignWithSeed {
                    base: _,
                    seed: _,
                    owner: _,
                } => {
                    self.sol_log("Processing AssignWithSeed");
                    Ok(())
                }
            }
        } else if instruction.program_id == spl_token::ID {
            let signers = signers_seeds
                .iter()
                .map(|seeds| Pubkey::create_program_address(seeds, &self.caller).unwrap())
                .collect::<Vec<_>>();

            let mut new_account_infos = vec![];
            for meta in instruction.accounts.iter() {
                for account_info in account_infos.iter() {
                    if meta.pubkey == *account_info.key {
                        let mut new_account_info = account_info.clone();
                        new_account_info.is_signer = true;
                        for signer in signers.iter() {
                            if *account_info.key == *signer {
                                new_account_info.is_signer = true;
                            }
                        }
                        new_account_infos.push(new_account_info);
                    }
                }
            }
            spl_token::processor::Processor::process(
                &instruction.program_id,
                &new_account_infos,
                &instruction.data,
            )
        } else {
            let message = format!(
                "SyscallStubs: sol_invoke_signed() for {} not available",
                instruction.program_id
            );
            self.sol_log(&message);
            Ok(())
        }
    }
    fn sol_get_clock_sysvar(&self, var_addr: *mut u8) -> u64 {
        let now = Clock::now();
        unsafe {
            *(var_addr as *mut _ as *mut Clock) = Clock::clone(&now);
            0
        }
    }

    // fn sol_get_stack_height(&self) -> u64 {
    //     1
    // }
}

pub fn subtract_lamports(from: &AccountInfo, lamports: u64) {
    let from_lamports = from.lamports();
    match from_lamports.checked_sub(lamports) {
        Some(new_balance) => {
            let mut mutable_lamports = from
                .try_borrow_mut_lamports()
                .expect("From: cannot borrow mutable lamports");
            **mutable_lamports = new_balance;
        }
        None => {
            panic!("From: not enough lamports")
        }
    }
}

pub fn add_lamports(to: &AccountInfo, lamports: u64) {
    let to_lamports = to.lamports();
    match to_lamports.checked_add(lamports) {
        Some(new_balance) => {
            let mut mutable_lamports = to
                .try_borrow_mut_lamports()
                .expect("To: cannot borrow mutable lamports");
            **mutable_lamports = new_balance;
        }
        None => {
            panic!("To: lamports addition overflow")
        }
    }
}

pub fn assign(to: &AccountInfo, new_owner: &Pubkey) {
    to.assign(new_owner);
}

pub fn test_syscall_stubs(program_id: Pubkey) {
    use std::sync::Once;
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        program_stubs::set_syscall_stubs(Box::new(TestSyscallStubs { caller: program_id }));
    });
}

#[repr(C)]
#[derive(Serialize, Deserialize, Debug, CloneZeroed, Default, PartialEq, Eq)]
pub struct Clock {
    /// The current `Slot`.
    pub slot: Slot,
    /// The timestamp of the first `Slot` in this `Epoch`.
    pub epoch_start_timestamp: UnixTimestamp,
    /// The current `Epoch`.
    pub epoch: Epoch,
    /// The future `Epoch` for which the leader schedule has
    /// most recently been calculated.
    pub leader_schedule_epoch: Epoch,
    /// The approximate real world time of the current slot.
    ///
    /// This value was originally computed from genesis creation time and
    /// network time in slots, incurring a lot of drift. Following activation of
    /// the [`timestamp_correction` and `timestamp_bounding`][tsc] features it
    /// is calculated using a [validator timestamp oracle][oracle].
    ///
    /// [tsc]: https://docs.solana.com/implemented-proposals/bank-timestamp-correction
    /// [oracle]: https://docs.solana.com/implemented-proposals/validator-timestamp-oracle
    pub unix_timestamp: UnixTimestamp,
}

impl Clock {
    pub fn now() -> Self {
        let unix_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Clock {
            slot: 0,
            epoch_start_timestamp: 0,
            epoch: 0,
            leader_schedule_epoch: 0,
            unix_timestamp: i64::try_from(unix_timestamp).unwrap_or_default(),
        }
    }
}
