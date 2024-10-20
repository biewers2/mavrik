mod signal_listener;
mod task_executor;
mod events;
mod rb;
mod io;
mod runtime;
mod event_loop;

#[magnus::init]
fn init(ruby: &magnus::Ruby) -> Result<(), magnus::Error> {
    env_logger::init();
    rb::define_rb(ruby)?;
    Ok(())
}
