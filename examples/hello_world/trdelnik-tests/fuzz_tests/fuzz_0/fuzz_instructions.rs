pub mod hello_world_fuzz_instructions {
    use crate::accounts_snapshots::*;
    use trdelnik_client::{fuzzing::*, solana_sdk::native_token::LAMPORTS_PER_SOL};
    #[derive(Arbitrary, DisplayIx, FuzzTestExecutorLight, FuzzDeserialize)]
    pub enum FuzzInstruction {
        Initialize(Initialize),
        UpdateEscrow(UpdateEscrow),
    }
    #[derive(Arbitrary, Debug)]
    pub struct Initialize {
        pub accounts: InitializeAccounts,
        pub data: InitializeData,
    }
    #[derive(Arbitrary, Debug)]
    pub struct InitializeAccounts {
        pub signer: AccountId,
        pub escrow: AccountId,
        pub system_program: AccountId,
    }
    #[derive(Arbitrary, Debug)]
    pub struct InitializeData {
        pub index: u8,
    }
    #[derive(Arbitrary, Debug)]
    pub struct UpdateEscrow {
        pub accounts: UpdateEscrowAccounts,
        pub data: UpdateEscrowData,
    }
    #[derive(Arbitrary, Debug)]
    pub struct UpdateEscrowAccounts {
        pub signer: AccountId,
        pub escrow: AccountId,
    }
    #[derive(Arbitrary, Debug)]
    pub struct UpdateEscrowData {
        pub index: u8,
    }
    impl<'info> IxOps<'info> for Initialize {
        type IxData = hello_world::instruction::Initialize;
        type IxAccounts = FuzzAccounts;
        type IxSnapshot = InitializeSnapshot<'info>;
        fn get_data(
            &self,
            _client: &mut impl FuzzClient,
            _fuzz_accounts: &mut FuzzAccounts,
        ) -> Result<Self::IxData, FuzzingError> {
            let data = hello_world::instruction::Initialize {
                index: self.data.index,
            };
            Ok(data)
        }
        fn get_accounts(
            &self,
            client: &mut impl FuzzClient,
            fuzz_accounts: &mut FuzzAccounts,
        ) -> Result<(Vec<Keypair>, Vec<AccountMeta>), FuzzingError> {
            let signer =
                fuzz_accounts
                    .signer
                    .get_or_create_account(1, client, LAMPORTS_PER_SOL * 20);

            let escrow = fuzz_accounts
                .escrow
                .get_or_create_data_account(1, client, &[b"escrow_seed"], &crate::PROGRAM_ID, 8 + 1)
                .unwrap();

            let signers = vec![signer.clone()];
            let acc_meta = hello_world::accounts::Initialize {
                signer: signer.pubkey(),
                escrow: escrow.pubkey,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None);
            Ok((signers, acc_meta))
        }
        fn check(
            &self,
            _pre_ix: Self::IxSnapshot,
            post_ix: Self::IxSnapshot,
            _ix_data: Self::IxData,
        ) -> Result<(), FuzzingError> {
            if let Some(escrow) = post_ix.escrow {
                show_account!(escrow);
                let account_info = escrow.to_account_info();
                show_account!(account_info);

                if *account_info.owner != crate::PROGRAM_ID {
                    return Err(FuzzingError::Custom(5));
                }
                if account_info.lamports() == 0 {
                    return Err(FuzzingError::Custom(7));
                }
            }
            Ok(())
        }
    }
    impl<'info> IxOps<'info> for UpdateEscrow {
        type IxData = hello_world::instruction::UpdateEscrow;
        type IxAccounts = FuzzAccounts;
        type IxSnapshot = UpdateEscrowSnapshot<'info>;
        fn get_data(
            &self,
            _client: &mut impl FuzzClient,
            _fuzz_accounts: &mut FuzzAccounts,
        ) -> Result<Self::IxData, FuzzingError> {
            let data = hello_world::instruction::UpdateEscrow {
                index: self.data.index,
            };
            Ok(data)
        }
        fn get_accounts(
            &self,
            client: &mut impl FuzzClient,
            fuzz_accounts: &mut FuzzAccounts,
        ) -> Result<(Vec<Keypair>, Vec<AccountMeta>), FuzzingError> {
            let signer =
                fuzz_accounts
                    .signer
                    .get_or_create_account(1, client, 5 * LAMPORTS_PER_SOL);

            let escrow = fuzz_accounts
                .escrow
                .get_or_create_data_account(1, client, &[b"escrow_seed"], &crate::PROGRAM_ID, 0)
                .unwrap();

            let signers = vec![signer.clone()];
            let acc_meta = hello_world::accounts::UpdateEscrow {
                signer: signer.pubkey(),
                escrow: escrow.pubkey(),
            }
            .to_account_metas(None);
            Ok((signers, acc_meta))
        }
        fn check(
            &self,
            _pre_ix: Self::IxSnapshot,
            post_ix: Self::IxSnapshot,
            _ix_data: Self::IxData,
        ) -> Result<(), FuzzingError> {
            if post_ix.escrow.index == 131 {
                return Err(FuzzingError::Custom(2));
            }
            Ok(())
        }
    }
    #[doc = r" Use AccountsStorage<T> where T can be one of:"]
    #[doc = r" Keypair, PdaStore, TokenStore, MintStore, ProgramStore"]
    #[derive(Default)]
    pub struct FuzzAccounts {
        escrow: AccountsStorage<PdaStore>,
        signer: AccountsStorage<Keypair>,
        // system_program: AccountsStorage<todo!()>,
    }
}
