use serde::{Deserialize, Serialize};
use solana_program::{
    clock::UnixTimestamp, entrypoint::ProgramResult, slot_history::Slot, stake_history::Epoch,
};
use solana_sdk::{
    account_info::AccountInfo, instruction::Instruction, program_stubs,
    system_instruction::SystemInstruction,
};
use solana_sdk_macro::CloneZeroed;
struct TestSyscallStubs {}

impl program_stubs::SyscallStubs for TestSyscallStubs {
    fn sol_log(&self, message: &str) {
        println!("{message}");
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
            let instruction =
                solana_sdk::program_utils::limited_deserialize(&instruction.data).unwrap();
            // try to think about https://github.com/solana-labs/solana/blob/e4064023bf7936ced97b0d4de22137742324983d/sdk/program/src/program.rs#L289
            // if the refcell does not allow to actually update the memory
            match instruction {
                SystemInstruction::CreateAccount {
                    lamports,
                    space,
                    owner,
                } => {
                    self.sol_log("Processing CreateAccount");
                    let from_lamports = account_infos[0].lamports();
                    let to_lamports = account_infos[1].lamports();
                    // anchor_lang::system_program::CreateAccount
                    // anchor_lang::system_program::create_account
                    match from_lamports.checked_sub(lamports) {
                        Some(new_balance) => {
                            let mut mutable_lamports = account_infos[0]
                                .try_borrow_mut_lamports()
                                .expect("From: cannot borrow mutable lamports");
                            **mutable_lamports = new_balance;
                        }
                        None => {
                            panic!("From: not enough lamports")
                        }
                    }
                    match to_lamports.checked_add(lamports) {
                        Some(new_balance) => {
                            let mut mutable_lamports = account_infos[1]
                                .try_borrow_mut_lamports()
                                .expect("To: cannot borrow mutable lamports");
                            **mutable_lamports = new_balance;
                        }
                        None => {
                            panic!("To: lamports addition overflow")
                        }
                    }
                    account_infos[1].assign(&owner);
                }
                SystemInstruction::CreateAccountWithSeed {
                    base: _,
                    seed: _,
                    lamports: _,
                    space: _,
                    owner: _,
                } => self.sol_log("Processing CreateAccountWithSeed"),
                SystemInstruction::Assign { owner } => {
                    // for now we will implement this manually but check if the logic cannot be actually
                    // reused
                    // https://github.com/solana-labs/solana/blob/e4064023bf7936ced97b0d4de22137742324983d/programs/system/src/system_processor.rs#L300
                    self.sol_log("Processing Assign");
                    account_infos[0].assign(&owner);
                    // let account_data =
                    // let account = &mut account_infos[0];
                    // account.owner = &owner.clone();
                }
                SystemInstruction::Transfer { lamports: _ } => self.sol_log("Processing Transfer"),
                SystemInstruction::TransferWithSeed {
                    lamports: _,
                    from_seed: _,
                    from_owner: _,
                } => self.sol_log("Processing TransferWithSeed"),
                SystemInstruction::AdvanceNonceAccount => {
                    self.sol_log("Processing AdvanceNonceAccount")
                }
                SystemInstruction::WithdrawNonceAccount(_lamports) => {
                    self.sol_log("Processing WithdrawNonceAccount")
                }
                SystemInstruction::InitializeNonceAccount(_authorized) => {
                    self.sol_log("Processing InitializeNonceAccount")
                }
                SystemInstruction::AuthorizeNonceAccount(_nonce_authority) => {
                    self.sol_log("Processing AuthorizeNonceAccount")
                }
                SystemInstruction::UpgradeNonceAccount => {
                    self.sol_log("Processing UpgradeNonceAccount")
                }
                SystemInstruction::Allocate { space: _ } => {
                    self.sol_log("Processing Allocate");
                }
                SystemInstruction::AllocateWithSeed {
                    base: _,
                    seed: _,
                    space: _,
                    owner: _,
                } => self.sol_log("Processing AllocateWithSeed"),
                SystemInstruction::AssignWithSeed {
                    base: _,
                    seed: _,
                    owner: _,
                } => self.sol_log("Processing AssignWithSeed"),
            }
        } else {
            if instruction.program_id == spl_token::ID {
                return spl_token::processor::Processor::process(
                    &instruction.program_id,
                    account_infos,
                    &instruction.data,
                );
            }
            let message = format!(
                "SyscallStubs: sol_invoke_signed() for {} not available",
                instruction.program_id
            );
            self.sol_log(&message);
        }
        Ok(())
    }
    fn sol_get_clock_sysvar(&self, var_addr: *mut u8) -> u64 {
        let now = Clock::now();
        unsafe {
            *(var_addr as *mut _ as *mut Clock) = Clock::clone(&now);
            0
        }
    }
}

pub fn test_syscall_stubs() {
    use std::sync::Once;
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        program_stubs::set_syscall_stubs(Box::new(TestSyscallStubs {}));
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
