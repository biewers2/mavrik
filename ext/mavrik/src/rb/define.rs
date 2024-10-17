use magnus::Ruby;
use crate::rb::client::define_client;
use crate::rb::client_submit_task::define_client_submit_task;
use crate::rb::init::define_init;
use crate::rb::main::define_main;

pub fn define_rb(ruby: &Ruby) -> Result<(), magnus::Error> {
    define_main(ruby)?;
    define_client(ruby)?;
    define_init(ruby)?;
    define_client_submit_task(ruby)?;
    Ok(())
}
