use proc_macro::{TokenStream, TokenTree};
use proc_macro2::Span;
use quote::quote;
use syn::Ident;

pub(crate) fn declare_output_enum(input: TokenStream) -> TokenStream {
    let branches = match input.into_iter().next() {
        Some(TokenTree::Group(group)) => group.stream().into_iter().count(),
        _ => panic!("unexpected macro input"),
    };

    let variants = (0..branches)
        .map(|num| Ident::new(&format!("_{}", num), Span::call_site()))
        .collect::<Vec<_>>();

    let mask = Ident::new(
        if branches <= 8 {
            "u8"
        } else if branches <= 16 {
            "u16"
        } else if branches <= 32 {
            "u32"
        } else if branches <= 64 {
            "u64"
        } else {
            panic!("up to 64 branches supported");
        },
        Span::call_site(),
    );

    TokenStream::from(quote! {
        pub(super) enum Out<#( #variants ),*> {
            #( #variants(#variants), )*
            Disabled,
        }

        pub(super) type Mask = #mask;
    })
}

pub(crate) fn clean_pattern_macro(input: TokenStream) -> TokenStream {
    let mut input: syn::Pat = match syn::parse(input.clone()) {
        Ok(it) => it,
        Err(_) => return input,
    };

    clean_pattern(&mut input);
    quote::ToTokens::into_token_stream(input).into()
}

fn clean_pattern(pat: &mut syn::Pat) {
    match pat {
        syn::Pat::Box(_box) => {}
        syn::Pat::Lit(_literal) => {}
        syn::Pat::Macro(_macro) => {}
        syn::Pat::Path(_path) => {}
        syn::Pat::Range(_range) => {}
        syn::Pat::Rest(_rest) => {}
        syn::Pat::Verbatim(_tokens) => {}
        syn::Pat::Wild(_underscore) => {}
        syn::Pat::Ident(ident) => {
            ident.by_ref = None;
            ident.mutability = None;
            if let Some((_at, pat)) = &mut ident.subpat {
                clean_pattern(&mut *pat);
            }
        }
        syn::Pat::Or(or) => {
            for case in or.cases.iter_mut() {
                clean_pattern(case);
            }
        }
        syn::Pat::Slice(slice) => {
            for elem in slice.elems.iter_mut() {
                clean_pattern(elem);
            }
        }
        syn::Pat::Struct(struct_pat) => {
            for field in struct_pat.fields.iter_mut() {
                clean_pattern(&mut field.pat);
            }
        }
        syn::Pat::Tuple(tuple) => {
            for elem in tuple.elems.iter_mut() {
                clean_pattern(elem);
            }
        }
        syn::Pat::TupleStruct(tuple) => {
            for elem in tuple.pat.elems.iter_mut() {
                clean_pattern(elem);
            }
        }
        syn::Pat::Reference(reference) => {
            reference.mutability = None;
            clean_pattern(&mut *reference.pat);
        }
        syn::Pat::Type(type_pat) => {
            clean_pattern(&mut *type_pat.pat);
        }
        _ => {}
    }
}

#[proc_macro]
pub fn select_priv_declare_output_enum(input: TokenStream) -> TokenStream {
    declare_output_enum(input)
}

#[proc_macro]
pub fn select_priv_clean_pattern(input: TokenStream) -> TokenStream {
    clean_pattern_macro(input)
}
