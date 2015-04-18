#![feature(quote, plugin_registrar, rustc_private, custom_attribute, plugin)]

extern crate syntax;
extern crate rustc;

use rustc::plugin::Registry;

use syntax::ptr::P;
use syntax::ast::{Item, Item_, MetaItem, ItemFn, Block};
use syntax::ast::MetaItem_::{MetaList, MetaNameValue};
use syntax::ast::Lit_::LitStr;
use syntax::codemap::Span;
use syntax::ext::base::ExtCtxt;
use syntax::ext::base::SyntaxExtension::Modifier;

use syntax::ext::build::AstBuilder;
use syntax::parse::token::intern;

#[plugin_registrar]
pub fn registrar(reg: &mut Registry) {
    reg.register_syntax_extension(intern("trace"), Modifier(Box::new(trace_expand)));
}

fn trace_expand(cx: &mut ExtCtxt, sp: Span, meta: &MetaItem, item: P<Item>) -> P<Item> {
    let (prefix_enter, prefix_exit) = get_prefixes(meta);
    match item.node {
        ItemFn(_, _, _, _, _) => {
            let ref name = item.ident.name.as_str();
            let new_item = mod_function(cx, prefix_enter, prefix_exit, name, &item.node);
            cx.item(item.span, item.ident, item.attrs.clone(), new_item)
        }
        _ => {
            cx.span_err(sp, "trace is only permissible on functions");
            item.clone()
        }
    }
}

fn new_block(prefix_enter: &str, prefix_exit: &str, cx: &mut ExtCtxt, name: &str,
             block: &P<Block>) -> P<Block> {
    let new_block = quote_expr!(cx,
    {
        println!("{} Entering {}", $prefix_enter, $name);
        $block;
        println!("{} Exiting {}", $prefix_exit, $name);
    });
    cx.block_expr(new_block)
}

fn mod_function(cx: &mut ExtCtxt, prefix_enter: &str, prefix_exit: &str, name: &str,
                item: &Item_) -> Item_ {
    if let &ItemFn(ref decl, ref style, ref abi, ref generics, ref block) = item {
        let new_block = new_block(prefix_enter, prefix_exit, cx, name, block);
        ItemFn((*decl).clone(), style.clone(), abi.clone(), generics.clone(), new_block)
    } else {
        panic!("Expected ItemFn")
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
