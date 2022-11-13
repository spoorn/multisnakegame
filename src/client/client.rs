use quinn::Endpoint;
use crate::common::quinn_helpers::make_client_endpoint;

// pub fn client_main() {
//     let code = {
//         if let Err(e) = run() {
//             eprintln!("ERROR: {}", e);
//             1
//         } else {
//             0
//         }
//     };
//     ::std::process::exit(code);
// }

//#[tokio::main]
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let server_addr = "127.0.0.1:5000".parse().unwrap();
    let client_addr = "127.0.0.1:5001".parse().unwrap();
    // Bind this endpoint to a UDP socket on the given client address.
    let mut endpoint = make_client_endpoint(client_addr, &[])?;

    // Connect to the server passing in the server name which is supposed to be in the server certificate.
    let connection = endpoint.connect(server_addr, "localhost")?.await?;
    println!("[client] connected: addr={}", connection.remote_address());

    // Waiting for a stream will complete with an error when the server closes the connection
    let _ = connection.accept_uni().await;

    // Give the server has a chance to clean up
    endpoint.wait_idle().await;
    
    Ok(())
}