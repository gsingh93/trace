#![feature(quote, plugin_registrar, rustc_private, collections)]

extern crate syntax;
extern crate rustc;

use std::slice::SliceConcatExt;
use syntax::ext::quote::rt::ExtParseUtils;
use syntax::ext::quote::rt::ToTokens;

use rustc::plugin::Registry;

use syntax::ptr::P;
use syntax::ast::{self, Item, Item_, MetaItem, ItemFn, ItemMod, Block, Stmt, Ident, TokenTree,
                  Mod};
use syntax::ast::MetaItem_::{MetaList, MetaNameValue};
use syntax::ast::Lit_::LitStr;
use syntax::codemap::{self, Span, Spanned};
use syntax::ext::base::ExtCtxt;
use syntax::ext::base::SyntaxExtension::Modifier;

use syntax::ext::build::AstBuilder;
use syntax::parse::token::{self, intern};

#[plugin_registrar]
pub fn registrar(reg: &mut Registry) {
    reg.register_syntax_extension(intern("trace"), Modifier(Box::new(trace_expand)));
}

fn trace_expand(cx: &mut ExtCtxt, sp: Span, meta: &MetaItem, item: P<Item>) -> P<Item> {
    let (prefix_enter, prefix_exit) = get_prefixes(meta);
    match &item.node {
        &ItemFn(_, _, _, _, _) => {
            let new_item = expand_function(cx, prefix_enter, prefix_exit, &item, sp);
            cx.item(item.span, item.ident, item.attrs.clone(), new_item)
        }
        &ItemMod(ref m) => {
            let mut new_items = vec!();
            for i in m.items.iter() {
                if let &ItemFn(_, _, _, _, _) = &i.node {
                    let new_item = expand_function(cx, prefix_enter, prefix_exit, i, i.span);
                    new_items.push(cx.item(i.span, i.ident, i.attrs.clone(), new_item));
                } else {
                    new_items.push((*i).clone());
                }
            }
            return cx.item(item.span, item.ident, item.attrs.clone(),
                           ItemMod(Mod { inner: m.inner, items: new_items }))
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

fn expand_function(cx: &mut ExtCtxt, prefix_enter: &str, prefix_exit: &str, item: &P<Item>,
                   sp: Span) -> Item_ {
    let ref name = item.ident.name.as_str();
    if let &ItemFn(ref decl, style, abi, ref generics, _) = &item.node {
        let fn_ident = ast::Ident::new(intern(&format!("__trace_inner_{}", name)));
        let inner_item = P(Item { attrs: Vec::new(), vis: ast::Inherited, .. (**item).clone() });
        let new_decl = fn_decl(sp.clone(), fn_ident, inner_item);

        let args = match args(cx, &**decl, sp) {
            Some(args) => args,
            None => { return item.node.clone(); }
        };
        let ty_args = ty_args(generics, sp);
        let result_expr = assign_result_expr(cx, fn_ident, args.clone(), ty_args);

        let new_block = new_block(cx, prefix_enter, prefix_exit, name, new_decl, result_expr, args);
        ItemFn(decl.clone(), style, abi, generics.clone(), new_block)
    } else {
        panic!("Expected ItemFn")
    }
}

fn fn_decl(sp: Span, fn_ident: Ident, item: P<Item>) -> P<Stmt> {
    match &item.node {
        &ast::ItemFn(ref decl, style, abi, ref generics, ref body) => {
            let inner = Item {
                ident: fn_ident,
                node: ast::ItemFn(decl.clone(), style, abi, generics.clone(), body.clone()),
                .. (*item).clone() };

            let inner = ast::DeclItem(P(inner));
            let inner = P(Spanned{ node: inner, span: sp });

            let stmt = ast::StmtDecl(inner, ast::DUMMY_NODE_ID);
            P(Spanned{ node: stmt, span: sp })
        }
        _ => panic!("This should be checked by the caller")
    }
}

fn assign_result_expr(cx: &mut ExtCtxt, fn_ident: Ident, arg_toks: Vec<TokenTree>,
                      ty_arg_toks: Vec<TokenTree>) -> P<Stmt> {
    if ty_arg_toks.is_empty() {
        quote_stmt!(cx, let __trace_result = $fn_ident::<$ty_arg_toks>($arg_toks)).unwrap()
    } else {
        quote_stmt!(cx, let __trace_result = $fn_ident($arg_toks)).unwrap()
    }
}

fn args(cx: &ExtCtxt, decl: &ast::FnDecl, sp: Span) -> Option<Vec<TokenTree>> {
    if !decl.inputs.iter().map(|a| &*a.pat).all(is_sane_pattern) {
        return None;
    }

    let cm = &cx.parse_sess.span_diagnostic.cm;
    Some(decl.inputs
        .iter()
        // span_to_snippet really shouldn't return None, so I hope the
        // unwrap is OK. Not sure we can do anything it is does in any case.
        .map(|a| cx.parse_tts(cm.span_to_snippet(a.pat.span).unwrap()))
        .collect::<Vec<_>>()
        .connect(&ast::TtToken(sp, token::Comma)))
}

fn ty_args(generics: &ast::Generics, sp: Span) -> Vec<TokenTree> {
    generics.ty_params
        .iter()
        .map(|tp| vec![token::Ident(tp.ident, token::Plain)])
        .collect::<Vec<_>>()
        .connect(&token::Comma)
        .into_iter()
        .map(|t| ast::TtToken(sp, t))
        .collect()
}

// Check that a pattern can trivially be used to instantiate that pattern.
// For example if we have `fn foo((x, y): ...) {...}` we can call `foo((x, y))`
// (assuming x and y are in scope and have the correct type) with the exact same
// syntax as the pattern is declared. But if the pattern is `z @ (x,y)` we cannot
// (we need to use `(x, y)`).
//
// Ideally we would just translate the pattern to the correct one. But for now
// we just check if we can skip the translation phase and fail otherwise (FIXME).
fn is_sane_pattern(pat: &ast::Pat) -> bool {
    match &pat.node {
        &ast::PatWild(_) | &ast::PatMac(_) | &ast::PatStruct(..) |
        &ast::PatLit(_) | &ast::PatRange(..) | &ast::PatVec(..) => false,
        &ast::PatIdent(ast::BindByValue(ast::MutImmutable), _, _) => true,
        &ast::PatIdent(..) => false,
        &ast::PatEnum(_, Some(ref ps)) | &ast::PatTup(ref ps) =>
            ps.iter().all(|p| is_sane_pattern(&**p)),
        &ast::PatEnum(..) => false,
        &ast::PatBox(ref p) | &ast::PatRegion(ref p, _) => is_sane_pattern(&**p)
    }
}

fn get_idents(args: &[TokenTree], idents: &mut Vec<Ident>) {
    for arg in args.iter() {
        match arg {
            &ast::TtToken(_, token::Ident(ref ident, _)) => idents.push((*ident).clone()),
            &ast::TtToken(_, token::Comma) => (),
            &ast::TtDelimited(_, ref delim) => get_idents(&delim.tts, idents),
            _ => panic!("Unexpected token {:?}", arg)
        }
    }
}

fn new_block(cx: &mut ExtCtxt, prefix_enter: &str, prefix_exit: &str, name: &str,
             inner_func: P<Stmt>, result_expr: P<Stmt>, args: Vec<TokenTree>) -> P<Block> {
    let mut idents = vec!();
    get_idents(&args, &mut idents);
    let args: Vec<TokenTree> = idents
        .iter()
        .map(|ident| vec![token::Ident((*ident).clone(), token::Plain)])
        .collect::<Vec<_>>()
        .connect(&token::Comma)
        .into_iter()
        .map(|t| ast::TtToken(codemap::DUMMY_SP, t))
        .collect();


    let mut arg_fmt = vec!();
    for ident in idents.iter() {
        arg_fmt.push(format!("{}: {{:?}}", ident))
    }
    let arg_fmt_str = &*arg_fmt.connect(", ");
    let new_block = quote_expr!(cx,
    unsafe {
        let mut s = String::new();
        (0..depth).map(|_| s.push(' ')).count();
        let args = format!($arg_fmt_str, $args);
        println!("{}{} Entering {}({})", s, $prefix_enter, $name, args);
        depth += 1;
        $inner_func;
        $result_expr;
        depth -= 1;
        println!("{}{} Exiting {} = {:?}", s, $prefix_exit, $name, __trace_result);
        __trace_result
    });
    cx.block_expr(new_block)
}
