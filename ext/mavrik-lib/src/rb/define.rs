use magnus::Ruby;
use crate::rb::connection::define_connection;
use crate::rb::main::define_main;

pub fn define_rb(ruby: &Ruby) -> Result<(), magnus::Error> {
    define_main(ruby)?;
    define_connection(ruby)?;
    Ok(())
}
