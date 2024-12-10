#![allow(async_fn_in_trait)]

pub mod io;
pub mod signal_listener;
pub mod messaging;
pub mod rb;
pub mod tcp;
pub mod runtime;
pub mod mavrik;
pub mod service;
pub mod executor;
pub mod store;

#[magnus::init]
fn init(ruby: &magnus::Ruby) -> Result<(), magnus::Error> {
    env_logger::init();
    rb::define::define_rb(ruby)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use magnus::Ruby;
    use crate::rb::{connection, main, util};

    /// Tests that need to be run in a Ruby context can be run here.
    /// 
    /// Because the Ruby VM can only be initialized once per-process, we have to initialize it in a single test thread
    /// and run all tests that require the VM in that thread.
    /// 
    #[test]
    fn test_in_ruby() -> Result<(), String> {
        Ruby::init(|ruby| {
            configure_mavrik(ruby)?;

            // crate::rb::connection
            connection::tests::define_connection_defines_ruby_class_and_methods(&ruby)?;
            connection::tests::new_connection_connects_to_server(&ruby)?;
            connection::tests::new_connection_fails_to_connect_to_server(&ruby)?;
            connection::tests::new_connection_requests_data_from_server(&ruby)?;

            // crate::rb::main
            main::tests::main_defines_ruby_class_and_methods(&ruby)?;

            // crate::rb::util
            util::tests::mrhash_fetch_sym(&ruby)?;
            util::tests::mrhash_fetch_sym_or(&ruby)?;
            util::tests::mrhash_try_fetch_sym(&ruby)?;
            util::tests::mrhash_fetch_str(&ruby)?;
            util::tests::mrhash_fetch_str_or(&ruby)?;
            util::tests::mrhash_try_fetch_str(&ruby)?;
            util::tests::mrhash_fetch(&ruby)?;
            util::tests::mrhash_fetch_or(&ruby)?;
            util::tests::mrhash_try_fetch(&ruby)?;
            util::tests::mrhash_set_sym(&ruby)?;
            util::tests::mrhash_set_str(&ruby)?;
            util::tests::mrhash_set(&ruby)?;
            util::tests::mavrik_module_is_defined(&ruby)?;
            util::tests::mavrik_error_class_is_defined(&ruby)?;
            util::tests::mavrik_error_uses_custom_message(&ruby)?;
            util::tests::in_ruby_calls_fn_in_gvl(&ruby)?;
            util::tests::in_ruby_locks_gvl_then_calls_fn(&ruby)?;
            
            Ok(())
        })
    }
    
    fn configure_mavrik(r: &Ruby) -> Result<magnus::Value, magnus::Error> {
        r.eval(r#"
          module Mavrik
            Error = Class.new(StandardError)
          end
        "#)
    }
}
