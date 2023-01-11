//! [oo_bindgen](https://github.com/stepfunc/oo_bindgen/) model for (extremely) limited FFI bindings to [tokio](https://tokio.rs/).

#![deny(
    dead_code,
    arithmetic_overflow,
    invalid_type_param_default,
    missing_fragment_specifier,
    mutable_transmutes,
    no_mangle_const_items,
    overflowing_literals,
    patterns_in_fns_without_body,
    pub_use_of_private_extern_crate,
    unknown_crate_types,
    order_dependent_trait_objects,
    illegal_floating_point_literal_pattern,
    improper_ctypes,
    late_bound_lifetime_arguments,
    non_camel_case_types,
    non_shorthand_field_patterns,
    non_snake_case,
    non_upper_case_globals,
    no_mangle_generic_items,
    private_in_public,
    stable_features,
    type_alias_bounds,
    tyvar_behind_raw_pointer,
    unconditional_recursion,
    unused_comparisons,
    unreachable_pub,
    anonymous_parameters,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_qualifications,
    clippy::all
)]
#![forbid(
    unsafe_code,
    rustdoc::broken_intra_doc_links,
    unaligned_references,
    while_true,
    bare_trait_objects
)]

use oo_bindgen::model::*;

/// Return the contents of the implementation file for the schema
pub const fn get_impl_file() -> &'static str {
    include_str!("../runtime.rs")
}

/// Define all runtime related API artifacts and return a handle to the runtime class
pub fn define(
    lib: &mut LibraryBuilder,
    error_type: ErrorType<Unvalidated>,
) -> BackTraced<ClassDeclarationHandle> {
    // Forward declare the class
    let runtime = lib.declare_class("runtime")?;

    let config_struct = define_runtime_config(lib)?;

    let constructor = lib
        .define_constructor(runtime.clone())?
        .param(
            "config",
            config_struct,
            "Runtime configuration",
        )?
        .fails_with(error_type)?
        .doc(
            doc("Creates a new runtime for running the protocol stack.")
                .warning("The runtime should be kept alive for as long as it's needed and it should be released with {class:runtime.[destructor]}")
        )?
        .build()?;

    let destructor = lib
        .define_destructor(
            runtime.clone(),
            doc("Destroy a runtime.")
                .details("This method will gracefully wait for all asynchronous operation to end before returning")
        )?;

    let set_shutdown_timeout =
        lib.define_method("set_shutdown_timeout", runtime.clone())?
            .doc(
                    doc("By default, when the runtime shuts down, it does so without a timeout and waits indefinitely for all spawned tasks to yield.")
                    .details("Setting this value will put a maximum time bound on the eventual shutdown. Threads that have not exited within this timeout will be terminated.")
                        .warning("This can leak memory. This method should only be used if the the entire application is being shut down so that memory can be cleaned up by the OS.")
            )?
            .param("timeout", BasicType::Duration(DurationType::Seconds), "Maximum number of seconds to wait for the runtime to shut down")?
            .build()?;

    let runtime = lib
        .define_class(&runtime)?
        .constructor(constructor)?
        .destructor(destructor)?
        .method(set_shutdown_timeout)?
        .custom_destroy("shutdown")?
        .doc("Handle to the underlying runtime")?
        .build()?;

    Ok(runtime.declaration())
}

fn define_runtime_config(lib: &mut LibraryBuilder) -> BackTraced<FunctionArgStructHandle> {
    let num_core_threads = Name::create("num_core_threads")?;

    let config_struct = lib.declare_function_argument_struct("runtime_config")?;
    let config_struct= lib
        .define_function_argument_struct(config_struct)?
        .add(
            &num_core_threads,
            Primitive::U16,
            doc("Number of runtime threads to spawn. For a guess of the number of CPU cores, use 0.")
                .details("Even if tons of connections are expected, it is preferred to use a value around the number of CPU cores for better performances. The library uses an efficient thread pool polling mechanism."),
        )?
        .doc("Runtime configuration")?
        .end_fields()?
        .begin_initializer("init", InitializerType::Normal, "Initialize the configuration to default values")?
        .default(&num_core_threads, NumberValue::U16(0))?
        .end_initializer()?
        .build()?;

    Ok(config_struct)
}
