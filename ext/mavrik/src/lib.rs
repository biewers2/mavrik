#![allow(async_fn_in_trait)]

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
    rb::define_rb(ruby)?;
    Ok(())
}
