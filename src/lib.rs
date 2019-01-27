extern crate proc_macro;
extern crate proc_macro2;
extern crate quote;
extern crate syn;

mod args;

use quote::{ToTokens, quote};
use syn::{
    parse_quote,
    parse::{Parse, Parser},
    spanned::Spanned,
};


#[proc_macro_attribute]
pub fn trace(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let raw_args = syn::parse_macro_input!(args as syn::AttributeArgs);
    let args = match args::Args::from_raw_args(raw_args) {
        Ok(args) => args,
        Err(errors) => return errors
            .iter()
            .map(syn::parse::Error::to_compile_error)
            .collect::<proc_macro2::TokenStream>()
            .into(),
    };

    let output = if let Ok(item) = syn::Item::parse.parse(input.clone()) {
        expand_item(&args, item)
    } else if let Ok(impl_item) = syn::ImplItem::parse.parse(input.clone()) {
        expand_impl_item(&args, impl_item)
    } else {
        let span = proc_macro2::TokenStream::from(input).span();
        syn::parse::Error::new(span, "expected one of: `fn`, `impl`, `mod`").to_compile_error()
    };

    output.into()
}


#[derive(Clone, Copy)]
enum AttrApplied {
    Directly,
    Indirectly,
}

fn expand_item(
    args: &args::Args,
    mut item: syn::Item,
) -> proc_macro2::TokenStream {
    transform_item(args, AttrApplied::Directly, &mut item);

    match item {
        syn::Item::Fn(_)   |
        syn::Item::Mod(_)  |
        syn::Item::Impl(_) => item.into_token_stream(),
        _ => {
            syn::parse::Error::new(item.span(), "#[trace] is not supported for this item")
                .to_compile_error()
        },
    }
}

fn expand_impl_item(
    args: &args::Args,
    mut impl_item: syn::ImplItem,
) -> proc_macro2::TokenStream {
    transform_impl_item(args, AttrApplied::Directly, &mut impl_item);

    match impl_item {
        syn::ImplItem::Method(_) => impl_item.into_token_stream(),
        _ => {
            syn::parse::Error::new(impl_item.span(), "#[trace] is not supported for this impl item")
                .to_compile_error()
        },
    }
}


fn transform_item(
    args: &args::Args,
    attr_applied: AttrApplied,
    item: &mut syn::Item,
) {
    match *item {
        syn::Item::Fn(ref mut item_fn) => transform_fn(args, attr_applied, item_fn),
        syn::Item::Mod(ref mut item_mod) => transform_mod(args, attr_applied, item_mod),
        syn::Item::Impl(ref mut item_impl) => transform_impl(args, attr_applied, item_impl),
        _ => (),
    }
}

fn transform_fn(
    args: &args::Args,
    attr_applied: AttrApplied,
    item_fn: &mut syn::ItemFn,
) {
    item_fn.block = Box::new(construct_traced_block(
        &args,
        attr_applied,
        &item_fn.ident,
        &item_fn.decl,
        &item_fn.block,
    ));
}

fn transform_mod(
    args: &args::Args,
    attr_applied: AttrApplied,
    item_mod: &mut syn::ItemMod,
) {
    assert!(
        (item_mod.content.is_some() && item_mod.semi.is_none()) ||
        (item_mod.content.is_none() && item_mod.semi.is_some())
    );

    if item_mod.semi.is_some() {
        unimplemented!();
    }

    if let Some((_, items)) = item_mod.content.as_mut() {
        items.iter_mut().for_each(|item| {
            if let AttrApplied::Directly = attr_applied {
                match *item {
                    syn::Item::Fn(syn::ItemFn { ref ident, .. })   |
                    syn::Item::Mod(syn::ItemMod { ref ident, .. }) => match args.filter {
                        args::Filter::Enable(ref idents) if !idents.contains(ident) => { return; }
                        args::Filter::Disable(ref idents) if idents.contains(ident) => { return; }
                        _ => (),
                    },
                    _ => (),
                }
            }

            transform_item(args, AttrApplied::Indirectly, item);
        });

        items.insert(0, parse_quote! {
            ::std::thread_local! {
                static DEPTH: ::std::cell::Cell<usize> = ::std::cell::Cell::new(0);
            }
        });
    }
}

fn transform_impl(
    args: &args::Args,
    attr_applied: AttrApplied,
    item_impl: &mut syn::ItemImpl,
) {
    item_impl.items.iter_mut().for_each(|impl_item| {
        if let syn::ImplItem::Method(ref mut impl_item_method) = *impl_item {
            if let AttrApplied::Directly = attr_applied {
                let ident = &impl_item_method.sig.ident;

                match args.filter {
                    args::Filter::Enable(ref idents) if !idents.contains(ident) => { return; }
                    args::Filter::Disable(ref idents) if idents.contains(ident) => { return; }
                    _ => (),
                }
            }

            impl_item_method.block = construct_traced_block(
                &args,
                AttrApplied::Indirectly,
                &impl_item_method.sig.ident,
                &impl_item_method.sig.decl,
                &impl_item_method.block,
            );
        }
    });
}

fn transform_impl_item(
    args: &args::Args,
    attr_applied: AttrApplied,
    impl_item: &mut syn::ImplItem,
) {
    // Will probably add more cases in the future
    #[cfg_attr(feature = "cargo-clippy", allow(single_match))]
    match *impl_item {
        syn::ImplItem::Method(ref mut impl_item_method) => {
            transform_method(args, attr_applied, impl_item_method)
        },
        _ => (),
    }
}

fn transform_method(
    args: &args::Args,
    attr_applied: AttrApplied,
    impl_item_method: &mut syn::ImplItemMethod,
) {
    impl_item_method.block = construct_traced_block(
        &args,
        attr_applied,
        &impl_item_method.sig.ident,
        &impl_item_method.sig.decl,
        &impl_item_method.block,
    );
}


fn construct_traced_block(
    args: &args::Args,
    attr_applied: AttrApplied,
    ident: &proc_macro2::Ident,
    fn_decl: &syn::FnDecl,
    original_block: &syn::Block,
) -> syn::Block {
    let arg_idents = extract_arg_idents(args, attr_applied, &fn_decl);
    let arg_idents_format = arg_idents
        .iter()
        .map(|arg_ident| format!("{} = {{:?}}", arg_ident))
        .collect::<Vec<_>>()
        .join(", ");

    let entering_format =
        format!("{{:depth$}}{} Entering {}({})", args.prefix_enter, ident, arg_idents_format);
    let exiting_format =
        format!("{{:depth$}}{} Exiting {} = {{:?}}", args.prefix_exit, ident);

    let pause_stmt = if args.pause {
        quote! {{
            use std::io::{self, BufRead};
            let stdin = io::stdin();
            stdin.lock().lines().next();
        }}
    } else {
        quote!()
    };

    parse_quote! {{
        println!(#entering_format, "", #(#arg_idents,)* depth = DEPTH.with(|d| d.get()));
        #pause_stmt
        DEPTH.with(|d| d.set(d.get() + 1));
        let mut fn_closure = move || #original_block;
        let fn_return_value = fn_closure();
        DEPTH.with(|d| d.set(d.get() - 1));
        println!(#exiting_format, "", fn_return_value, depth = DEPTH.with(|d| d.get()));
        #pause_stmt
        fn_return_value
    }}
}

fn extract_arg_idents(
    args: &args::Args,
    attr_applied: AttrApplied,
    fn_decl: &syn::FnDecl,
) -> Vec<proc_macro2::Ident> {
    fn process_pat(
        args: &args::Args,
        attr_applied: AttrApplied,
        pat: &syn::Pat,
        arg_idents: &mut Vec<proc_macro2::Ident>,
    ) {
        match *pat {
            syn::Pat::Ident(ref pat_ident) => {
                let ident = &pat_ident.ident;

                if let AttrApplied::Directly = attr_applied {
                    match args.filter {
                        args::Filter::Enable(ref idents) if !idents.contains(ident) => { return; },
                        args::Filter::Disable(ref idents) if idents.contains(ident) => { return; },
                        _ => (),
                    }
                }

                arg_idents.push(ident.clone());
            },
            syn::Pat::Tuple(ref pat_tuple) => {
                pat_tuple.front.iter().for_each(|pat| {
                    process_pat(args, attr_applied, pat, arg_idents);
                });
            },
            _ => unimplemented!(),
        }
    }

    let mut arg_idents = vec![];

    for input in &fn_decl.inputs {
        match *input {
            syn::FnArg::SelfRef(_)   |
            syn::FnArg::SelfValue(_) => (),  // ignore `self`
            syn::FnArg::Captured(ref arg_captured) => {
                process_pat(args, attr_applied, &arg_captured.pat, &mut arg_idents);
            },
            syn::FnArg::Inferred(_) |
            syn::FnArg::Ignored(_)  => unimplemented!(),
        }
    }

    arg_idents
}
