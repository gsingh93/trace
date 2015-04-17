#![feature(quote, plugin_registrar, rustc_private, custom_attribute, plugin)]

extern crate syntax;
extern crate rustc;

use rustc::plugin::Registry;

use syntax::ptr::P;
use syntax::ast::{Item, MetaItem, ItemFn};
use syntax::ast::MetaItem_::{MetaList, MetaNameValue};
use syntax::ast::Lit_::LitStr;
use syntax::codemap::Span;
use syntax::ext::base::ExtCtxt;
use syntax::ext::base::SyntaxExtension::Modifier;

use syntax::ext::build::AstBuilder;
use syntax::parse::token::intern;

#[plugin_registrar]
pub fn registrar(reg: &mut Registry) {
    reg.register_syntax_extension(intern("trace"),
                                  Modifier(Box::new(trace_expand)));
}

fn trace_expand(cx: &mut ExtCtxt, sp: Span, meta: &MetaItem,
                item: P<Item>) -> P<Item> {
    let (prefix_enter, prefix_exit) = get_prefixes(meta);
    match item.node {
        ItemFn(ref decl, ref style, ref abi, ref generics, ref block) => {
            let ref ident = item.ident.name.as_str();
            let new_contents = quote_expr!(&mut *cx,
                {
                    println!("{} Entering {}", $prefix_enter, $ident);
                    $block;
                    println!("{} Exiting {}", $prefix_exit, $ident);
                }
            );
            let new_item_ = ItemFn((*decl).clone(), style.clone(),
                                        abi.clone(), generics.clone(),
                                        cx.block_expr(new_contents));

            cx.item(item.span, item.ident, item.attrs.clone(), new_item_)
        }
        _ => {
            cx.span_err(sp, "trace is only permissible on functions");
            item.clone()
        }
    }
}

fn get_prefixes(meta: &MetaItem) -> (&str, &str) {
    let mut prefix_enter = "[+]";
    let mut prefix_exit = "[-]";
    if let MetaList(_, ref v) = meta.node {
        for i in v {
            if let MetaNameValue(ref name, ref s) = i.node {
                if *name == "prefix_enter" {
                    if let LitStr(ref new_prefix, _) = s.node {
                        prefix_enter = &*new_prefix;
                    }
                } else if *name == "prefix_exit" {
                    if let LitStr(ref new_prefix, _) = s.node {
                        prefix_exit = &*new_prefix;
                    }
                }
            }
        }
    }
    (prefix_enter, prefix_exit)
}
