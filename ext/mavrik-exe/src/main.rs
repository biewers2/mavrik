use log::info;
use serde_magnus::deserialize;
use mavrik::mavrik::{Mavrik, MavrikOptions};
use mavrik::rb::util::mavrik_error;
use mavrik::runtime::async_runtime;
use mavrik::without_gvl;

fn main() {
    env_logger::init();
    
    magnus::Ruby::init(|ruby| {
        ruby.require("./lib/mavrik")?;

        let options: MavrikOptions = deserialize(&ruby, ruby.hash_new())?;
        info!(options:?; "Starting Mavrik server");

        let options_ref = &options;
        let result = without_gvl!({ 
            async_runtime().block_on(async move {
                Mavrik::new(options_ref).run().await
            })
        });

        result.map_err(mavrik_error)?;
        info!("Mavrik server stopped");
        Ok(()) 
    }).unwrap();
}