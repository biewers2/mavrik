use log::info;
use magnus::RHash;
use mavrik::mavrik::{Mavrik, MavrikOptions};
use mavrik::rb::util::mavrik_error;
use mavrik::runtime::async_runtime;
use mavrik::without_gvl;

fn main() {
    env_logger::init();
    
    magnus::Ruby::init(|r| {
        r.require("./lib/mavrik")?;

        let options = RHash::new();
        info!(options:?; "Starting Mavrik server");

        let result = without_gvl!({ 
            async_runtime().block_on(async move {
                let options = MavrikOptions::from(options);
                Mavrik::new(options).run().await
            })
        });

        result.map_err(mavrik_error)?;
        info!("Mavrik server stopped");
        Ok(()) 
    }).unwrap();
}