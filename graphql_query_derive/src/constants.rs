use proc_macro2::{Ident, Span};

pub const TYPENAME_FIELD: &'static str = "__typename";

pub fn string_type() -> Ident {
    Ident::new("String", Span::call_site())
}
