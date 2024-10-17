use crate::client::Client;
use crate::rb::module_mavrik;
use magnus::{Module, Ruby};

#[derive(Debug)]
#[magnus::wrap(class = "Mavrik::Client", free_immediately, size)]
pub struct RbClient(pub Client);

pub fn define_client(ruby: &Ruby) -> Result<(), magnus::Error> {
    module_mavrik(ruby).define_class("Client", ruby.class_object())?;
    Ok(())
}
