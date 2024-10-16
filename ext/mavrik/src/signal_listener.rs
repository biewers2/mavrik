use std::sync::mpsc::Sender;
use anyhow::Context;
use log::info;
use signal_hook::consts::SIGINT;
use signal_hook::iterator::Signals;
use crate::events::MavrikEvent;

pub fn listen_for_signals(event_tx: Sender<MavrikEvent>) -> Result<(), anyhow::Error> {
    info!("Starting signal listener");
    
    let mut signals = Signals::new(&[SIGINT]).context("failed to add signal listener")?;
    let handle = signals.handle();

    for signal in signals.forever() {
        match signal {
            SIGINT => event_tx.send(MavrikEvent::Signal(SIGINT))?,
            _ => ()
        }
    }

    handle.close();
    Ok(())
}
