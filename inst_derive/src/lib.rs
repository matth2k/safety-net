use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

/// Derive macro for the Instantiable trait.
///
/// This macro works with enums where each variant wraps a type that implements Instantiable.
/// It generates an implementation that delegates all trait methods to the wrapped type.
///
/// Use the `#[instantiable(constant)]` attribute on a variant to specify which variant
/// should be used for `from_constant()`.
///
/// # Example
///
///
/// #[derive(Debug, Clone, Instantiable)]
/// enum Cell {
///     #[instantiable(constant)]
///     Lut(Lut),
///     FlipFlop(FlipFlop),
///     Gate(Gate),
/// }
///
fn impl_instantiable_trait(ast: DeriveInput) -> TokenStream2 {
    let ident = ast.ident;

    // Only support enums
    let variants = match ast.data {
        Data::Enum(data_enum) => data_enum.variants,
        _ => {
            return syn::Error::new_spanned(ident, "Instantiable can only be derived for enums")
                .to_compile_error();
        }
    };

    // Extract variant names and find the constant variant
    let mut variant_names = Vec::new();
    let mut constant_variant: Option<syn::Ident> = None;

    for variant in variants {
        let variant_name = &variant.ident;

        // Validate that the variant has exactly one unnamed field
        match variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {}
            _ => {
                return syn::Error::new_spanned(
                    variant,
                    "Each enum variant must have exactly one unnamed field",
                )
                .to_compile_error();
            }
        }

        // Check for #[instantiable(constant)] attribute
        for attr in &variant.attrs {
            if attr.path().is_ident("instantiable") {
                let result = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("constant") {
                        if constant_variant.is_some() {
                            return Err(syn::Error::new_spanned(
                                attr,
                                "Only one variant can be marked with #[instantiable(constant)]",
                            ));
                        }
                        constant_variant = Some(variant_name.clone());
                        Ok(())
                    } else {
                        Err(meta.error("expected 'constant'"))
                    }
                });

                if let Err(err) = result {
                    return err.to_compile_error();
                }
            }
        }
        variant_names.push(variant_name.clone());
    }

    // Generate match arms for each method
    let get_name_arms = variant_names.iter().map(|v| {
        quote! { #ident::#v(inner) => inner.get_name() }
    });

    let get_input_ports_arms = variant_names.iter().map(|v| {
        quote! { #ident::#v(inner) => inner.get_input_ports().into_iter().collect::<Vec<_>>() }
    });

    let get_output_ports_arms = variant_names.iter().map(|v| {
        quote! { #ident::#v(inner) => inner.get_output_ports().into_iter().collect::<Vec<_>>() }
    });

    let has_parameter_arms = variant_names.iter().map(|v| {
        quote! { #ident::#v(inner) => inner.has_parameter(id) }
    });

    let get_parameter_arms = variant_names.iter().map(|v| {
        quote! { #ident::#v(inner) => inner.get_parameter(id) }
    });

    let set_parameter_arms = variant_names.iter().map(|v| {
        quote! { #ident::#v(inner) => inner.set_parameter(id, val) }
    });

    let parameters_arms = variant_names.iter().map(|v| {
        quote! { #ident::#v(inner) => inner.parameters().collect::<Vec<_>>().into_iter() }
    });

    let get_constant_arms = variant_names.iter().map(|v| {
        quote! { #ident::#v(inner) => inner.get_constant() }
    });

    let is_seq_arms = variant_names.iter().map(|v| {
        quote! { #ident::#v(inner) => inner.is_seq() }
    });

    // Generate from_constant implementation based on the marked variant
    let from_constant_impl = if let Some(const_var) = constant_variant {
        quote! {
            fn from_constant(val: Logic) -> Option<Self> {
                if (val == Logic::True) || (val == Logic::False) {
                    return #const_var::from_constant(val).map(#ident::#const_var);
                } else {
                    return None;
                }
            }
        }
    } else {
        quote! {
            fn from_constant(_val: Logic) -> Option<Self> {
                None
            }
        }
    };

    // Generate the implementation
    quote! {
        impl Instantiable for #ident {
            fn get_name(&self) -> &Identifier {
                match self {
                    #(#get_name_arms),*
                }
            }

            fn get_input_ports(&self) -> impl IntoIterator<Item = &Net> {
                match self {
                    #(#get_input_ports_arms),*
                }
            }

            fn get_output_ports(&self) -> impl IntoIterator<Item = &Net> {
                match self {
                    #(#get_output_ports_arms),*
                }
            }

            fn has_parameter(&self, id: &Identifier) -> bool {
                match self {
                    #(#has_parameter_arms),*
                }
            }

            fn get_parameter(&self, id: &Identifier) -> Option<Parameter> {
                match self {
                    #(#get_parameter_arms),*
                }
            }

            fn set_parameter(&mut self, id: &Identifier, val: Parameter) -> Option<Parameter> {
                match self {
                    #(#set_parameter_arms),*
                }
            }

            fn parameters(&self) -> impl Iterator<Item = (Identifier, Parameter)> {
                match self {
                    #(#parameters_arms),*
                }
            }

            #from_constant_impl

            fn get_constant(&self) -> Option<Logic> {
                match self {
                    #(#get_constant_arms),*
                }
            }

            fn is_seq(&self) -> bool {
                match self {
                    #(#is_seq_arms),*
                }
            }
        }
    }
}

#[proc_macro_derive(Instantiable, attributes(instantiable))]
pub fn inst_derive_macro(item: TokenStream) -> TokenStream {
    let ast = syn::parse(item).unwrap();
    TokenStream::from(impl_instantiable_trait(ast))
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    /// Helper function to normalize TokenStreams for comparison
    /// This removes whitespace differences and formats consistently
    fn normalize_tokenstream(ts: TokenStream2) -> String {
        // Parse and re-format to normalize
        syn::parse2::<syn::File>(ts.clone())
            .map(|file| prettyplease::unparse(&file))
            .unwrap_or_else(|_| {
                // If it's not a complete file, try as an item
                syn::parse2::<syn::Item>(ts.clone())
                    .map(|item| quote!(#item).to_string())
                    .unwrap_or_else(|_| ts.to_string())
            })
    }

    /// Compare two TokenStreams, ignoring whitespace differences
    fn assert_tokens_eq(actual: TokenStream2, expected: TokenStream2) {
        let actual_normalized = normalize_tokenstream(actual.clone());
        let expected_normalized = normalize_tokenstream(expected.clone());

        if actual_normalized != expected_normalized {
            eprintln!("=== ACTUAL ===");
            eprintln!("{}", actual_normalized);
            eprintln!("\n=== EXPECTED ===");
            eprintln!("{}", expected_normalized);
            eprintln!("\n=== DIFFERENCE ===");

            // Show a simple diff
            let actual_lines: Vec<&str> = actual_normalized.lines().collect();
            let expected_lines: Vec<&str> = expected_normalized.lines().collect();

            for (i, (a, e)) in actual_lines.iter().zip(expected_lines.iter()).enumerate() {
                if a != e {
                    eprintln!("Line {}: ", i + 1);
                    eprintln!("  Actual:   {}", a);
                    eprintln!("  Expected: {}", e);
                }
            }

            panic!("TokenStreams do not match");
        }
    }

    #[test]
    fn test_simple_enum_tokenstream() {
        let input: DeriveInput = parse_quote! {
            #[derive(Instantiable)]
            enum SimpleCell {
                Lut(Lut),
                #[instantiable(constant)]
                Gate(Gate),
            }
        };

        let output = impl_instantiable_trait(input);

        let expected = quote! {
            impl Instantiable for SimpleCell {
                fn get_name(&self) -> &Identifier {
                    match self {
                        SimpleCell::Lut(inner) => inner.get_name(),
                        SimpleCell::Gate(inner) => inner.get_name()
                    }
                }

                fn get_input_ports(&self) -> impl IntoIterator<Item = &Net> {
                    match self {
                        SimpleCell::Lut(inner) => inner.get_input_ports().into_iter().collect::<Vec<_>>(),
                        SimpleCell::Gate(inner) => inner.get_input_ports().into_iter().collect::<Vec<_>>()
                    }
                }

                fn get_output_ports(&self) -> impl IntoIterator<Item = &Net> {
                    match self {
                        SimpleCell::Lut(inner) => inner.get_output_ports().into_iter().collect::<Vec<_>>(),
                        SimpleCell::Gate(inner) => inner.get_output_ports().into_iter().collect::<Vec<_>>()
                    }
                }

                fn has_parameter(&self, id: &Identifier) -> bool {
                    match self {
                        SimpleCell::Lut(inner) => inner.has_parameter(id),
                        SimpleCell::Gate(inner) => inner.has_parameter(id)
                    }
                }

                fn get_parameter(&self, id: &Identifier) -> Option<Parameter> {
                    match self {
                        SimpleCell::Lut(inner) => inner.get_parameter(id),
                        SimpleCell::Gate(inner) => inner.get_parameter(id)
                    }
                }

                fn set_parameter(&mut self, id: &Identifier, val: Parameter) -> Option<Parameter> {
                    match self {
                        SimpleCell::Lut(inner) => inner.set_parameter(id, val),
                        SimpleCell::Gate(inner) => inner.set_parameter(id, val)
                    }
                }

                fn parameters(&self) -> impl Iterator<Item = (Identifier, Parameter)> {
                    match self {
                        SimpleCell::Lut(inner) => inner.parameters().collect::<Vec<_>>().into_iter(),
                        SimpleCell::Gate(inner) => inner.parameters().collect::<Vec<_>>().into_iter()
                    }
                }

                fn from_constant(val: Logic) -> Option<Self> {
                    if (val == Logic::True) || (val == Logic::False) {
                        return Gate::from_constant(val).map(SimpleCell::Gate);
                    } else {
                        return None;
                    }
                }

                fn get_constant(&self) -> Option<Logic> {
                    match self {
                        SimpleCell::Lut(inner) => inner.get_constant(),
                        SimpleCell::Gate(inner) => inner.get_constant()
                    }
                }

                fn is_seq(&self) -> bool {
                    match self {
                        SimpleCell::Lut(inner) => inner.is_seq(),
                        SimpleCell::Gate(inner) => inner.is_seq()
                    }
                }
            }
        };

        assert_tokens_eq(output, expected);
    }

    #[test]
    fn test_no_constant_variant_tokenstream() {
        let input: DeriveInput = parse_quote! {
            #[derive(Instantiable)]
            enum SimpleCell {
                Lut(Lut),
                Gate(Gate),
            }
        };

        let output = impl_instantiable_trait(input);

        let expected = quote! {
            impl Instantiable for SimpleCell {
                fn get_name(&self) -> &Identifier {
                    match self {
                        SimpleCell::Lut(inner) => inner.get_name(),
                        SimpleCell::Gate(inner) => inner.get_name()
                    }
                }

                fn get_input_ports(&self) -> impl IntoIterator<Item = &Net> {
                    match self {
                        SimpleCell::Lut(inner) => inner.get_input_ports().into_iter().collect::<Vec<_>>(),
                        SimpleCell::Gate(inner) => inner.get_input_ports().into_iter().collect::<Vec<_>>()
                    }
                }

                fn get_output_ports(&self) -> impl IntoIterator<Item = &Net> {
                    match self {
                        SimpleCell::Lut(inner) => inner.get_output_ports().into_iter().collect::<Vec<_>>(),
                        SimpleCell::Gate(inner) => inner.get_output_ports().into_iter().collect::<Vec<_>>()
                    }
                }

                fn has_parameter(&self, id: &Identifier) -> bool {
                    match self {
                        SimpleCell::Lut(inner) => inner.has_parameter(id),
                        SimpleCell::Gate(inner) => inner.has_parameter(id)
                    }
                }

                fn get_parameter(&self, id: &Identifier) -> Option<Parameter> {
                    match self {
                        SimpleCell::Lut(inner) => inner.get_parameter(id),
                        SimpleCell::Gate(inner) => inner.get_parameter(id)
                    }
                }

                fn set_parameter(&mut self, id: &Identifier, val: Parameter) -> Option<Parameter> {
                    match self {
                        SimpleCell::Lut(inner) => inner.set_parameter(id, val),
                        SimpleCell::Gate(inner) => inner.set_parameter(id, val)
                    }
                }

                fn parameters(&self) -> impl Iterator<Item = (Identifier, Parameter)> {
                    match self {
                        SimpleCell::Lut(inner) => inner.parameters().collect::<Vec<_>>().into_iter(),
                        SimpleCell::Gate(inner) => inner.parameters().collect::<Vec<_>>().into_iter()
                    }
                }

                fn from_constant(_val: Logic) -> Option<Self> {
                    None
                }

                fn get_constant(&self) -> Option<Logic> {
                    match self {
                        SimpleCell::Lut(inner) => inner.get_constant(),
                        SimpleCell::Gate(inner) => inner.get_constant()
                    }
                }

                fn is_seq(&self) -> bool {
                    match self {
                        SimpleCell::Lut(inner) => inner.is_seq(),
                        SimpleCell::Gate(inner) => inner.is_seq()
                    }
                }
            }
        };

        assert_tokens_eq(output, expected);
    }

    #[test]
    fn test_two_constant_variants_error() {
        let input: DeriveInput = parse_quote! {
            #[derive(Instantiable)]
            enum SimpleCell {
                #[instantiable(constant)]
                Lut(Lut),
                #[instantiable(constant)]
                Gate(Gate),
            }
        };

        let output = impl_instantiable_trait(input);

        let expected_error_msg = "Only one variant can be marked with #[instantiable(constant)]";

        let output_str = output.to_string();
        assert!(
            output_str.contains(expected_error_msg),
            "Expected error message not found. Output was:\n{}",
            output_str
        );
    }
}
