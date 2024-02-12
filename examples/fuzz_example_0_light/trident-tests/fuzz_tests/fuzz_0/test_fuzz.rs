use fuzz_example0::entry;
use fuzz_example0::ID as PROGRAM_ID;
use fuzz_instructions::fuzz_example0_fuzz_instructions::{FuzzInstruction, Initialize};
use trident_client::{fuzz_trident, fuzzing::*};
mod accounts_snapshots;
mod fuzz_instructions;

struct MyFuzzData;

impl FuzzDataBuilder<FuzzInstruction> for MyFuzzData {
    fn pre_ixs(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Vec<FuzzInstruction>> {
        let init = FuzzInstruction::Initialize(Initialize::arbitrary(u)?);
        Ok(vec![init])
    }
}

fn main() {
    loop {
        fuzz_trident!(fuzz_ix: FuzzInstruction, |fuzz_data: MyFuzzData| {
            let mut client =
                LighClient::new(entry,PROGRAM_ID)
                    .unwrap();
            let _ = fuzz_data.run_with_runtime(PROGRAM_ID, &mut client);
        });
    }
}
