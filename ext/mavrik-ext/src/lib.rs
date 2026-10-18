use mavrik::ruby::define::define_rb;

/// Entrypoint for the Ruby extension. Initializes the logger for client code and defines the Ruby bindings.
#[magnus::init]
fn init(ruby: &magnus::Ruby) -> Result<(), magnus::Error> {
    env_logger::init();
    define_rb(ruby)?;
    Ok(())
}
