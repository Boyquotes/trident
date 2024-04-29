use fuzz_example3::entry;
use fuzz_example3::ID as PROGRAM_ID;

use trident_client::fuzzing::*;
use trident_client::solana_sdk::native_token::LAMPORTS_PER_SOL;

fn main() {
    let mut client = LighClient::new(entry, PROGRAM_ID).unwrap();

    let recipient = Keypair::new();
    let data = fuzz_example3::instruction::InitVesting {
        recipient: recipient.pubkey(),
        amount: 1,
        start_at: 0,
        end_at: 10,
        interval: 2,
    };

    let sender = client.set_account(100 * LAMPORTS_PER_SOL);
    let mint = client.set_mint_account(9, &sender.pubkey(), None);
    let sender_token_account =
        client.set_token_account(mint, sender.pubkey(), 1000, None, None, 0, None);

    let recipient = client.set_account(100 * LAMPORTS_PER_SOL);

    let escrow = client
        .set_pda_account(&[recipient.pubkey().as_ref(), b"ESCROW_SEED"], &PROGRAM_ID)
        .unwrap()
        .pubkey();

    let recipient_token_account_correct =
        dbg!(anchor_spl::associated_token::get_associated_token_address(&recipient.pubkey(), &mint));

    let recipient_token_account = dbg!(Pubkey::find_program_address(
        &[
            recipient.pubkey().as_ref(),
            anchor_spl::token::ID.as_ref(),
            mint.as_ref(),
        ],
        &anchor_spl::associated_token::ID,
    )
    .0);

    let acc_meta = fuzz_example3::accounts::InitVesting {
        sender: sender.pubkey(),
        sender_token_account,
        escrow,
        recipient: recipient.pubkey(),
        recipient_token_account,
        mint,
        token_program: anchor_spl::token::ID,
        associated_token_program: anchor_spl::associated_token::ID,
        system_program: SYSTEM_PROGRAM_ID,
    }
    .to_account_metas(None);

    let ixx = Instruction {
        program_id: PROGRAM_ID,
        accounts: acc_meta,
        data: data.data(),
    };
    let ix_res = client.process_instruction(ixx);
    match ix_res {
        Ok(_) => {
            println!("IX SUCCESSFULL !!!");
            let rec = client.get_account(&recipient_token_account);
            dbg!(rec);
        }
        Err(_) => println!("IX FAILED !!!"),
    }
    // let mut client =
    //     ProgramTestClientBlocking::new("fuzz_example3", PROGRAM_ID, processor!(entry)).unwrap();
}
