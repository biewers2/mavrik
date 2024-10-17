use magnus::{Module, RClass, RModule, Ruby};

pub fn module_mavrik(ruby: &Ruby) -> RModule {
    ruby
        .class_object()
        .const_get::<_, RModule>("Mavrik")
        .expect("Mavrik module not defined")
}

pub fn class_mavrik_client(ruby: &Ruby) -> Result<RClass, magnus::Error> {
    let mavrik = module_mavrik(ruby);
    let client = mavrik
        .const_get("Client")
        .unwrap_or(mavrik.define_class("Client", ruby.class_object())?);
    Ok(client)
}
