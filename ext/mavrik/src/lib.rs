#![allow(async_fn_in_trait)]

pub mod sig;
pub mod events;
pub mod rb;
pub mod tcp;
pub mod runtime;
pub mod event_loop;
pub mod service;
pub mod exe;

#[magnus::init]
fn init(ruby: &magnus::Ruby) -> Result<(), magnus::Error> {
    env_logger::init();
    rb::define_rb(ruby)?;
    Ok(())
}
