pub mod signal_listener;
pub mod task_executor;
pub mod events;
pub mod rb;
pub mod io;
pub mod runtime;
pub mod event_loop;
pub mod service;

#[magnus::init]
fn init(ruby: &magnus::Ruby) -> Result<(), magnus::Error> {
    env_logger::init();
    rb::define_rb(ruby)?;
    Ok(())
}
