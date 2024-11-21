use magnus::Ruby;
use crate::rb::connection::define_client;
use crate::rb::main::define_main;

pub fn define_rb(ruby: &Ruby) -> Result<(), magnus::Error> {
    define_main(ruby)?;
    define_client(ruby)?;
    Ok(())
}
