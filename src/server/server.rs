
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

use crate::networking::quinn_helpers::make_server_endpoint;

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

    let mut send = conn
        .open_uni()
        .await?;

    for _ in 0..100 {
        send.write_all(b"testALSKGJA;LW4JT3WL4YJL;3HJK;LKW5JHPOWHJOPSEIJHPO5IJHPOSEHNOSWN3459PHN3WE5OHUNEOS;HJNPWO35HJUPOESHNOSEHP935HJNPOESHPO3E5NHPOE35UHPOSBHJPO9W385HJIOESBNOSJNB93P5WONHOSEIHY9P3W5ONHO;ESBHNOP35YHJPOSHYPOISHPOH").await?;
        send.write_all(b"AAAAA").await?;
    }
    send.finish().await?;
    
    println!("[server] Closing connection");

    Ok(())
}