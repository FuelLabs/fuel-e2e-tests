use fuels_core::code_gen::abigen::Abigen;
use proc_macro::TokenStream;
use syn::parse::{Parse, ParseStream, Result as ParseResult};
use syn::{parse_macro_input, Ident, LitStr, Token};
use test_macros::test_project_abi_path;

/// A shallow wrapper around `abigen!` from fuels-rs. It takes the given project
/// name and expands it to the path of the project inside the generated `assets`
/// dir.
///
/// # Arguments
///
/// * `input`: the sway project dir name.
#[proc_macro]
pub fn test_project_abigen(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ContractArgs);

    let full_path = test_project_abi_path(&args.abi);

    let c = Abigen::new(&args.name, &full_path).unwrap();

    c.expand().unwrap().into()
}

impl Parse for ContractArgs {
    fn parse(input: ParseStream) -> ParseResult<Self> {
        let name = input.parse::<Ident>()?.to_string();

        input.parse::<Token![,]>()?;

        let abi = input.parse::<LitStr>()?.value();

        if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }

        Ok(ContractArgs { name, abi })
    }
}

#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
struct ContractArgs {
    name: String,
    abi: String,
}
