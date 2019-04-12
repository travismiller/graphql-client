use crate::deprecation::{DeprecationStatus, DeprecationStrategy};
use crate::objects::GqlObjectField;
use crate::query::QueryContext;
use crate::selection::*;
use failure::*;
use heck::{CamelCase, SnakeCase};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

pub(crate) fn render_object_field(
    field_name: &str,
    field_type: &TokenStream,
    description: Option<&str>,
    status: &DeprecationStatus,
    strategy: &DeprecationStrategy,
) -> TokenStream {
    #[allow(unused_assignments)]
    let mut deprecation = quote!();
    match (status, strategy) {
        // If the field is deprecated and we are denying usage, don't generate the
        // field in rust at all and short-circuit.
        (DeprecationStatus::Deprecated(_), DeprecationStrategy::Deny) => return quote!(),
        // Everything is allowed so there is nothing to do.
        (_, DeprecationStrategy::Allow) => deprecation = quote!(),
        // Current so there is nothing to do.
        (DeprecationStatus::Current, _) => deprecation = quote!(),
        // A reason was provided, translate it to a note.
        (DeprecationStatus::Deprecated(Some(reason)), DeprecationStrategy::Warn) => {
            deprecation = quote!(#[deprecated(note = #reason)])
        }
        // No reason provided, just mark as deprecated.
        (DeprecationStatus::Deprecated(None), DeprecationStrategy::Warn) => {
            deprecation = quote!(#[deprecated])
        }
    };

    let description = description.map(|s| quote!(#[doc = #s]));

    // List of keywords based on https://doc.rust-lang.org/grammar.html#keywords
    let reserved = &[
        "abstract", "alignof", "as", "become", "box", "break", "const", "continue", "crate", "do",
        "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in", "let", "loop",
        "macro", "match", "mod", "move", "mut", "offsetof", "override", "priv", "proc", "pub",
        "pure", "ref", "return", "Self", "self", "sizeof", "static", "struct", "super", "trait",
        "true", "type", "typeof", "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
    ];

    if reserved.contains(&field_name) {
        let name_ident = Ident::new(&format!("{}_", field_name), Span::call_site());
        return quote! {
            #description
            #deprecation
            #[serde(rename = #field_name)]
            pub #name_ident: #field_type
        };
    }

    let snake_case_name = field_name.to_snake_case();
    let rename = crate::shared::field_rename_annotation(&field_name, &snake_case_name);
    let name_ident = Ident::new(&snake_case_name, Span::call_site());

    quote!(#description #deprecation #rename pub #name_ident: #field_type)
}

pub(crate) fn field_impls_for_selection(
    fields: &[GqlObjectField<'_>],
    context: &QueryContext<'_, '_>,
    selection: &Selection<'_>,
    prefix: &str,
) -> Result<Vec<TokenStream>, failure::Error> {
    (&selection)
        .into_iter()
        .map(|selected| {
            if let SelectionItem::Field(selected) = selected {
                let name = &selected.name;
                let alias = selected.alias.as_ref().unwrap_or(name);

                let ty = fields
                    .iter()
                    .find(|f| &f.name == name)
                    .ok_or_else(|| format_err!("could not find field `{}`", name))?
                    .type_
                    .inner_name_str();
                let prefix = format!("{}{}", prefix.to_camel_case(), alias.to_camel_case());
                context.maybe_expand_field(&ty, &selected.fields, &prefix)
            } else {
                Ok(quote!())
            }
        })
        .collect()
}

pub(crate) fn response_fields_for_selection(
    type_name: &str,
    schema_fields: &[GqlObjectField<'_>],
    context: &QueryContext<'_, '_>,
    selection: &Selection<'_>,
    prefix: &str,
) -> Result<Vec<TokenStream>, failure::Error> {
    (&selection)
        .into_iter()
        .map(|item| match item {
            SelectionItem::Field(f) => {
                let name = &f.name;
                let alias = f.alias.as_ref().unwrap_or(name);

                let schema_field = &schema_fields
                    .iter()
                    .find(|field| &field.name == name)
                    .ok_or_else(|| {
                        format_err!(
                            "Could not find field `{}` on `{}`. Available fields: `{}`.",
                            *name,
                            type_name,
                            schema_fields
                                .iter()
                                .map(|ref field| &field.name)
                                .fold(String::new(), |mut acc, item| {
                                    acc.push_str(item);
                                    acc.push_str(", ");
                                    acc
                                })
                                .trim_end_matches(", ")
                        )
                    })?;
                let ty = schema_field.type_.to_rust(
                    context,
                    &format!("{}{}", prefix.to_camel_case(), alias.to_camel_case()),
                );

                Ok(render_object_field(
                    alias,
                    &ty,
                    schema_field.description.as_ref().cloned(),
                    &schema_field.deprecation,
                    &context.deprecation_strategy,
                ))
            }
            SelectionItem::FragmentSpread(fragment) => {
                let field_name =
                    Ident::new(&fragment.fragment_name.to_snake_case(), Span::call_site());
                context.require_fragment(&fragment.fragment_name);
                let fragment_from_context = context
                    .fragments
                    .get(&fragment.fragment_name)
                    .ok_or_else(|| format_err!("Unknown fragment: {}", &fragment.fragment_name))?;
                let type_name = Ident::new(&fragment.fragment_name, Span::call_site());
                let type_name = if fragment_from_context.is_recursive() {
                    quote!(Box<#type_name>)
                } else {
                    quote!(#type_name)
                };
                Ok(quote! {
                    #[serde(flatten)]
                    pub #field_name: #type_name
                })
            }
            SelectionItem::InlineFragment(_) => Err(format_err!(
                "unimplemented: inline fragment on object field"
            ))?,
        })
        .filter(|x| match x {
            // Remove empty fields so callers always know a field has some
            // tokens.
            Ok(f) => !f.is_empty(),
            Err(_) => true,
        })
        .collect()
}

/// Given the GraphQL schema name for an object/interface/input object field and
/// the equivalent rust name, produces a serde annotation to map them during
/// (de)serialization if it is necessary, otherwise an empty TokenStream.
pub(crate) fn field_rename_annotation(graphql_name: &str, rust_name: &str) -> TokenStream {
    if graphql_name != rust_name {
        quote!(#[serde(rename = #graphql_name)])
    } else {
        quote!()
    }
}
