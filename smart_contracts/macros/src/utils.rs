pub(crate) fn compute_blake2b256(bytes: &[u8]) -> [u8; 32] {
    let mut context = blake2_rfc::blake2b::Blake2b::new(32);
    context.update(bytes);
    context.finalize().as_bytes().try_into().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize() {
        let tuple = syn::parse_quote! {
            (u8, u32, Option<(String, u64)>)
        };
        assert_eq!(
            super::sanitized_type_name(&tuple),
            "(u8,u32,Option<(String,u64)>)".to_string()
        );

        let unsigned_32 = syn::parse_quote! {
            u32
        };
        assert_eq!(super::sanitized_type_name(&unsigned_32), "u32".to_string());

        let result_ty = syn::parse_quote! {
            Result<u32, Error>
        };
        assert_eq!(
            super::sanitized_type_name(&result_ty),
            "Result<u32,Error>".to_string()
        );
    }
    #[test]
    fn test_selector_preimage() {
        let foo_function = syn::parse_quote! {
            fn foo_function(arg:(u8, u32)) -> u32
        };
        let my_function = syn::parse_quote! {
            fn my_function(arg1: u32, arg2: String) -> u64
        };
        let my_function_with_receiver = syn::parse_quote! {
            fn my_function_with_receiver(&mut self) -> String
        };
        assert_eq!(selector_preimage(&foo_function), "foo_function((u8,u32))");
        assert_eq!(selector_preimage(&my_function), "my_function(u32,String)");
        assert_eq!(
            selector_preimage(&my_function_with_receiver),
            "my_function_with_receiver()"
        );
    }
}
