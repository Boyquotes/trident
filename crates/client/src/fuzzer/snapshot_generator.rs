// To generate the snapshot data types, we need to first find all context struct within the program and parse theirs accounts.
// The parsing of individual Anchor accounts is done using Anchor syn parser:
// https://github.com/coral-xyz/anchor/blob/master/lang/syn/src/parser/accounts/mod.rs

use std::{error::Error, fs::File, io::Read};

use anchor_lang::anchor_syn::{AccountField, Ty};
use cargo_metadata::camino::Utf8PathBuf;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::parse::{Error as ParseError, Result as ParseResult};
use syn::spanned::Spanned;
use syn::{parse_quote, Attribute, Fields, GenericArgument, Item, ItemStruct, PathArguments};

use anchor_lang::anchor_syn::parser::accounts::parse_account_field;

pub fn generate_snapshots_code(code_path: Vec<(String, Utf8PathBuf)>) -> Result<String, String> {
    let code = code_path.iter().map(|(code, path)| {
        let mut mod_program = None::<syn::ItemMod>;
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| e.to_string())?;

        let parse_result = syn::parse_file(&content).map_err(|e| e.to_string())?;

        // locate the program module to extract instructions and corresponding Context structs.
        for item in parse_result.items.iter() {
            if let Item::Mod(module) = item {
                // Check if the module has the #[program] attribute
                if has_program_attribute(&module.attrs) {
                    mod_program = Some(module.clone())
                }
            }
        }

        let mod_program = mod_program.ok_or("module with program attribute not found")?;

        let (_, items) = mod_program
            .content
            .ok_or("the content of program module is missing")?;

        let ix_ctx_pairs = get_ix_ctx_pairs(&items)?;

        let (structs, impls) = get_snapshot_structs_and_impls(code, &ix_ctx_pairs)?;

        let use_statements = quote! {
            use trdelnik_client::anchor_lang::{prelude::*, self};
            use trdelnik_client::anchor_lang::solana_program::instruction::AccountMeta;
            use trdelnik_client::fuzzing::{get_account_infos_option, FuzzingError};
        }
        .into_token_stream();
        Ok(format!("{}{}{}", use_statements, structs, impls))
    });

    code.into_iter().collect()
}

/// Creates new snapshot structs with fields wrapped in Option<_> if approriate and the
/// respective implementations with snapshot deserialization methods
fn get_snapshot_structs_and_impls(
    code: &str,
    ix_ctx_pairs: &[(Ident, GenericArgument)],
) -> Result<(String, String), String> {
    let mut structs = String::new();
    let mut impls = String::new();
    let parse_result = syn::parse_file(code).map_err(|e| e.to_string())?;
    for pair in ix_ctx_pairs {
        let mut ty = None;
        if let GenericArgument::Type(syn::Type::Path(tp)) = &pair.1 {
            ty = tp.path.get_ident().cloned();
            // TODO add support for types with fully qualified path such as ix::Initialize
        }
        let ty = ty.ok_or(format!("malformed parameters of {} instruction", pair.0))?;

        // recursively find the context struct and create a new version with wrapped fields into Option
        if let Some(ctx) = find_ctx_struct(&parse_result.items, &ty) {
            let fields_parsed = if let Fields::Named(f) = ctx.fields.clone() {
                let field_deser: ParseResult<Vec<AccountField>> =
                    f.named.iter().map(parse_account_field).collect();
                field_deser
            } else {
                Err(ParseError::new(
                    ctx.fields.span(),
                    "Context struct parse errror.",
                ))
            }
            .map_err(|e| e.to_string())?;

            let wrapped_struct = wrap_fields_in_option(ctx, &fields_parsed).unwrap();
            let deser_code =
                deserialize_ctx_struct_anchor(ctx, &fields_parsed).map_err(|e| e.to_string())?;
            // let deser_code = deserialize_ctx_struct(ctx).unwrap();
            structs = format!("{}{}", structs, wrapped_struct.into_token_stream());
            impls = format!("{}{}", impls, deser_code.into_token_stream());
        } else {
            return Err(format!("The Context struct {} was not found", ty));
        }
    }

    Ok((structs, impls))
}

/// Iterates through items and finds functions with the Context<_> parameter. Returns pairs with the function name and the Context's inner type.
fn get_ix_ctx_pairs(items: &[Item]) -> Result<Vec<(Ident, GenericArgument)>, String> {
    let mut ix_ctx_pairs = Vec::new();
    for item in items {
        if let syn::Item::Fn(func) = item {
            let func_name = &func.sig.ident;
            let first_param_type = if let Some(param) = func.sig.inputs.iter().next() {
                let mut ty = None::<GenericArgument>;
                if let syn::FnArg::Typed(t) = param {
                    if let syn::Type::Path(tp) = *t.ty.clone() {
                        if let Some(seg) = tp.path.segments.iter().next() {
                            if let PathArguments::AngleBracketed(arg) = &seg.arguments {
                                ty = arg.args.first().cloned();
                            }
                        }
                    }
                }
                ty
            } else {
                None
            };

            let first_param_type = first_param_type.ok_or(format!(
                "The function {} does not have the Context parameter and is malformed.",
                func_name
            ))?;

            ix_ctx_pairs.push((func_name.clone(), first_param_type));
        }
    }
    Ok(ix_ctx_pairs)
}

/// Recursively find a struct with a given `name`
fn find_ctx_struct<'a>(items: &'a Vec<syn::Item>, name: &'a syn::Ident) -> Option<&'a ItemStruct> {
    for item in items {
        if let Item::Struct(struct_item) = item {
            if struct_item.ident == *name {
                return Some(struct_item);
            }
        }
    }

    // if the ctx struct is not found on the first level, recursively continue to search in submodules
    for item in items {
        if let Item::Mod(mod_item) = item {
            if let Some((_, items)) = &mod_item.content {
                let r = find_ctx_struct(items, name);
                if r.is_some() {
                    return r;
                }
            };
        }
    }

    None
}

/// Determines if an Account should be wrapped into the `Option` type.
/// The function returns true if the account has the init or close constraints set
/// and is not already wrapped into the `Option` type.
fn is_optional(parsed_field: &AccountField) -> bool {
    let is_optional = match parsed_field {
        AccountField::Field(field) => field.is_optional,
        AccountField::CompositeField(_) => false,
    };
    let constraints = match parsed_field {
        AccountField::Field(f) => &f.constraints,
        AccountField::CompositeField(f) => &f.constraints,
    };

    (constraints.init.is_some() || constraints.is_close()) && !is_optional
}

/// Determines if an Accout should be deserialized as optional.
/// The function returns true if the account has the init or close constraints set
/// or if it is explicitly optional (it was wrapped into the `Option` type already
/// in the definition of it's corresponding context structure).
fn deserialize_as_option(parsed_field: &AccountField) -> bool {
    let is_optional = match parsed_field {
        AccountField::Field(field) => field.is_optional,
        AccountField::CompositeField(_) => false,
    };
    let constraints = match parsed_field {
        AccountField::Field(f) => &f.constraints,
        AccountField::CompositeField(f) => &f.constraints,
    };

    constraints.init.is_some() || constraints.is_close() || is_optional
}

fn wrap_fields_in_option(
    orig_struct: &ItemStruct,
    parsed_fields: &[AccountField],
) -> Result<TokenStream, Box<dyn Error>> {
    let struct_name = format_ident!("{}Snapshot", orig_struct.ident);
    let wrapped_fields = match orig_struct.fields.clone() {
        Fields::Named(named) => {
            let field_wrappers =
                named
                    .named
                    .iter()
                    .zip(parsed_fields)
                    .map(|(field, parsed_field)| {
                        let field_name = &field.ident;
                        let field_type = &field.ty;
                        if is_optional(parsed_field) {
                            quote! {
                                pub #field_name: Option<#field_type>,
                            }
                        } else {
                            quote! {
                                pub #field_name: #field_type,
                            }
                        }
                    });

            quote! {
                { #(#field_wrappers)* }
            }
        }

        _ => return Err("Only structs with named fields are supported".into()),
    };

    // Generate the new struct with Option-wrapped fields
    let generated_struct: syn::ItemStruct = parse_quote! {
        pub struct #struct_name<'info> #wrapped_fields
    };

    Ok(generated_struct.to_token_stream())
}

/// Generates code to deserialize the snapshot structs.
fn deserialize_ctx_struct_anchor(
    snapshot_struct: &ItemStruct,
    parsed_fields: &[AccountField],
) -> Result<TokenStream, Box<dyn Error>> {
    let impl_name = format_ident!("{}Snapshot", snapshot_struct.ident);
    let names_deser_pairs: Result<Vec<(TokenStream, TokenStream)>, _> = parsed_fields
        .iter()
        .map(|parsed_f| match parsed_f {
            AccountField::Field(f) => {
                let field_name = f.ident.clone();
                let deser_tokens = match ty_to_tokens(&f.ty) {
                    Some((return_type, deser_method)) => deserialize_account_tokens(
                        &field_name,
                        deserialize_as_option(parsed_f),
                        return_type,
                        deser_method,
                    ),
                    None if matches!(&f.ty, Ty::UncheckedAccount) => {
                        acc_unchecked_tokens(&field_name)
                    }
                    None => acc_info_tokens(&field_name),
                };
                Ok((
                    quote! {#field_name},
                    quote! {
                        #deser_tokens
                    },
                ))
            }
            AccountField::CompositeField(_) => Err("CompositeFields not supported!"),
        })
        .collect();

    let (names, fields_deser): (Vec<_>, Vec<_>) = names_deser_pairs?.iter().cloned().unzip();

    let generated_deser_impl: syn::Item = parse_quote! {
        impl<'info> #impl_name<'info> {
            pub fn deserialize_option(
                metas: &'info [AccountMeta],
                accounts: &'info mut [Option<trdelnik_client::solana_sdk::account::Account>],
            ) -> core::result::Result<Self, FuzzingError> {
                let accounts = get_account_infos_option(accounts, metas)
                    .map_err(|_| FuzzingError::CannotGetAccounts)?;

                let mut accounts_iter = accounts.into_iter();

                #(#fields_deser)*

                Ok(Self {
                    #(#names),*
                })
            }
        }
    };

    Ok(generated_deser_impl.to_token_stream())
}

/// Get the identifier (name) of the passed sysvar type.
fn sysvar_to_ident(sysvar: &anchor_lang::anchor_syn::SysvarTy) -> String {
    let str = match sysvar {
        anchor_lang::anchor_syn::SysvarTy::Clock => "Clock",
        anchor_lang::anchor_syn::SysvarTy::Rent => "Rent",
        anchor_lang::anchor_syn::SysvarTy::EpochSchedule => "EpochSchedule",
        anchor_lang::anchor_syn::SysvarTy::Fees => "Fees",
        anchor_lang::anchor_syn::SysvarTy::RecentBlockhashes => "RecentBlockhashes",
        anchor_lang::anchor_syn::SysvarTy::SlotHashes => "SlotHashes",
        anchor_lang::anchor_syn::SysvarTy::SlotHistory => "SlotHistory",
        anchor_lang::anchor_syn::SysvarTy::StakeHistory => "StakeHistory",
        anchor_lang::anchor_syn::SysvarTy::Instructions => "Instructions",
        anchor_lang::anchor_syn::SysvarTy::Rewards => "Rewards",
    };
    str.into()
}

/// Converts passed account type to token streams. The function returns a pair of streams where the first
/// variable in the pair is the type itself and the second is a fully qualified function to deserialize
/// the given type.
pub fn ty_to_tokens(ty: &anchor_lang::anchor_syn::Ty) -> Option<(TokenStream, TokenStream)> {
    let (return_type, deser_method) = match ty {
        Ty::AccountInfo | Ty::UncheckedAccount => return None,
        Ty::SystemAccount => (
            quote! { SystemAccount<'_>},
            quote!(anchor_lang::accounts::system_account::SystemAccount::try_from(&acc)),
        ),
        Ty::Sysvar(sysvar) => {
            let id = syn::Ident::new(&sysvar_to_ident(sysvar), Span::call_site());
            (
                quote! { Sysvar<#id>},
                quote!(anchor_lang::accounts::sysvar::Sysvar::from_account_info(
                    &acc
                )),
            )
        }
        Ty::Signer => (
            quote! { Signer<'_>},
            quote!(anchor_lang::accounts::signer::Signer::try_from(&acc)),
        ),
        Ty::Account(acc) => {
            let path = &acc.account_type_path;
            (
                quote! { anchor_lang::accounts::account::Account<#path>},
                quote! {anchor_lang::accounts::account::Account::try_from(&acc)},
            )
        }
        Ty::AccountLoader(acc) => {
            let path = &acc.account_type_path;
            (
                quote! { anchor_lang::accounts::account_loader::AccountLoader<#path>},
                quote! {anchor_lang::accounts::account_loader::AccountLoader::try_from(&acc)},
            )
        }
        Ty::Program(prog) => {
            let path = &prog.account_type_path;
            (
                quote! { anchor_lang::accounts::program::Program<#path>},
                quote!(anchor_lang::accounts::program::Program::try_from(&acc)),
            )
        }
        Ty::Interface(interf) => {
            let path = &interf.account_type_path;
            (
                quote! { anchor_lang::accounts::interface::Interface<#path>},
                quote! {anchor_lang::accounts::interface::Interface::try_from(&acc)},
            )
        }
        Ty::InterfaceAccount(interf_acc) => {
            let path = &interf_acc.account_type_path;
            (
                quote! { anchor_lang::accounts::interface_account::InterfaceAccount<#path>},
                quote! {anchor_lang::accounts::interface_account::InterfaceAccount::try_from(&acc)},
            )
        }
        Ty::ProgramData => return None,
    };
    Some((return_type, deser_method))
}

/// Generates the code necessary to deserialize an account
fn deserialize_account_tokens(
    name: &syn::Ident,
    is_optional: bool,
    return_type: TokenStream,
    deser_method: TokenStream,
) -> TokenStream {
    if is_optional {
        quote! {
            let #name:Option<#return_type> = accounts_iter
            .next()
            .ok_or(FuzzingError::NotEnoughAccounts)?
            .map(|acc| #deser_method)
            .transpose()
            .unwrap_or(None);
        }
    } else {
        quote! {
            let #name: #return_type = accounts_iter
            .next()
            .ok_or(FuzzingError::NotEnoughAccounts)?
            .map(|acc| #deser_method)
            .ok_or(FuzzingError::AccountNotFound)?
            .map_err(|_| FuzzingError::CannotDeserializeAccount)?;
        }
    }
}

/// Generates the code used with raw accounts as AccountInfo
fn acc_info_tokens(name: &syn::Ident) -> TokenStream {
    quote! {
        let #name = accounts_iter
        .next()
        .ok_or(FuzzingError::NotEnoughAccounts)?
        .ok_or(FuzzingError::AccountNotFound)?;
    }
}

/// Generates the code used with Unchecked accounts
fn acc_unchecked_tokens(name: &syn::Ident) -> TokenStream {
    quote! {
        let #name = accounts_iter
        .next()
        .ok_or(FuzzingError::NotEnoughAccounts)?
        .map(anchor_lang::accounts::unchecked_account::UncheckedAccount::try_from)
        .ok_or(FuzzingError::AccountNotFound)?;
    }
}

/// Checks if the program attribute is present
fn has_program_attribute(attrs: &Vec<Attribute>) -> bool {
    for attr in attrs {
        if attr.path.is_ident("program") {
            return true;
        }
    }
    false
}
