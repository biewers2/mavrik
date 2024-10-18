pub mod signal_listener;
pub mod task_executor;
pub mod tcp_listener;
pub mod events;
pub mod rb;
pub mod client;
pub mod io;
mod runtime;

#[magnus::init]
fn init(ruby: &magnus::Ruby) -> Result<(), magnus::Error> {
    env_logger::init();
    rb::define_rb(ruby)?;
    Ok(())
}
