use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[proc_macro_attribute]
pub fn fs_mod(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemStruct);

    let ident = input.ident.clone();
    let output = quote! {
        fs_modding_api::static_assertions::assert_impl_all!(#ident: Default);
        mod _fs_modding_api_impl {
            use super::#ident;
            use fs_modding_api::wasm_plugin_guest as wasm_plugin_guest;
            #[wasm_plugin_guest::export_function]
            fn init() -> fs_modding_api::fs_common_types::modding::ModMeta {
                fs_modding_api::init(#ident::default())
            }
        }

        #input
    };
    output.into()
}
