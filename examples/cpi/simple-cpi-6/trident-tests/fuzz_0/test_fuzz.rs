use trident_fuzz::fuzzing::*;
mod fuzz_instructions;
use callee::entry as entry_callee;
use caller::entry as entry_caller;
use fuzz_instructions::FuzzInstruction;
use fuzz_instructions::*;

struct InstructionsSequence;
/// Define instruction sequences for invocation.
/// `pre` runs at the start, `middle` in the middle, and `post` at the end.
/// For example, to call `InitializeFn`, `UpdateFn` and then `WithdrawFn` during
/// each fuzzing iteration:
/// ```
/// impl FuzzDataBuilder<FuzzInstruction> for InstructionsSequence {
///     pre_sequence!(InitializeFn,UpdateFn);
///     middle_sequence!(WithdrawFn);
///}
/// ```
/// For more details, see: https://ackee.xyz/trident/docs/latest/features/instructions-sequences/#instructions-sequences
impl FuzzDataBuilder<FuzzInstruction> for InstructionsSequence {
    pre_sequence!(InitializeCaller);
    middle_sequence!();
    post_sequence!();
}
fn main() {
    let program_callee = ProgramEntrypoint::new(
        pubkey!("HJR1TK8bgrUWzysdpS1pBGBYKF7zi1tU9cS4qj8BW8ZL"),
        None,
        processor!(entry_callee),
    );
    let program_caller = ProgramEntrypoint::new(
        pubkey!("FWtSodrkUnovFPnNRCxneP6VWh6JH6jtQZ4PHoP8Ejuz"),
        None,
        processor!(entry_caller),
    );
    let config = TridentConfig::new();
    let mut client = TridentSVM::new_client(&[program_callee, program_caller], &config);
    fuzz_trident ! (fuzz_ix : FuzzInstruction , | fuzz_data : InstructionsSequence , client : TridentSVM , config : TridentConfig |);
}
