use mavrik::client::{Client, ClientOptions};
use mavrik::rb::SubmittedTask;

#[test]
fn test_client_sends_string_less_than_buffer_over_tcp() -> Result<(), anyhow::Error> {
   env_logger::init();
   // let (event_tx, event_rx) = channel::<MavrikEvent>();
   // let server = thread::spawn(|| listen_for_tcp_connections(event_tx));
   
   let mut client = Client::new(ClientOptions {
      host: "127.0.0.1".to_owned(),
      port: 3009
   });
   
   let task = SubmittedTask {
      queue: "default".to_string(),
      definition: "Test".to_string(),
      input_args: "[1, 2]".to_string(),
      input_kwargs: "{\"c\": 3}".to_string(),
   };
   client.submit(&task)?;
   
   // signal_hook::low_level::raise(SIGTERM)?;
   // server.join().unwrap()?;
   Ok(())
}