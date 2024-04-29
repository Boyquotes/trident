use fuzz_example3::entry;
use fuzz_example3::ID as PROGRAM_ID;
use fuzz_instructions::fuzz_example3_fuzz_instructions::{FuzzInstruction, InitVesting};

use trident_client::{fuzz_trident, fuzzing::*};
mod accounts_snapshots;
mod fuzz_instructions;

struct MyFuzzData;

impl FuzzDataBuilder<FuzzInstruction> for MyFuzzData {
    fn pre_ixs(u: &mut arbitrary::Unstructured) -> arbitrary::Result<Vec<FuzzInstruction>> {
        let init_ix = FuzzInstruction::InitVesting(InitVesting::arbitrary(u)?);

        Ok(vec![init_ix])
    }
}

fn main() {
    loop {
        fuzz_trident!(fuzz_ix: FuzzInstruction, |fuzz_data: MyFuzzData| {
            let mut client =
                LighClient::new(entry,PROGRAM_ID)
                    .unwrap();
            // let mut client =
            //     ProgramTestClientBlocking::new("fuzz_example3", PROGRAM_ID, processor!(entry)).unwrap();
            let _ = fuzz_data.run_with_runtime(PROGRAM_ID, &mut client);
        });
    }
}
