use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0:?}")]
    RustParsingError(#[from] syn::Error),
    #[error("missing or invalid program item: '{0}'")]
    MissingOrInvalidProgramItems(&'static str),
}

#[derive(Debug)]
pub struct Idl {
    pub programs: Vec<IdlForDetect>,
}

#[derive(Debug)]
pub struct IdlForDetect {
    pub functions: Vec<syn::ItemFn>,
}

pub async fn parse_to_idl_program_for_detect(
    code: &str,
    module_name: &String,
) -> Result<IdlForDetect, Error> {
    let mut mod_instruction_contents = None::<syn::ItemMod>;

    for item in syn::parse_file(code)?.items.into_iter() {
        if let syn::Item::Mod(item_mod) = item {
            let mod_name = String::from(item_mod.ident.to_string().as_str());
            if mod_name == *module_name {
                mod_instruction_contents = Some(item_mod)
            }
        }
    }
    let mod_instruction_contents =
        mod_instruction_contents.ok_or(Error::MissingOrInvalidProgramItems("missing mod "))?;

    let program_functions_items = {
        let items = mod_instruction_contents
            .content
            .map(|(_, items)| items)
            .unwrap_or_default();

        let mut function_vec = Vec::<syn::ItemFn>::new();

        for item in items.into_iter() {
            match item {
                syn::Item::Fn(item_fn) => function_vec.push(item_fn),
                syn::Item::Mod(function_mod) => {
                    let inner_mod_items = function_mod
                        .content
                        .map(|(_, items)| items)
                        .unwrap_or_default();
                    for inner_item in inner_mod_items.into_iter() {
                        if let syn::Item::Fn(item_fn) = inner_item {
                            function_vec.push(item_fn)
                        }
                    }
                }
                _ => (),
            }
        }
        function_vec
    };

    // ------ // ------

    Ok(IdlForDetect {
        functions: program_functions_items,
    })
}
