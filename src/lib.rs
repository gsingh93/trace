#![feature(quote, plugin_registrar, rustc_private, collections)]

extern crate syntax;
extern crate rustc;

use std::slice::SliceConcatExt;
use std::collections::HashSet;

use rustc::plugin::Registry;

use syntax::ptr::P;
use syntax::ast::{self, Item, Item_, MetaItem, ItemFn, ItemMod, Block, Ident, TokenTree, FnDecl,
                  Mod, ItemStatic, ItemImpl, ImplItem, ImplItem_};
use syntax::ast::ImplItem_::MethodImplItem;
use syntax::ast::Expr_::ExprLit;
use syntax::ast::Mutability::MutMutable;
use syntax::ast::MetaItem_::{MetaList, MetaNameValue, MetaWord};
use syntax::ast::Lit_::{LitStr, LitInt};
use syntax::codemap::{self, Span};
use syntax::ext::base::{ExtCtxt, Annotatable};
use syntax::ext::base::SyntaxExtension::MultiModifier;
use syntax::ext::quote::rt::ExtParseUtils;
use syntax::ext::quote::rt::ToTokens;
use syntax::ext::build::AstBuilder;
use syntax::parse::token::{self, intern};

#[plugin_registrar]
pub fn registrar(reg: &mut Registry) {
    reg.register_syntax_extension(intern("trace"), MultiModifier(Box::new(trace_expand)));
}

fn trace_expand(cx: &mut ExtCtxt, sp: Span, meta: &MetaItem,
                annotatable: Annotatable) -> Annotatable {
    let options = get_options(cx, meta);
    match annotatable {
        Annotatable::Item(item) => {
            let res = match &item.node {
                &ItemFn(..) => {
                    let new_item = expand_function(cx, options, &item, true);
                    cx.item(item.span, item.ident, item.attrs.clone(), new_item)
                }
                &ItemMod(ref m) => {
                    let new_items = expand_mod(cx, m, options);
                    cx.item(item.span, item.ident, item.attrs.clone(),
                            ItemMod(Mod { inner: m.inner, items: new_items }))
                }
                &ItemImpl(safety, polarity, ref generics, ref traitref, ref ty, ref items) => {
                    let new_items = expand_impl(cx, &**items, options);
                    cx.item(item.span, item.ident, item.attrs.clone(),
                            ItemImpl(safety, polarity, generics.clone(), traitref.clone(),
                                     ty.clone(), new_items))
                }
                _ => {
                    cx.span_err(sp, "trace is only permissible on functions, mods, or impls");
                    item.clone()
                }
            };
            Annotatable::Item(res)
        }
        Annotatable::ImplItem(item) => {
            let new_item = expand_impl_method(cx, options, &item, true);
            Annotatable::ImplItem(
                P(ImplItem { node: new_item, attrs: vec!(), .. (*item).clone() }))
        }
        Annotatable::TraitItem(_) => {
            cx.span_err(sp, "trace is not applicable to trait items");
            annotatable.clone()
        }
    }
}

#[derive(Clone)]
struct Options {
    prefix_enter: String,
    prefix_exit: String,
    enable: Option<HashSet<String>>,
    disable: Option<HashSet<String>>,
    pause: bool
}

impl Options {
    fn new() -> Options {
        Options { prefix_enter: "[+]".to_string(), prefix_exit: "[-]".to_string(),
                  enable: None, disable: None, pause: false }
    }
}

fn get_options(cx: &mut ExtCtxt, meta: &MetaItem) -> Options {
    fn meta_list_to_set(cx: &mut ExtCtxt, list: &[P<MetaItem>]) -> HashSet<String> {
        let mut v = HashSet::new();
        for item in list {
            match &item.node {
                &MetaWord(ref item_name) => { v.insert(item_name.to_string()); },
                &MetaList(ref item_name, _) | &MetaNameValue(ref item_name, _) =>
                    cx.span_warn(item.span, &format!("Invalid option {}", item_name))
            }
        }
        v
    }

    let mut options = Options::new();
    if let MetaList(_, ref v) = meta.node {
        for i in v {
            match &i.node {
                &MetaNameValue(ref name, ref s) => {
                    if *name == "prefix_enter" {
                        if let LitStr(ref new_prefix, _) = s.node {
                            options.prefix_enter = new_prefix.to_string();
                        }
                    } else if *name == "prefix_exit" {
                        if let LitStr(ref new_prefix, _) = s.node {
                            options.prefix_exit = new_prefix.to_string();
                        }
                    } else {
                        cx.span_warn(i.span, &format!("Invalid option {}", name));
                    }
                }
                &MetaList(ref name, ref list) =>  {
                    if *name == "enable" {
                        options.enable = Some(meta_list_to_set(cx, list));
                    } else if *name == "disable" {
                        options.disable = Some(meta_list_to_set(cx, list));
                    } else {
                        cx.span_warn(i.span, &format!("Invalid option {}", name));
                    }
                }
                &MetaWord(ref name) => {
                    if *name == "pause" {
                        options.pause = true;
                    } else {
                        cx.span_warn(i.span, &format!("Invalid option {}", name))
                    }
                }
            }
        }
    }
    if options.enable.is_some() && options.disable.is_some() {
        cx.span_err(meta.span, "Cannot use both enable and disable options with trace");
    }
    options
}

fn expand_impl(cx: &mut ExtCtxt, items: &[P<ImplItem>], options: Options) -> Vec<P<ImplItem>> {
    let mut new_items = vec!();
    for item in items.iter() {
        if let MethodImplItem(..) = item.node {
            let new_item = expand_impl_method(cx, options.clone(), item, false);
            new_items.push(P(ImplItem { node: new_item, attrs: vec!(), .. (**item).clone() }));
        }
    }
    new_items
}

fn expand_impl_method(cx: &mut ExtCtxt, options: Options, item: &ImplItem,
                      direct: bool) -> ImplItem_ {
    let ref name = item.ident.name.as_str();

    // If the attribute is not directly on this method, we filter by function names
    if !direct {
        match (&options.enable, &options.disable) {
            (&Some(ref s), &None) => if !s.contains(*name) { return item.node.clone() },
            (&None, &Some(ref s)) => if s.contains(*name) { return item.node.clone() },
            (&Some(_), &Some(_)) => unreachable!(),
            _ => ()
        }
    }

    if let &MethodImplItem(ref sig, ref block) = &item.node {
        let idents = arg_idents(&sig.decl);
        let new_block = new_block(cx, options, name, block.clone(), idents, direct);
        MethodImplItem(sig.clone(), new_block)
    } else {
        panic!("Expected method");
    }
}

fn expand_mod(cx: &mut ExtCtxt, m: &Mod, options: Options) -> Vec<P<Item>> {
    let mut new_items = vec!();
    let mut depth_correct = false;
    let mut depth_span = None;
    for i in m.items.iter() {
        match &i.node {
            &ItemFn(..) => {
                let new_item = expand_function(cx, options.clone(), i, false);
                new_items.push(cx.item(i.span, i.ident, i.attrs.clone(), new_item));
            }
            &ItemStatic(_, ref mut_, ref expr) => {
                let ref name = i.ident.name.as_str();
                if *name == "depth" {
                    depth_span = Some(i.span);
                    if let &MutMutable = mut_ {
                        if let ExprLit(ref lit) = expr.node {
                            if let LitInt(ref val, _) = lit.node {
                                if *val == 0 {
                                    depth_correct = true;
                                }
                            }
                        }
                    }
                }
                new_items.push((*i).clone());
            }
            &ItemImpl(safety, polarity, ref generics, ref traitref, ref ty, ref items) => {
                let new_impl_items = expand_impl(cx, &**items, options.clone());
                new_items.push(cx.item(i.span, i.ident, i.attrs.clone(),
                                       ItemImpl(safety, polarity, generics.clone(), traitref.clone(),
                                 ty.clone(), new_impl_items)));
            }
            _ => {
                new_items.push((*i).clone());
            }
        }
    }
    if let Some(sp) = depth_span {
        if !depth_correct {
            cx.span_err(sp, "A static variable with the name `depth` was found, but \
                             either the mutability, the type, or the inital value are \
                             incorrect");
        }
    } else {
        let depth_ident = Ident::new(intern("depth"));
        let u32_ident = Ident::new(intern("u32"));
        let ty = cx.ty_path(cx.path(codemap::DUMMY_SP, vec![u32_ident]));
        let item_ = cx.item_static(codemap::DUMMY_SP, depth_ident, ty, MutMutable,
                                   cx.expr_u32(codemap::DUMMY_SP, 0));
        new_items.push(item_);
    }

    new_items
}

fn expand_function(cx: &mut ExtCtxt, options: Options, item: &P<Item>, direct: bool) -> Item_ {
    let ref name = item.ident.name.as_str();

    // If the attribute is not directly on this method, we filter by function names
    if !direct {
        match (&options.enable, &options.disable) {
            (&Some(ref s), &None) | (&None, &Some(ref s)) =>
                if !s.contains(*name) { return item.node.clone() },
            (&Some(_), &Some(_)) => unreachable!(),
            _ => ()
        }
    }

    if let &ItemFn(ref decl, style, abi, ref generics, ref block) = &item.node {
        let idents = arg_idents(&**decl);
        let new_block = new_block(cx, options, name, block.clone(), idents, direct);
        ItemFn(decl.clone(), style, abi, generics.clone(), new_block)
    } else {
        panic!("Expected a function")
    }
}

fn arg_idents(decl: &FnDecl) -> Vec<Ident> {
    fn extract_idents(pat: &ast::Pat_, idents: &mut Vec<Ident>) {
        match pat {
            &ast::PatWild(_) | &ast::PatMac(_) | &ast::PatEnum(_, None) | &ast::PatLit(_)
                | &ast::PatRange(..) => (),
            &ast::PatIdent(_, sp, _) => if sp.node.as_str() != "self" { idents.push(sp.node) },
            &ast::PatEnum(_, Some(ref v)) | &ast::PatTup(ref v) => {
                for p in v {
                    extract_idents(&p.node, idents);
                }
            }
            &ast::PatStruct(_, ref v, _) => {
                for p in v {
                    extract_idents(&p.node.pat.node, idents);
                }
            }
            &ast::PatVec(ref v1, ref opt, ref v2) => {
                for p in v1 {
                    extract_idents(&p.node, idents);
                }
                if let &Some(ref p) = opt {
                    extract_idents(&p.node, idents);
                }
                for p in v2 {
                    extract_idents(&p.node, idents);
                }
            }
            &ast::PatBox(ref p) | &ast::PatRegion(ref p, _) => extract_idents(&p.node, idents),
        }
    }
    let mut idents = vec!();
    for arg in decl.inputs.iter() {
        extract_idents(&arg.pat.node, &mut idents);
    }
    idents
}

fn new_block(cx: &mut ExtCtxt, options: Options, name: &str, block: P<Block>,
             idents: Vec<Ident>, direct: bool) -> P<Block> {
    // If the attribute is on this method, we filter the arguments
    let idents = if direct {
        match (&options.enable, &options.disable) {
            (&Some(ref s), &None) =>
                idents.into_iter().filter(|x| s.contains(x.name.as_str())).collect(),
            (&None, &Some(ref s)) =>
                idents.into_iter().filter(|x| !s.contains(x.name.as_str())).collect(),
            (&Some(_), &Some(_)) => unreachable!(),
            _ => idents
        }
    } else {
        idents
    };

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

    let prefix_enter = &*options.prefix_enter;
    let prefix_exit = &*options.prefix_exit;
    let pause = options.pause;

    let new_block = quote_expr!(cx,
    unsafe {
        let mut s = String::new();
        (0..depth).map(|_| s.push(' ')).count();
        let args = format!($arg_fmt_str, $args);
        println!("{}{} Entering {}({})", s, $prefix_enter, $name, args);
        if $pause {
            use std::io::BufRead;
            let stdin = std::io::stdin();
            stdin.lock().lines().next();
        }
        depth += 1;
        let __trace_closure = move || $block;
        let __trace_result = __trace_closure();
        depth -= 1;
        println!("{}{} Exiting {} = {:?}", s, $prefix_exit, $name, __trace_result);
        if $pause {
            use std::io::BufRead;
            let stdin = std::io::stdin();
            stdin.lock().lines().next();
        }
        __trace_result
    });
    cx.block_expr(new_block)
}
