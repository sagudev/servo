/// Generated JS-Rust bindings.
#[allow(missing_docs, non_snake_case)]
pub mod codegen {
    #[allow(dead_code, crown::unrooted_must_root)]
    pub mod Bindings {
        include!(concat!(env!("OUT_DIR"), "/Bindings/mod.rs"));
    }
    pub mod InterfaceObjectMap {
        include!(concat!(env!("OUT_DIR"), "/InterfaceObjectMap.rs"));
    }
    #[allow(dead_code, unused_imports, clippy::enum_variant_names)]
    pub mod InheritTypes {
        include!(concat!(env!("OUT_DIR"), "/InheritTypes.rs"));
    }
    #[allow(clippy::upper_case_acronyms)]
    pub mod PrototypeList {
        include!(concat!(env!("OUT_DIR"), "/PrototypeList.rs"));
    }
    pub mod RegisterBindings {
        include!(concat!(env!("OUT_DIR"), "/RegisterBindings.rs"));
    }
    #[allow(
        non_camel_case_types,
        unused_imports,
        unused_variables,
        clippy::large_enum_variant,
        clippy::upper_case_acronyms,
        clippy::enum_variant_names
    )]
    pub mod UnionTypes {
        include!(concat!(env!("OUT_DIR"), "/UnionTypes.rs"));
    }
}
