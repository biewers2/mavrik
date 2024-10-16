mod main_rb;
mod signal_listener;
mod task_executor;
mod tcp_listener;
mod events;

use crate::main_rb::define_main;
use magnus::Ruby;

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), magnus::Error> {
    define_main(ruby)?;
    Ok(())
}
