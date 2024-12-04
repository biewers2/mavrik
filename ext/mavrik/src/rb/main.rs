use crate::mavrik::{Mavrik, MavrikOptions};
use crate::rb::util::{mavrik_error, module_mavrik};
use crate::runtime::async_runtime;
use crate::without_gvl;
use log::info;
use magnus::{function, Object, RHash, Ruby};

pub(crate) fn define_main(_ruby: &Ruby) -> Result<(), magnus::Error> {
    module_mavrik().define_singleton_method("main", function!(main, 1))?;
    Ok(())
}

fn main(options: RHash) -> Result<(), magnus::Error> {
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
}
