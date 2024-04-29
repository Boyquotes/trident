use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(FuzzTestExecutor)]
pub fn fuzz_test_executor(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = &input.ident;

    let display_impl = match &input.data {
        Data::Enum(enum_data) => {
            let display_match_arms = enum_data.variants.iter().map(|variant| {
                let variant_name = &variant.ident;
                quote! {
                    #enum_name::#variant_name (ix) => {
                        let (mut signers, metas) = ix.get_accounts(client, &mut accounts.borrow_mut())
                            .map_err(|e| e.with_origin(Origin::Instruction(self.to_context_string())))
                            .expect("Accounts calculation expect");

                        let mut snaphot = Snapshot::new(&metas, ix);
                        snaphot.capture_before(client).unwrap();

                        let data = ix.get_data(client, &mut accounts.borrow_mut())
                            .map_err(|e| e.with_origin(Origin::Instruction(self.to_context_string())))
                            .expect("Data calculation expect");

                        let ixx = Instruction {
                            program_id,
                            accounts: metas.clone(),
                            data: data.data(),
                        };

                        let mut transaction =
                            Transaction::new_with_payer(&[ixx], Some(&client.payer().pubkey()));

                        signers.push(client.payer().clone());
                        let sig: Vec<&Keypair> = signers.iter().collect();
                        transaction.sign(&sig, client.get_last_blockhash());

                        let duplicate_tx = if cfg!(allow_duplicate_txs) {
                            None
                        } else {
                            let message_hash = transaction.message().hash();
                            sent_txs.insert(message_hash, ())
                        };

                        match duplicate_tx {
                            Some(_) => eprintln!("\x1b[1;93mWarning\x1b[0m: Skipping duplicate instruction `{}`", self.to_context_string()),
                            None => {
                                let tx_result = client.process_transaction(transaction)
                                .map_err(|e| e.with_origin(Origin::Instruction(self.to_context_string())));

                                match tx_result {
                                    Ok(_) => {
                                        snaphot.capture_after(client).unwrap();
                                        let (acc_before, acc_after) = snaphot.get_snapshot()
                                            .map_err(|e| e.with_origin(Origin::Instruction(self.to_context_string())))
                                            .expect("Snapshot deserialization expect"); // we want to panic if we cannot unwrap to cause a crash

                                        if let Err(e) = ix.check(acc_before, acc_after, data).map_err(|e| e.with_origin(Origin::Instruction(self.to_context_string()))) {
                                            eprintln!(
                                                "\x1b[31mCRASH DETECTED!\x1b[0m Custom check after the {} instruction did not pass!",
                                                self.to_context_string());
                                            panic!("{}", e)
                                        }
                                    },
                                    Err(e) => {
                                        let mut raw_accounts = snaphot.get_raw_pre_ix_accounts();
                                        ix.tx_error_handler(e, data, &mut raw_accounts)?
                                    }
                                }
                            }
                        }
                    }
                }
            });

            quote! {
               impl FuzzTestExecutor<FuzzAccounts> for FuzzInstruction {
                   fn run_fuzzer(
                       &self,
                       program_id: Pubkey,
                       accounts: &RefCell<FuzzAccounts>,
                       client: &mut impl FuzzClient,
                       sent_txs: &mut HashMap<Hash, ()>,
                   ) -> core::result::Result<(), FuzzClientErrorWithOrigin> {
                           match self {
                               #(#display_match_arms)*
                           }
                           Ok(())
                   }
                }
            }
        }
        _ => panic!("FuzzTestExecutor can only be derived for enums"),
    };

    TokenStream::from(display_impl)
}

#[proc_macro_derive(FuzzTestExecutorLight)]
pub fn fuzz_test_executor_light(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = &input.ident;

    let display_impl = match &input.data {
        Data::Enum(enum_data) => {
            let display_match_arms = enum_data.variants.iter().map(|variant| {
                let variant_name = &variant.ident;
                quote! {
                    #enum_name::#variant_name (ix) => {
                    let (mut signers, metas) = ix.get_accounts(client, &mut accounts.borrow_mut())
                            .map_err(|e| e.with_origin(Origin::Instruction(self.to_context_string())))
                            .expect("Accounts calculation expect");


                    let mut snaphot = Snapshot::new(&metas, ix);
                    snaphot.capture_before(client).unwrap();


                    let data = ix.get_data(client, &mut accounts.borrow_mut())
                            .map_err(|e| e.with_origin(Origin::Instruction(self.to_context_string())))
                            .expect("Data calculation expect");

                    let ixx = Instruction {
                        program_id,
                        accounts: metas.clone(),
                        data: data.data(),
                    };


                    let ix_res = client.process_instruction(ixx);

                    if ix_res.is_ok(){
                        snaphot.capture_after(client).unwrap();
                            let (acc_before, acc_after) = snaphot.get_snapshot()
                                .map_err(|e| e.with_origin(Origin::Instruction(self.to_context_string())))
                                .expect("Snapshot deserialization expect"); // we want to panic if we cannot unwrap to cause a crash
                        if let Err(e) = ix.check(acc_before, acc_after, data).map_err(|e| e.with_origin(Origin::Instruction(self.to_context_string()))) {
                                eprintln!(
                                    "CRASH DETECTED! Custom check after the {} instruction did not pass!",
                                    self.to_context_string());
                                panic!("{}", e)
                            }

                    }

                    // snaphot.capture_after(client).unwrap();
                    // let (acc_before, acc_after) = snaphot.get_snapshot().unwrap(); // we want to panic if we cannot unwrap to cause a crash
                    // if let Err(e) = ix.check(acc_before, acc_after, data) {
                    //     eprintln!(
                    //         "Custom check after the {} instruction did not pass with the error message: {}",
                    //         self, e
                    //     );
                    //     eprintln!("Instruction data submitted to the instruction were:"); // TODO data does not implement Debug trait -> derive Debug trait on InitializeIx and automaticaly implement conversion from Initialize to InitializeIx
                    //     panic!("{}", e)
                    // }

                    // if res.is_err() {
                    //     return Ok(());
                    // }
                }
                }
            });

            quote! {
               impl FuzzTestExecutor<FuzzAccounts> for FuzzInstruction {
                   fn run_fuzzer(
                       &self,
                       program_id: Pubkey,
                       accounts: &RefCell<FuzzAccounts>,
                       client: &mut impl FuzzClient,
                       sent_txs: &mut HashMap<Hash, ()>,
                   ) -> core::result::Result<(), FuzzClientErrorWithOrigin> {
                           match self {
                               #(#display_match_arms)*
                           }
                           Ok(())
                   }
                }
            }
        }
        _ => panic!("FuzzTestExecutor can only be derived for enums"),
    };

    TokenStream::from(display_impl)
}
