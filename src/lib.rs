//! A procedural macro for tracing the execution of functions.
//!
//! Adding `#[trace]` to the top of any function will insert `println!` statements at the beginning
//! and the end of that function, notifying you of when that function was entered and exited and
//! printing the argument and return values.  This is useful for quickly debugging whether functions
//! that are supposed to be called are actually called without manually inserting print statements.
//!
//! Note that this macro requires all arguments to the function and the return value to have types
//! that implement `Debug`. You can disable the printing of certain arguments if necessary.
//!
//! You can also add `#[trace]` to `impl`s and `mod`s to enable tracing for all functions in the
//! `impl` or `mod`. If you use `#[trace]` on a `mod` or `impl` as well as on a method or function
//! inside one of those elements, then only the outermost `#[trace]` is used.
//!
//! `#[trace]` takes a few optional arguments that configure things like the prefixes to use,
//! enabling/disabling particular arguments or functions, and more. See the
//! [documentation](macro@trace) for details.
//!
//! ## Example
//!
//! See the examples in `examples/`. You can run the following example with
//! `cargo run --example example_prefix`.
//! ```
//! use trace::trace;
//!
//! trace::init_depth_var!();
//!
//! fn main() {
//!     foo(1, 2);
//! }
//!
//! #[trace]
//! fn foo(a: i32, b: i32) {
//!     println!("I'm in foo!");
//!     bar((a, b));
//! }
//!
//! #[trace(prefix_enter="[ENTER]", prefix_exit="[EXIT]")]
//! fn bar((a, b): (i32, i32)) -> i32 {
//!     println!("I'm in bar!");
//!     if a == 1 {
//!         2
//!     } else {
//!         b
//!     }
//! }
//! ```
//!
//! Output:
//! ```text
//! [+] Entering foo(a = 1, b = 2)
//! I'm in foo!
//!  [ENTER] Entering bar(a = 1, b = 2)
//! I'm in bar!
//!  [EXIT] Exiting bar = 2
//! [-] Exiting foo = ()
//! ```
//!
//! Note the convenience [`trace::init_depth_var!()`](macro@init_depth_var) macro which declares and
//! initializes the thread-local `DEPTH` variable that is used for indenting the output. Calling
//! `trace::init_depth_var!()` is equivalent to writing:
//! ```
//! use std::cell::Cell;
//!
//! thread_local! {
//!     static DEPTH: Cell<usize> = Cell::new(0);
//! }
//! ```
//!
//! The only time it can be omitted is when `#[trace]` is applied to `mod`s as it's defined for you
//! automatically (see `examples/example_mod.rs`). Note that the `DEPTH` variable isn't shared
//! between `mod`s, so indentation won't be perfect when tracing functions in multiple `mod`s. Also
//! note that using trace as an inner attribute (`#![trace]`) is not supported at this time.

mod args;

use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, Parser},
    parse_quote,
};

/// A convenience macro for declaring the `DEPTH` variable used for indenting the output
///
/// Calling this macro is equivalent to:
/// ```
/// use std::cell::Cell;
///
/// thread_local! {
///     static DEPTH: Cell<usize> = Cell::new(0);
/// }
/// ```
///
/// It is required to declare a `DEPTH` variable unless using `#[trace]` on a `mod`, in which case
/// the variable is declared for you.
#[proc_macro]
pub fn init_depth_var(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let output = if input.is_empty() {
        quote! {
            ::std::thread_local! {
                static DEPTH: ::std::cell::Cell<usize> = ::std::cell::Cell::new(0);
            }
        }
    } else {
        let input2 = proc_macro2::TokenStream::from(input);
        syn::Error::new_spanned(input2, "`init_depth_var` takes no arguments").to_compile_error()
    };

    output.into()
}

/// Enables tracing the execution of functions
///
/// It supports the following optional arguments (see the `examples` folder for examples of using
/// each of these):
///
/// - `prefix_enter` - The prefix of the `println!` statement when a function is entered. Defaults
/// to `[+]`.
///
/// - `prefix_exit` - The prefix of the `println!` statement when a function is exited. Defaults to
/// `[-]`.
///
/// - `enable` - When applied to a `mod` or `impl`, `enable` takes a list of function names to
/// print, not printing any functions that are not part of this list. All functions are enabled by
/// default. When applied to an `impl` method or a function, `enable` takes a list of arguments to
/// print, not printing any arguments that are not part of the list. All arguments are enabled by
/// default.
///
/// - `disable` - When applied to a `mod` or `impl`, `disable` takes a list of function names to not
/// print, printing all other functions in the `mod` or `impl`. No functions are disabled by
/// default. When applied to an `impl` method or a function, `disable` takes a list of arguments to
/// not print, printing all other arguments. No arguments are disabled by default.
///
/// - `pause` - When given as an argument to `#[trace]`, execution is paused after each line of
/// tracing output until enter is pressed. This allows you to trace through a program step by
/// step. Disabled by default.
///
/// - `pretty` - Pretty print the output (use `{:#?}` instead of `{:?}`). Disabled by default.
///
/// - `logging` - Use `log::trace!` from the `log` crate instead of `println`. Disabled by default.
///
/// Note that `enable` and `disable` can not be used together, and doing so will result in an error.
#[proc_macro_attribute]
pub fn trace(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let raw_args = syn::parse_macro_input!(args as syn::AttributeArgs);
    let args = match args::Args::from_raw_args(raw_args) {
        Ok(args) => args,
        Err(errors) => {
            return errors
                .iter()
                .map(syn::Error::to_compile_error)
                .collect::<proc_macro2::TokenStream>()
                .into()
        }
    };

    let output = if let Ok(item) = syn::Item::parse.parse(input.clone()) {
        expand_item(&args, item)
    } else if let Ok(impl_item) = syn::ImplItem::parse.parse(input.clone()) {
        expand_impl_item(&args, impl_item)
    } else {
        let input2 = proc_macro2::TokenStream::from(input);
        syn::Error::new_spanned(input2, "expected one of: `fn`, `impl`, `mod`").to_compile_error()
    };

    output.into()
}

#[derive(Clone, Copy)]
enum AttrApplied {
    Directly,
    Indirectly,
}

fn expand_item(args: &args::Args, mut item: syn::Item) -> proc_macro2::TokenStream {
    transform_item(args, AttrApplied::Directly, &mut item);

    match item {
        syn::Item::Fn(_) | syn::Item::Mod(_) | syn::Item::Impl(_) => item.into_token_stream(),
        _ => syn::Error::new_spanned(item, "#[trace] is not supported for this item")
            .to_compile_error(),
    }
}

fn expand_impl_item(args: &args::Args, mut impl_item: syn::ImplItem) -> proc_macro2::TokenStream {
    transform_impl_item(args, AttrApplied::Directly, &mut impl_item);

    match impl_item {
        syn::ImplItem::Method(_) => impl_item.into_token_stream(),
        _ => syn::Error::new_spanned(impl_item, "#[trace] is not supported for this impl item")
            .to_compile_error(),
    }
}

fn transform_item(args: &args::Args, attr_applied: AttrApplied, item: &mut syn::Item) {
    match *item {
        syn::Item::Fn(ref mut item_fn) => transform_fn(args, attr_applied, item_fn),
        syn::Item::Mod(ref mut item_mod) => transform_mod(args, attr_applied, item_mod),
        syn::Item::Impl(ref mut item_impl) => transform_impl(args, attr_applied, item_impl),
        _ => (),
    }
}

fn transform_fn(args: &args::Args, attr_applied: AttrApplied, item_fn: &mut syn::ItemFn) {
    item_fn.block = Box::new(construct_traced_block(
        args,
        attr_applied,
        &item_fn.sig,
        &item_fn.block,
    ));
}

fn transform_mod(args: &args::Args, attr_applied: AttrApplied, item_mod: &mut syn::ItemMod) {
    assert!(
        (item_mod.content.is_some() && item_mod.semi.is_none())
            || (item_mod.content.is_none() && item_mod.semi.is_some())
    );

    if item_mod.semi.is_some() {
        unimplemented!();
    }

    if let Some((_, items)) = item_mod.content.as_mut() {
        items.iter_mut().for_each(|item| {
            if let AttrApplied::Directly = attr_applied {
                match *item {
                    syn::Item::Fn(syn::ItemFn {
                        sig: syn::Signature { ref ident, .. },
                        ..
                    })
                    | syn::Item::Mod(syn::ItemMod { ref ident, .. }) => match args.filter {
                        args::Filter::Enable(ref idents) if !idents.contains(ident) => {
                            return;
                        }
                        args::Filter::Disable(ref idents) if idents.contains(ident) => {
                            return;
                        }
                        _ => (),
                    },
                    _ => (),
                }
            }

            transform_item(args, AttrApplied::Indirectly, item);
        });

        items.insert(
            0,
            parse_quote! {
                ::std::thread_local! {
                    static DEPTH: ::std::cell::Cell<usize> = ::std::cell::Cell::new(0);
                }
            },
        );
    }
}

fn transform_impl(args: &args::Args, attr_applied: AttrApplied, item_impl: &mut syn::ItemImpl) {
    item_impl.items.iter_mut().for_each(|impl_item| {
        if let syn::ImplItem::Method(ref mut impl_item_method) = *impl_item {
            if let AttrApplied::Directly = attr_applied {
                let ident = &impl_item_method.sig.ident;

                match args.filter {
                    args::Filter::Enable(ref idents) if !idents.contains(ident) => {
                        return;
                    }
                    args::Filter::Disable(ref idents) if idents.contains(ident) => {
                        return;
                    }
                    _ => (),
                }
            }

            impl_item_method.block = construct_traced_block(
                args,
                AttrApplied::Indirectly,
                &impl_item_method.sig,
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
    #[allow(clippy::single_match)]
    match *impl_item {
        syn::ImplItem::Method(ref mut impl_item_method) => {
            transform_method(args, attr_applied, impl_item_method)
        }
        _ => (),
    }
}

fn transform_method(
    args: &args::Args,
    attr_applied: AttrApplied,
    impl_item_method: &mut syn::ImplItemMethod,
) {
    impl_item_method.block = construct_traced_block(
        args,
        attr_applied,
        &impl_item_method.sig,
        &impl_item_method.block,
    );
}

fn construct_traced_block(
    args: &args::Args,
    attr_applied: AttrApplied,
    sig: &syn::Signature,
    original_block: &syn::Block,
) -> syn::Block {
    let arg_idents = extract_arg_idents(args, attr_applied, sig);
    let arg_idents_format = arg_idents
        .iter()
        .map(|arg_ident| format!("{} = {{:?}}", arg_ident))
        .collect::<Vec<_>>()
        .join(", ");

    let pretty = if args.pretty { "#" } else { "" };
    let entering_format = format!(
        "{{:depth$}}{} Entering {}({})",
        args.prefix_enter, sig.ident, arg_idents_format
    );
    let exiting_format = format!(
        "{{:depth$}}{} Exiting {} = {{:{}?}}",
        args.prefix_exit, sig.ident, pretty
    );

    let pause_stmt = if args.pause {
        quote! {{
            use std::io::{self, BufRead};
            let stdin = io::stdin();
            stdin.lock().lines().next();
        }}
    } else {
        quote!()
    };

    let printer = if args.logging {
        quote! { log::trace! }
    } else {
        quote! { println! }
    };

    parse_quote! {{
        #printer(#entering_format, "", #(#arg_idents,)* depth = DEPTH.with(|d| d.get()));
        #pause_stmt
        DEPTH.with(|d| d.set(d.get() + 1));
        let fn_return_value = #original_block;
        DEPTH.with(|d| d.set(d.get() - 1));
        #printer(#exiting_format, "", fn_return_value, depth = DEPTH.with(|d| d.get()));
        #pause_stmt
        fn_return_value
    }}
}

fn extract_arg_idents(
    args: &args::Args,
    attr_applied: AttrApplied,
    sig: &syn::Signature,
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
                        args::Filter::Enable(ref idents) if !idents.contains(ident) => {
                            return;
                        }
                        args::Filter::Disable(ref idents) if idents.contains(ident) => {
                            return;
                        }
                        _ => (),
                    }
                }

                arg_idents.push(ident.clone());
            }
            syn::Pat::Tuple(ref pat_tuple) => {
                pat_tuple.elems.iter().for_each(|pat| {
                    process_pat(args, attr_applied, pat, arg_idents);
                });
            }
            _ => unimplemented!(),
        }
    }

    let mut arg_idents = vec![];

    for input in &sig.inputs {
        match input {
            syn::FnArg::Receiver(_) => (), // ignore `self`
            syn::FnArg::Typed(arg_typed) => {
                process_pat(args, attr_applied, &arg_typed.pat, &mut arg_idents);
            }
        }
    }

    arg_idents
}
