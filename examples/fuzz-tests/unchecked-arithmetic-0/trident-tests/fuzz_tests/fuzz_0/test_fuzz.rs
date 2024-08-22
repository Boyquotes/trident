use fuzz_instructions::unchecked_arithmetic_0_fuzz_instructions::Initialize;
use fuzz_instructions::unchecked_arithmetic_0_fuzz_instructions::Update;
use trident_client::fuzzing::*;

mod fuzz_instructions;

use unchecked_arithmetic_0::entry as entry_unchecked_arithmetic_0;
use unchecked_arithmetic_0::ID as PROGRAM_ID_UNCHECKED_ARITHMETIC_0;

const PROGRAM_NAME_UNCHECKED_ARITHMETIC_0: &str = "unchecked_arithmetic_0";

use fuzz_instructions::unchecked_arithmetic_0_fuzz_instructions::FuzzInstruction as fuzz_instruction_unchecked_arithmetic_0;

pub type FuzzInstruction = fuzz_instruction_unchecked_arithmetic_0;

struct MyFuzzData;

impl FuzzDataBuilder<FuzzInstruction> for MyFuzzData {
    fn pre_ixs(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Vec<FuzzInstruction>> {
        let init = FuzzInstruction::Initialize(Initialize::arbitrary(u)?);
        Ok(vec![init])
    }
    fn ixs(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Vec<FuzzInstruction>> {
        let update = FuzzInstruction::Update(Update::arbitrary(u)?);
        Ok(vec![update])
    }
    fn post_ixs(_u: &mut arbitrary::Unstructured) -> arbitrary::Result<Vec<FuzzInstruction>> {
        Ok(vec![])
    }
}

fn fuzz_iteration<T: FuzzTestExecutor<U> + std::fmt::Display, U>(fuzz_data: FuzzData<T, U>) {
    let fuzzing_program_unchecked_arithmetic_0 = FuzzingProgram::new(
        PROGRAM_NAME_UNCHECKED_ARITHMETIC_0,
        &PROGRAM_ID_UNCHECKED_ARITHMETIC_0,
        processor!(convert_entry!(entry_unchecked_arithmetic_0)),
    );

    let mut client =
        ProgramTestClientBlocking::new(&[fuzzing_program_unchecked_arithmetic_0]).unwrap();

    let _ = fuzz_data.run_with_runtime(PROGRAM_ID_UNCHECKED_ARITHMETIC_0, &mut client);
}

fn main() {
    loop {
        fuzz_trident ! (fuzz_ix : FuzzInstruction , | fuzz_data : MyFuzzData | { fuzz_iteration (fuzz_data) ; });
    }
}
