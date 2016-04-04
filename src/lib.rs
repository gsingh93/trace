#![feature(quote, plugin_registrar, rustc_private, slice_concat_ext)]

extern crate syntax;
extern crate rustc_plugin;

use std::slice::SliceConcatExt;
use std::collections::HashSet;

use rustc_plugin::Registry;

use syntax::ptr::P;
use syntax::ast::{self, Item, ItemKind, MetaItem, Block, Ident, TokenTree, FnDecl, ImplItem,
                  ImplItemKind, PatKind};
use syntax::ast::ExprKind::Lit;
use syntax::ast::ItemKind::{Fn, Mod, Impl, Static};
use syntax::ast::Mutability::Mutable;
use syntax::ast::MetaItemKind::{List, NameValue, Word};
use syntax::ast::LitKind::{Str, Int};
use syntax::codemap::{self, Span};
use syntax::ext::base::{ExtCtxt, Annotatable};
use syntax::ext::base::SyntaxExtension::MultiModifier;
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
                &Fn(..) => {
                    let new_item = expand_function(cx, options, &item, true);
                    cx.item(item.span, item.ident, item.attrs.clone(), new_item)
                }
                &Mod(ref m) => {
                    let new_items = expand_mod(cx, m, options);
                    cx.item(item.span, item.ident, item.attrs.clone(),
                            Mod(ast::Mod { inner: m.inner, items: new_items }))
                }
                &Impl(safety, polarity, ref generics, ref traitref, ref ty, ref items) => {
                    let new_items = expand_impl(cx, &*items, options);
                    cx.item(item.span, item.ident, item.attrs.clone(),
                            Impl(safety, polarity, generics.clone(), traitref.clone(),
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
                &Word(ref item_name) => { v.insert(item_name.to_string()); },
                &List(ref item_name, _) | &NameValue(ref item_name, _) =>
                    cx.span_warn(item.span, &format!("Invalid option {}", item_name))
            }
        }
        v
    }

    let mut options = Options::new();
    if let List(_, ref v) = meta.node {
        for i in v {
            match &i.node {
                &NameValue(ref name, ref s) => {
                    if *name == "prefix_enter" {
                        if let Str(ref new_prefix, _) = s.node {
                            options.prefix_enter = new_prefix.to_string();
                        }
                    } else if *name == "prefix_exit" {
                        if let Str(ref new_prefix, _) = s.node {
                            options.prefix_exit = new_prefix.to_string();
                        }
                    } else {
                        cx.span_warn(i.span, &format!("Invalid option {}", name));
                    }
                }
                &List(ref name, ref list) =>  {
                    if *name == "enable" {
                        options.enable = Some(meta_list_to_set(cx, list));
                    } else if *name == "disable" {
                        options.disable = Some(meta_list_to_set(cx, list));
                    } else {
                        cx.span_warn(i.span, &format!("Invalid option {}", name));
                    }
                }
                &Word(ref name) => {
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

fn expand_impl(cx: &mut ExtCtxt, items: &[ImplItem], options: Options) -> Vec<ImplItem> {
    let mut new_items = vec!();
    for item in items.iter() {
        if let ImplItemKind::Method(..) = item.node {
            let new_item = expand_impl_method(cx, options.clone(), item, false);
            new_items.push(ImplItem { node: new_item, attrs: vec!(), .. (*item).clone() });
        }
    }
    new_items
}

fn expand_impl_method(cx: &mut ExtCtxt, options: Options, item: &ImplItem,
                      direct: bool) -> ImplItemKind {
    let name = &*item.ident.name.as_str();

    // If the attribute is not directly on this method, we filter by function names
    if !direct {
        match (&options.enable, &options.disable) {
            (&Some(ref s), &None) => if !s.contains(name) { return item.node.clone() },
            (&None, &Some(ref s)) => if s.contains(name) { return item.node.clone() },
            (&Some(_), &Some(_)) => unreachable!(),
            _ => ()
        }
    }

    if let &ImplItemKind::Method(ref sig, ref block) = &item.node {
        let idents = arg_idents(cx, &sig.decl);
        let new_block = new_block(cx, options, name, block.clone(), idents, direct);
        ImplItemKind::Method(sig.clone(), new_block)
    } else {
        panic!("Expected method");
    }
}

fn expand_mod(cx: &mut ExtCtxt, m: &ast::Mod, options: Options) -> Vec<P<Item>> {
    let mut new_items = vec!();
    let mut depth_correct = false;
    let mut depth_span = None;
    for i in m.items.iter() {
        match &i.node {
            &Fn(..) => {
                let new_item = expand_function(cx, options.clone(), i, false);
                new_items.push(cx.item(i.span, i.ident, i.attrs.clone(), new_item));
            }
            &Static(_, ref mut_, ref expr) => {
                let ref name = i.ident.name.as_str();
                if *name == "depth" {
                    depth_span = Some(i.span);
                    if let &Mutable = mut_ {
                        if let Lit(ref lit) = expr.node {
                            if let Int(ref val, _) = lit.node {
                                if *val == 0 {
                                    depth_correct = true;
                                }
                            }
                        }
                    }
                }
                new_items.push((*i).clone());
            }
            &Impl(safety, polarity, ref generics, ref traitref, ref ty, ref items) => {
                let new_impl_items = expand_impl(cx, &**items, options.clone());
                new_items.push(cx.item(i.span, i.ident, i.attrs.clone(),
                                       Impl(safety, polarity, generics.clone(), traitref.clone(),
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
        let depth_ident = Ident::with_empty_ctxt(intern("depth"));
        let u32_ident = Ident::with_empty_ctxt(intern("u32"));
        let ty = cx.ty_path(cx.path(codemap::DUMMY_SP, vec![u32_ident]));
        let item_ = cx.item_static(codemap::DUMMY_SP, depth_ident, ty, Mutable,
                                   cx.expr_u32(codemap::DUMMY_SP, 0));
        new_items.push(item_);
    }

    new_items
}

fn expand_function(cx: &mut ExtCtxt, options: Options, item: &P<Item>, direct: bool) -> ItemKind {
    let ref name = &*item.ident.name.as_str();

    // If the attribute is not directly on this method, we filter by function names
    if !direct {
        match (&options.enable, &options.disable) {
            (&Some(ref s), &None) | (&None, &Some(ref s)) =>
                if !s.contains(*name) { return item.node.clone() },
            (&Some(_), &Some(_)) => unreachable!(),
            _ => ()
        }
    }

    if let &Fn(ref decl, style, constness, abi, ref generics, ref block) = &item.node {
        let idents = arg_idents(cx, &**decl);
        let new_block = new_block(cx, options, name, block.clone(), idents, direct);
        Fn(decl.clone(), style, constness, abi, generics.clone(), new_block)
    } else {
        panic!("Expected a function")
    }
}

fn arg_idents(cx: &mut ExtCtxt, decl: &FnDecl) -> Vec<Ident> {
    fn extract_idents(cx: &mut ExtCtxt, pat: &ast::PatKind, idents: &mut Vec<Ident>) {
        match pat {
            &PatKind::Wild | &PatKind::TupleStruct(_, None) | &PatKind::Lit(_)
                | &PatKind::Range(..) | &PatKind::Path(..) | &PatKind::QPath(..) => (),
            &PatKind::Ident(_, sp, _) => {
                if &*sp.node.name.as_str() != "self" {
                    idents.push(sp.node);
                }
            },
            &PatKind::TupleStruct(_, Some(ref v)) | &PatKind::Tup(ref v) => {
                for p in v {
                    extract_idents(cx, &p.node, idents);
                }
            }
            &PatKind::Struct(_, ref v, _) => {
                for p in v {
                    extract_idents(cx, &p.node.pat.node, idents);
                }
            }
            &PatKind::Vec(ref v1, ref opt, ref v2) => {
                for p in v1 {
                    extract_idents(cx, &p.node, idents);
                }
                if let &Some(ref p) = opt {
                    extract_idents(cx, &p.node, idents);
                }
                for p in v2 {
                    extract_idents(cx, &p.node, idents);
                }
            }
            &PatKind::Box(ref p) | &PatKind::Ref(ref p, _) => extract_idents(cx, &p.node, idents),
            &PatKind::Mac(ref m) => {
                let sp = m.node.path.span;
                cx.span_err(sp, "trace does not work on functions with macros in the arg list");
            }
        }
    }
    let mut idents = vec!();
    for arg in decl.inputs.iter() {
        extract_idents(cx, &arg.pat.node, &mut idents);
    }
    idents
}

fn new_block(cx: &mut ExtCtxt, options: Options, name: &str, block: P<Block>,
             idents: Vec<Ident>, direct: bool) -> P<Block> {
    // If the attribute is on this method, we filter the arguments
    let idents = if direct {
        match (&options.enable, &options.disable) {
            (&Some(ref s), &None) =>
                idents.into_iter().filter(|x| s.contains(&*x.name.as_str())).collect(),
            (&None, &Some(ref s)) =>
                idents.into_iter().filter(|x| !s.contains(&*x.name.as_str())).collect(),
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
        .join(&token::Comma)
        .into_iter()
        .map(|t| TokenTree::Token(codemap::DUMMY_SP, t))
        .collect();

    let mut arg_fmt = vec!();
    for ident in idents.iter() {
        arg_fmt.push(format!("{}: {{:?}}", ident))
    }
    let arg_fmt_str = &*arg_fmt.join(", ");

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
            use std::io::{BufRead, stdin};
            let stdin = stdin();
            stdin.lock().lines().next();
        }
        depth += 1;
        let mut __trace_closure = move || $block;
        let __trace_result = __trace_closure();
        depth -= 1;
        println!("{}{} Exiting {} = {:?}", s, $prefix_exit, $name, __trace_result);
        if $pause {
            use std::io::{BufRead, stdin};
            let stdin = stdin();
            stdin.lock().lines().next();
        }
        __trace_result
    });
    cx.block_expr(new_block)
}
