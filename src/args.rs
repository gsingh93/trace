use std::collections::HashSet;

use syn::{self, spanned::Spanned};

pub(crate) struct Args {
    pub(crate) prefix_enter: String,
    pub(crate) prefix_exit: String,
    pub(crate) filter: Filter,
    pub(crate) pause: bool,
    pub(crate) pretty: bool,
    pub(crate) logging: bool,
}

pub(crate) enum Filter {
    None,
    Enable(HashSet<proc_macro2::Ident>),
    Disable(HashSet<proc_macro2::Ident>),
}

const DEFAULT_PREFIX_ENTER: &str = "[+]";
const DEFAULT_PREFIX_EXIT: &str = "[-]";
const DEFAULT_PAUSE: bool = false;
const DEFAULT_PRETTY: bool = false;
const DEFAULT_LOGGING: bool = false;

impl Args {
    pub(crate) fn from_raw_args(raw_args: syn::AttributeArgs) -> Result<Self, Vec<syn::Error>> {
        // Different types of arguments accepted by `#[trace]`;
        // spans are needed for friendly error reporting of duplicate arguments
        enum Arg {
            PrefixEnter(proc_macro2::Span, String),
            PrefixExit(proc_macro2::Span, String),
            Enable(proc_macro2::Span, HashSet<proc_macro2::Ident>),
            Disable(proc_macro2::Span, HashSet<proc_macro2::Ident>),
            Pause(proc_macro2::Span, bool),
            Pretty(proc_macro2::Span, bool),
            Logging(proc_macro2::Span, bool),
        }

        // Parse arguments
        let args_res = raw_args.into_iter().map(|nested_meta| match nested_meta {
            syn::NestedMeta::Meta(ref meta) => {
                enum ArgName {
                    PrefixEnter,
                    PrefixExit,
                    Enable,
                    Disable,
                    Pause,
                    Pretty,
                    Logging,
                }

                let ident = &meta.path().segments.first().unwrap().ident;
                let arg_name = match ident.to_string().as_str() {
                    "prefix_enter" => ArgName::PrefixEnter,
                    "prefix_exit" => ArgName::PrefixExit,
                    "enable" => ArgName::Enable,
                    "disable" => ArgName::Disable,
                    "pause" => ArgName::Pause,
                    "pretty" => ArgName::Pretty,
                    "logging" => ArgName::Logging,
                    _ => {
                        return Err(vec![syn::Error::new_spanned(
                            ident.clone(),
                            format_args!("unknown attribute argument `{}`", ident),
                        )])
                    }
                };

                let prefix_enter_type_error = || {
                    vec![syn::Error::new_spanned(
                        ident.clone(),
                        "`prefix_enter` requires a string value",
                    )]
                };
                let prefix_exit_type_error = || {
                    vec![syn::Error::new_spanned(
                        ident.clone(),
                        "`prefix_exit` requires a string value",
                    )]
                };
                let enable_type_error = || {
                    vec![syn::Error::new_spanned(
                        ident.clone(),
                        "`enable` requires a list of meta words",
                    )]
                };
                let disable_type_error = || {
                    vec![syn::Error::new_spanned(
                        ident.clone(),
                        "`disable` requires a list of meta words",
                    )]
                };
                let pause_type_error = || {
                    vec![syn::Error::new_spanned(
                        ident.clone(),
                        "`pause` must be a meta word",
                    )]
                };
                let pretty_type_error = || {
                    vec![syn::Error::new_spanned(
                        ident.clone(),
                        "`pretty` must be a meta word",
                    )]
                };
                let logging_type_error = || {
                    vec![syn::Error::new_spanned(
                        ident.clone(),
                        "`logging` must be a meta word",
                    )]
                };

                match *meta {
                    syn::Meta::Path(_) => match arg_name {
                        ArgName::Pause => Ok(Arg::Pause(meta.span(), true)),
                        ArgName::Pretty => Ok(Arg::Pretty(meta.span(), true)),
                        ArgName::Logging => Ok(Arg::Logging(meta.span(), true)),

                        ArgName::PrefixEnter => Err(prefix_enter_type_error()),
                        ArgName::PrefixExit => Err(prefix_exit_type_error()),
                        ArgName::Enable => Err(enable_type_error()),
                        ArgName::Disable => Err(disable_type_error()),
                    },
                    syn::Meta::List(syn::MetaList { ref nested, .. }) => match arg_name {
                        ArgName::Enable => {
                            let mut idents = HashSet::new();
                            let mut other_nested_meta_errors = Vec::new();

                            nested.iter().for_each(|nested_meta| match *nested_meta {
                                syn::NestedMeta::Meta(syn::Meta::Path(ref path))
                                    if path.segments.len() == 1 =>
                                {
                                    idents.insert(path.segments.first().unwrap().ident.clone());
                                }
                                _ => other_nested_meta_errors.push(syn::Error::new_spanned(
                                    nested_meta,
                                    "`enable` must contain single ident paths only",
                                )),
                            });

                            if other_nested_meta_errors.is_empty() {
                                Ok(Arg::Enable(meta.span(), idents))
                            } else {
                                Err(other_nested_meta_errors)
                            }
                        }
                        ArgName::Disable => {
                            let mut idents = HashSet::new();
                            let mut other_nested_meta_errors = Vec::new();

                            nested.iter().for_each(|nested_meta| match *nested_meta {
                                syn::NestedMeta::Meta(syn::Meta::Path(ref path))
                                    if path.segments.len() == 1 =>
                                {
                                    idents.insert(path.segments.first().unwrap().ident.clone());
                                }
                                _ => other_nested_meta_errors.push(syn::Error::new_spanned(
                                    nested_meta,
                                    "`disable` must contain single ident paths only",
                                )),
                            });

                            if other_nested_meta_errors.is_empty() {
                                Ok(Arg::Disable(meta.span(), idents))
                            } else {
                                Err(other_nested_meta_errors)
                            }
                        }

                        ArgName::PrefixEnter => Err(prefix_enter_type_error()),
                        ArgName::PrefixExit => Err(prefix_exit_type_error()),
                        ArgName::Pause => Err(pause_type_error()),
                        ArgName::Pretty => Err(pretty_type_error()),
                        ArgName::Logging => Err(logging_type_error()),
                    },
                    syn::Meta::NameValue(syn::MetaNameValue { ref lit, .. }) => match arg_name {
                        ArgName::PrefixEnter => match *lit {
                            syn::Lit::Str(ref lit_str) => {
                                Ok(Arg::PrefixEnter(meta.span(), lit_str.value()))
                            }
                            _ => Err(vec![syn::Error::new_spanned(
                                lit,
                                "`prefix_enter` must have a string value",
                            )]),
                        },
                        ArgName::PrefixExit => match *lit {
                            syn::Lit::Str(ref lit_str) => {
                                Ok(Arg::PrefixExit(meta.span(), lit_str.value()))
                            }
                            _ => Err(vec![syn::Error::new_spanned(
                                lit,
                                "`prefix_exit` must have a string value",
                            )]),
                        },

                        ArgName::Enable => Err(enable_type_error()),
                        ArgName::Disable => Err(disable_type_error()),
                        ArgName::Pause => Err(pause_type_error()),
                        ArgName::Pretty => Err(pretty_type_error()),
                        ArgName::Logging => Err(logging_type_error()),
                    },
                }
            }
            syn::NestedMeta::Lit(_) => Err(vec![syn::Error::new_spanned(
                nested_meta,
                "literal attribute not allowed",
            )]),
        });

        let mut prefix_enter_args = vec![];
        let mut prefix_exit_args = vec![];
        let mut enable_args = vec![];
        let mut disable_args = vec![];
        let mut pause_args = vec![];
        let mut pretty_args = vec![];
        let mut logging_args = vec![];
        let mut errors = vec![];

        // Group arguments of the same type and errors
        for arg_res in args_res {
            match arg_res {
                Ok(arg) => match arg {
                    Arg::PrefixEnter(span, s) => prefix_enter_args.push((span, s)),
                    Arg::PrefixExit(span, s) => prefix_exit_args.push((span, s)),
                    Arg::Enable(span, idents) => enable_args.push((span, idents)),
                    Arg::Disable(span, idents) => disable_args.push((span, idents)),
                    Arg::Pause(span, b) => pause_args.push((span, b)),
                    Arg::Pretty(span, b) => pretty_args.push((span, b)),
                    Arg::Logging(span, b) => logging_args.push((span, b)),
                },
                Err(es) => errors.extend(es),
            }
        }

        // Report duplicates
        if prefix_enter_args.len() >= 2 {
            errors.extend(
                prefix_enter_args
                    .iter()
                    .map(|(span, _)| syn::Error::new(*span, "duplicate `prefix_enter`")),
            );
        }
        if prefix_exit_args.len() >= 2 {
            errors.extend(
                prefix_exit_args
                    .iter()
                    .map(|(span, _)| syn::Error::new(*span, "duplicate `prefix_exit`")),
            );
        }
        if enable_args.len() >= 2 {
            errors.extend(
                enable_args
                    .iter()
                    .map(|(span, _)| syn::Error::new(*span, "duplicate `enable`")),
            );
        }
        if disable_args.len() >= 2 {
            errors.extend(
                disable_args
                    .iter()
                    .map(|(span, _)| syn::Error::new(*span, "duplicate `disable`")),
            );
        }
        if pause_args.len() >= 2 {
            errors.extend(
                pause_args
                    .iter()
                    .map(|(span, _)| syn::Error::new(*span, "duplicate `pause`")),
            );
        }
        if pretty_args.len() >= 2 {
            errors.extend(
                pretty_args
                    .iter()
                    .map(|(span, _)| syn::Error::new(*span, "duplicate `pretty`")),
            );
        }
        if logging_args.len() >= 2 {
            errors.extend(
                logging_args
                    .iter()
                    .map(|(span, _)| syn::Error::new(*span, "duplicate `logging`")),
            );
        }

        // Report the presence of mutually exclusive arguments
        if enable_args.len() == 1 && disable_args.len() == 1 {
            errors.push(syn::Error::new(
                enable_args[0].0,
                "cannot have both `enable` and `disable`",
            ));
            errors.push(syn::Error::new(
                disable_args[0].0,
                "cannot have both `enable` and `disable`",
            ));
        }

        if errors.is_empty() {
            macro_rules! first_no_span {
                ($iterable:expr) => {
                    $iterable.into_iter().next().map(|(_, elem)| elem)
                };
            }

            let prefix_enter = first_no_span!(prefix_enter_args)
                .unwrap_or_else(|| DEFAULT_PREFIX_ENTER.to_owned());
            let prefix_exit =
                first_no_span!(prefix_exit_args).unwrap_or_else(|| DEFAULT_PREFIX_EXIT.to_owned());
            let filter = match (first_no_span!(enable_args), first_no_span!(disable_args)) {
                (None, None) => Filter::None,
                (Some(idents), None) => Filter::Enable(idents),
                (None, Some(idents)) => Filter::Disable(idents),
                (Some(_), Some(_)) => unreachable!(),
            };
            let pause = first_no_span!(pause_args).unwrap_or(DEFAULT_PAUSE);
            let pretty = first_no_span!(pretty_args).unwrap_or(DEFAULT_PRETTY);
            let logging = first_no_span!(logging_args).unwrap_or(DEFAULT_LOGGING);

            Ok(Self {
                prefix_enter,
                prefix_exit,
                filter,
                pause,
                pretty,
                logging,
            })
        } else {
            Err(errors)
        }
    }
}
