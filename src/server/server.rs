use crate::common::quinn_helpers::make_server_endpoint;

// pub fn server_main() {
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

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let server_addr = "127.0.0.1:5000".parse().unwrap();
    let (endpoint, server_cert) = make_server_endpoint(server_addr)?;
    
    // Single connection
    let incoming_conn = endpoint.accept().await.unwrap();
    let conn = incoming_conn.await.unwrap();
    println!(
        "[server] connection accepted: addr={}",
        conn.remote_address()
    );
    
    Ok(())
}