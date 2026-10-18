use crate::ruby::connection::define_connection;
use crate::ruby::main::define_main;
use magnus::Ruby;

pub fn define_rb(ruby: &Ruby) -> Result<(), magnus::Error> {
    define_main(ruby)?;
    define_connection(ruby)?;
    Ok(())
}
