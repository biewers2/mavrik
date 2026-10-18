use mavrik::ruby::define::define_rb;

#[magnus::init]
fn init(ruby: &magnus::Ruby) -> Result<(), magnus::Error> {
    env_logger::init();
    define_rb(ruby)?;
    Ok(())
}
