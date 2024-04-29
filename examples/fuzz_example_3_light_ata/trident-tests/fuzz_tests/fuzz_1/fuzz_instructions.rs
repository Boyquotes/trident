pub mod fuzz_example3_fuzz_instructions {
    use crate::accounts_snapshots::*;
    use trident_client::{
        fuzzing::*,
        solana_sdk::{account::AccountSharedData, native_token::LAMPORTS_PER_SOL},
    };
    #[derive(Arbitrary, DisplayIx, FuzzTestExecutor, FuzzDeserialize)]
    pub enum FuzzInstruction {
        InitVesting(InitVesting),
        WithdrawUnlocked(WithdrawUnlocked),
    }
    #[derive(Arbitrary, Debug)]
    pub struct InitVesting {
        pub accounts: InitVestingAccounts,
        pub data: InitVestingData,
    }
    #[derive(Arbitrary, Debug)]
    pub struct InitVestingAccounts {
        pub sender: AccountId,
        pub sender_token_account: AccountId,
        pub escrow: AccountId,
        pub recipient: AccountId,
        pub recipient_token_account: AccountId,
        pub mint: AccountId,
        pub token_program: AccountId,
        pub associated_token_program: AccountId,
        pub system_program: AccountId,
    }
    #[derive(Arbitrary, Debug)]
    pub struct InitVestingData {
        pub recipient: AccountId,
        pub amount: u64,
        pub start_at: u64,
        pub end_at: u64,
        pub interval: u64,
    }
    #[derive(Arbitrary, Debug)]
    pub struct WithdrawUnlocked {
        pub accounts: WithdrawUnlockedAccounts,
        pub data: WithdrawUnlockedData,
    }
    #[derive(Arbitrary, Debug)]
    pub struct WithdrawUnlockedAccounts {
        pub recipient: AccountId,
        pub recipient_token_account: AccountId,
        pub escrow: AccountId,
        pub escrow_token_account: AccountId,
        pub escrow_pda_authority: AccountId,
        pub mint: AccountId,
        pub token_program: AccountId,
        pub system_program: AccountId,
    }
    #[derive(Arbitrary, Debug)]
    pub struct WithdrawUnlockedData {}
    impl<'info> IxOps<'info> for InitVesting {
        type IxData = fuzz_example3::instruction::InitVesting;
        type IxAccounts = FuzzAccounts;
        type IxSnapshot = InitVestingSnapshot<'info>;
        fn get_data(
            &self,
            client: &mut impl FuzzClient,
            fuzz_accounts: &mut FuzzAccounts,
        ) -> Result<Self::IxData, FuzzingError> {
            let recipient = fuzz_accounts.recipient.get_or_create_account(
                self.data.recipient,
                client,
                10 * LAMPORTS_PER_SOL,
            );
            let data = fuzz_example3::instruction::InitVesting {
                recipient: recipient.pubkey(),
                amount: self.data.amount,
                start_at: self.data.start_at,
                end_at: self.data.end_at,
                interval: self.data.interval,
            };
            Ok(data)
        }
        fn get_accounts(
            &self,
            client: &mut impl FuzzClient,
            fuzz_accounts: &mut FuzzAccounts,
        ) -> Result<(Vec<Keypair>, Vec<AccountMeta>), FuzzingError> {
            let sender =
                fuzz_accounts
                    .sender
                    .get_or_create_account(0, client, 1000 * LAMPORTS_PER_SOL);
            // INFO use constant Account ID, so we will not generate multiple mints,
            // and also we can easily link to Withdraw
            let mint = fuzz_accounts
                .mint
                .get_or_create_account(0, client, 6, &sender.pubkey(), None)
                .unwrap();

            let sender_token_account = fuzz_accounts
                .sender_token_account
                .get_or_create_account(0, client, mint, sender.pubkey(), 10000, None, None, 0, None)
                .unwrap();

            let recipient =
                fuzz_accounts
                    .recipient
                    .get_or_create_account(0, client, 10 * LAMPORTS_PER_SOL);

            let escrow = fuzz_accounts
                .escrow
                .get_or_create_account(
                    0,
                    // client,
                    &[recipient.pubkey().as_ref(), b"ESCROW_SEED"],
                    &fuzz_example3::ID,
                )
                .unwrap();

            // let recipient_token_account = fuzz_accounts
            //     .recipient_token_account
            //     .storage()
            //     .entry(self.accounts.recipient_token_account)
            //     .or_insert_with(|| {
            //         let pda = anchor_spl::associated_token::get_associated_token_address(
            //             &sender.pubkey(),
            //             &mint,
            //         );
            //         let recipient = recipient.pubkey().to_bytes();
            //         let mint = mint.to_bytes();
            //         let program = anchor_spl::token::ID.to_bytes();
            //         let seeds = &[recipient.as_ref(), program.as_ref(), mint.as_ref()];
            //         // let pda = Pubkey::find_program_address(seeds, &fuzz_example3::id()).0;
            //         let account = AccountSharedData::new_data_with_space::<[u8; 0]>(
            //             0,
            //             &[],
            //             0,
            //             &SYSTEM_PROGRAM_ID,
            //         )
            //         .unwrap();
            //         client.set_account_custom(&pda, &account);
            //         let vec_of_seeds: Vec<Vec<u8>> =
            //             seeds.iter().map(|&seed| seed.to_vec()).collect();
            //         PdaStore {
            //             pubkey: pda,
            //             seeds: vec_of_seeds,
            //         }
            //     })
            //     .pubkey();

            // let recipient_token_account = fuzz_accounts
            //     .recipient_token_account
            //     .storage()
            //     .entry(self.accounts.recipient_token_account)
            //     .or_insert_with(|| {
            //         let pda = anchor_spl::associated_token::get_associated_token_address(
            //             &recipient.pubkey(),
            //             &mint,
            //         );
            //         let recipient = recipient.pubkey().to_bytes();
            //         let mint = mint.to_bytes();
            //         let program = anchor_spl::token::ID.to_bytes();
            //         let seeds = &[recipient.as_ref(), program.as_ref(), mint.as_ref()];
            //         // let pda = Pubkey::find_program_address(seeds, &fuzz_example3::id()).0;
            //         let account = AccountSharedData::new_data_with_space::<[u8; 0]>(
            //             0,
            //             &[],
            //             0,
            //             &SYSTEM_PROGRAM_ID,
            //         )
            //         .unwrap();
            //         client.set_account_custom(&pda, &account);
            //         let vec_of_seeds: Vec<Vec<u8>> =
            //             seeds.iter().map(|&seed| seed.to_vec()).collect();
            //         PdaStore {
            //             pubkey: pda,
            //             seeds: vec_of_seeds,
            //         }
            //     })
            //     .pubkey();

            let recipient_token_account = fuzz_accounts
                .recipient_token_account
                .get_or_create_account(
                    0,
                    &[
                        recipient.pubkey().as_ref(),
                        anchor_spl::token::ID.as_ref(),
                        mint.as_ref(),
                    ],
                    &anchor_spl::associated_token::ID,
                )
                .unwrap();

            // let escrow_token_account = fuzz_accounts
            //     .escrow_token_account
            //     .get_or_create_account(
            //         self.accounts.escrow_token_account,
            //         client,
            //         mint,
            //         sender.pubkey(),
            //         0,
            //         None,
            //         None,
            //         0,
            //         None,
            //     )
            //     .unwrap();

            let acc_meta = fuzz_example3::accounts::InitVesting {
                sender: sender.pubkey(),
                sender_token_account,
                escrow: escrow.pubkey(),
                recipient: recipient.pubkey(),
                recipient_token_account: recipient_token_account.pubkey(),
                mint,
                token_program: anchor_spl::token::ID,
                associated_token_program: anchor_spl::associated_token::ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None);

            eprintln!("{:?}", acc_meta);

            Ok((vec![sender], acc_meta))
        }
    }
    impl<'info> IxOps<'info> for WithdrawUnlocked {
        type IxData = fuzz_example3::instruction::WithdrawUnlocked;
        type IxAccounts = FuzzAccounts;
        type IxSnapshot = WithdrawUnlockedSnapshot<'info>;
        fn get_data(
            &self,
            _client: &mut impl FuzzClient,
            _fuzz_accounts: &mut FuzzAccounts,
        ) -> Result<Self::IxData, FuzzingError> {
            let data = fuzz_example3::instruction::WithdrawUnlocked {};
            Ok(data)
        }
        fn get_accounts(
            &self,
            client: &mut impl FuzzClient,
            fuzz_accounts: &mut FuzzAccounts,
        ) -> Result<(Vec<Keypair>, Vec<AccountMeta>), FuzzingError> {
            let recipient = fuzz_accounts.recipient.get_or_create_account(
                self.accounts.recipient,
                client,
                1000 * LAMPORTS_PER_SOL,
            );

            // INFO use constant Account ID, so we will not generate multiple mints,
            // and also we can easily link to Initialize
            let mint = fuzz_accounts
                .mint
                .get_or_create_account(0, client, 6, &recipient.pubkey(), None)
                .unwrap();

            let recipient_token_account = fuzz_accounts
                .recipient_token_account
                .get_or_create_account(
                    self.accounts.recipient_token_account,
                    &[
                        recipient.pubkey().as_ref(),
                        mint.as_ref(),
                        anchor_spl::associated_token::ID.as_ref(),
                    ],
                    &anchor_spl::token::ID,
                )
                .unwrap();

            let escrow = fuzz_accounts
                .escrow
                .get_or_create_account(
                    self.accounts.escrow,
                    // client,
                    &[recipient.pubkey().as_ref(), b"ESCROW_SEED"],
                    &fuzz_example3::ID,
                )
                .unwrap();

            let escrow_pda_authority = fuzz_accounts
                .escrow_pda_authority
                .get_or_create_account(
                    self.accounts.escrow_pda_authority,
                    // client,
                    &[b"ESCROW_PDA_AUTHORITY"],
                    &fuzz_example3::ID,
                )
                .unwrap();

            let escrow_token_account = fuzz_accounts
                .escrow_token_account
                .storage()
                .entry(self.accounts.escrow_token_account)
                .or_insert_with(|| {
                    let pda = anchor_spl::associated_token::get_associated_token_address(
                        &escrow_pda_authority.pubkey(),
                        &mint,
                    );
                    let escrow = escrow.pubkey().to_bytes();
                    let mint = mint.to_bytes();
                    let program = anchor_spl::token::ID.to_bytes();
                    let seeds = &[escrow.as_ref(), program.as_ref(), mint.as_ref()];
                    let account = AccountSharedData::new_data_with_space::<[u8; 0]>(
                        0,
                        &[],
                        0,
                        &SYSTEM_PROGRAM_ID,
                    )
                    .unwrap();
                    client.set_account_custom(&pda, &account);
                    let vec_of_seeds: Vec<Vec<u8>> =
                        seeds.iter().map(|&seed| seed.to_vec()).collect();
                    PdaStore {
                        pubkey: pda,
                        seeds: vec_of_seeds,
                    }
                })
                .pubkey();
            // let escrow_token_account = fuzz_accounts
            //     .escrow_token_account
            //     .get_or_create_account(
            //         self.accounts.escrow_token_account,
            //         client,
            //         mint,
            //         escrow_pda_authority.pubkey(),
            //         u64::MAX,
            //         None,
            //         None,
            //         0,
            //         None,
            //     )
            //     .unwrap();

            let acc_meta = fuzz_example3::accounts::WithdrawUnlocked {
                recipient: recipient.pubkey(),
                recipient_token_account: recipient_token_account.pubkey(),
                escrow: escrow.pubkey(),
                escrow_token_account,
                escrow_pda_authority: escrow_pda_authority.pubkey(),
                mint,
                token_program: anchor_spl::token::ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None);
            Ok((vec![recipient], acc_meta))
        }
        fn check(
            &self,
            pre_ix: Self::IxSnapshot,
            post_ix: Self::IxSnapshot,
            _ix_data: Self::IxData,
        ) -> Result<(), FuzzingError> {
            if let Some(escrow) = pre_ix.escrow {
                let recipient = pre_ix.recipient;
                let recipient_token_account_pre = pre_ix.recipient_token_account;
                let recipient_token_account_post = post_ix.recipient_token_account;
                eprintln!(
                    "recipient_token_account_pre.amount = {}",
                    recipient_token_account_pre.amount
                );
                eprintln!(
                    "recipient_token_account_post.amount = {}",
                    recipient_token_account_post.amount
                );
                eprintln!("escrow.amount = {}", escrow.amount);
                if escrow.recipient == *recipient.key {
                    if recipient_token_account_pre.amount == recipient_token_account_post.amount {
                        // INFO Recipient was not able to withdraw
                        // return Err("Recipient was not able to withdraw any funds");
                        eprintln!("FuzzingError::BalanceMismatch");
                        return Err(FuzzingError::BalanceMismatch);
                    } else if recipient_token_account_pre.amount + escrow.amount
                        != recipient_token_account_post.amount
                    {
                        if recipient_token_account_pre.amount + escrow.amount
                            > recipient_token_account_post.amount
                        {
                            // eprintln!("recipient_token_account_pre.amount = {}", recipient_token_account_pre.amount);
                            // eprintln!("recipient_token_account_post.amount = {}", recipient_token_account_post.amount);
                            // INFO The recipient was able to withdraw,
                            // but not as much as was initially intended.
                            // return Err("Recipient withdrew LESS");
                            eprintln!("FuzzingError::Custom(15)");
                            return Err(FuzzingError::Custom(15));
                        } else {
                            // INFO The recipient was able to withdraw,
                            // but more as was initially intended.
                            // This option is possible because the program uses one token accout with corresponding mint
                            // across multiple Escrow Transactions, this means that we can actually withdraw more
                            // if prior to Withdraw call, was sufficient amount transferred to the escrow token account.
                            // (e.g. due to prior Initialization of different Escrow Transactions)
                            // For testing purposes inside debug use eprintln!()
                            // return Err("Recipient withdrew MORE");
                            eprintln!("FuzzingError::Custom(2)");
                            return Err(FuzzingError::Custom(2));
                        }
                    }
                }
            }
            eprintln!("WITHDRAW CHECK PASSED...");
            Ok(())
        }
    }
    #[doc = r" Use AccountsStorage<T> where T can be one of:"]
    #[doc = r" Keypair, PdaStore, TokenStore, MintStore, ProgramStore"]
    #[derive(Default)]
    pub struct FuzzAccounts {
        _token_program: AccountsStorage<ProgramStore>,
        sender_token_account: AccountsStorage<TokenStore>,
        escrow_token_account: AccountsStorage<PdaStore>,
        escrow_pda_authority: AccountsStorage<PdaStore>,
        sender: AccountsStorage<Keypair>,
        _system_program: AccountsStorage<ProgramStore>,
        recipient_token_account: AccountsStorage<PdaStore>,
        recipient: AccountsStorage<Keypair>,
        mint: AccountsStorage<MintStore>,
        escrow: AccountsStorage<PdaStore>,
    }
}
