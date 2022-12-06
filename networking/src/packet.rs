use std::any::{Any, type_name, TypeId};
use std::error::Error;
use std::fmt::Debug;
use std::sync::Arc;

use bimap::BiMap;
use bytes::Bytes;
use dashmap::DashMap;
use derive_more::Display;
use hashbrown::HashMap;
use quinn::{Connection, RecvStream, SendStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, RwLock};
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;

use networking_macros::ErrorMessageNew;

use crate::quinn_helpers::{make_client_endpoint, make_server_endpoint};

const FRAME_BOUNDARY: &[u8] = b"AAAAAA031320050421";

pub trait Packet {
    fn to_bytes(self) -> Bytes;
    
    // https://stackoverflow.com/questions/33687447/how-to-get-a-reference-to-a-concrete-type-from-a-trait-object
    // fn as_any(self: Self) -> Box<dyn Any>;
}

pub trait PacketBuilder<T: Packet> {
    
    fn read(&self, bytes: Bytes) -> Result<T, Box<dyn Error>>;
}

#[derive(Debug, Clone, Display, ErrorMessageNew)]
pub struct ConnectionError {
    message: String
}

#[derive(Debug, Clone, Display, ErrorMessageNew)]
pub struct ReceiveError {
    message: String
}

#[derive(Debug, Clone, Display, ErrorMessageNew)]
pub struct SendError {
    message: String
}

// TODO: Document that same runtime must be used for a PacketManager instance due to channels
#[derive(Debug)]
pub struct PacketManager {
    receive_packets: BiMap<u32, TypeId>,
    send_packets: BiMap<u32, TypeId>,
    recv_packet_builders: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    recv_streams: Arc<RwLock<Vec<(String, DashMap<u32, RecvStream>)>>>,
    send_streams: Arc<RwLock<Vec<(String, DashMap<u32, SendStream>)>>>,
    rx: Arc<RwLock<Vec<(String, DashMap<u32, (Receiver<Bytes>, JoinHandle<()>)>)>>>,
    // Endpoint and Connection structs moved to the struct fields to prevent closing connections
    // by dropping.
    client_connections: Arc<DashMap<String, Connection>>,
    server_connection: Option<Connection>,
    next_receive_id: u32,
    next_send_id: u32,
    // We construct a single Tokio Runtime to be used by each PacketManger instance, so that
    // methods can be synchronous.  There is also an async version of each API if the user wants
    // to use their own runtime.
    runtime: Arc<Option<Runtime>>
}

impl PacketManager {
    
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build();
        match runtime {
            Ok(runtime) => {
                PacketManager {
                    receive_packets: BiMap::new(),
                    send_packets: BiMap::new(),
                    recv_packet_builders: HashMap::new(),
                    recv_streams: Arc::new(RwLock::new(vec![])),
                    send_streams: Arc::new(RwLock::new(vec![])),
                    rx: Arc::new(RwLock::new(vec![])),
                    client_connections: Arc::new(DashMap::new()),
                    server_connection: None,
                    next_receive_id: 0,
                    next_send_id: 0,
                    runtime: Arc::new(Some(runtime))
                }
            }
            Err(e) => {
                panic!("Could not create a Tokio runtime for PacketManager.  If you are calling new() from code that already has an async runtime available, use PacketManager.new_async(), and respective async_*() versions of APIs.  --  {}", e);
            }
        }
    }
    
    pub fn new_for_async() -> Self {
        PacketManager {
            receive_packets: BiMap::new(),
            send_packets: BiMap::new(),
            recv_packet_builders: HashMap::new(),
            recv_streams: Arc::new(RwLock::new(vec![])),
            send_streams: Arc::new(RwLock::new(vec![])),
            rx: Arc::new(RwLock::new(vec![])),
            client_connections: Arc::new(DashMap::new()),
            server_connection: None,
            next_receive_id: 0,
            next_send_id: 0,
            runtime: Arc::new(None)
        }
    }

    pub fn init_connections<S: Into<String>>(&mut self, is_server: bool, num_incoming_streams: u32, num_outgoing_streams: u32, server_addr: S, client_addr: Option<S>, wait_for_clients: u32, expected_num_clients: Option<u32>) -> Result<(), Box<dyn Error>> {
        match self.runtime.as_ref() {
            None => {
                panic!("PacketManager does not have a runtime instance associated with it.  Did you mean to call async_init_connection()?");
            }
            Some(runtime) => {
                // TODO: this isn't so great, we can refactor to create static methods that don't require mutable ref to self, and use those instead later on
                runtime.block_on(PacketManager::async_init_connections_helper(is_server, num_incoming_streams, num_outgoing_streams, server_addr, client_addr,
                                                                                        wait_for_clients, expected_num_clients, &self.runtime, &self.send_streams, &self.rx,
                                                                                        &self.client_connections, &mut self.server_connection))
            }
        }
    }

    pub async fn async_init_connections<S: Into<String>>(&mut self, is_server: bool, num_incoming_streams: u32, num_outgoing_streams: u32, server_addr: S, mut client_addr: Option<S>, wait_for_clients: u32, expected_num_clients: Option<u32>) -> Result<(), Box<dyn Error>> {
        if self.runtime.is_some() {
            panic!("PacketManager has a runtime instance associated with it.  If you are using async_init_connections(), make sure you create the PacketManager using new_async(), not new()");
        }
        PacketManager::async_init_connections_helper(is_server, num_incoming_streams, num_outgoing_streams, server_addr, client_addr, 
                                                     wait_for_clients, expected_num_clients, &self.runtime, &self.send_streams, &self.rx, 
                                                     &self.client_connections, &mut self.server_connection).await
    }

        // TODO: validate number of streams against registered packets 
    async fn async_init_connections_helper<S: Into<String>>(is_server: bool, num_incoming_streams: u32, num_outgoing_streams: u32, 
                                                            server_addr: S, mut client_addr: Option<S>, wait_for_clients: u32, 
                                                            expected_num_clients: Option<u32>, runtime: &Arc<Option<Runtime>>,
                                                            send_streams: &Arc<RwLock<Vec<(String, DashMap<u32, SendStream>)>>>,
                                                            rx: &Arc<RwLock<Vec<(String, DashMap<u32, (Receiver<Bytes>, JoinHandle<()>)>)>>>,
                                                            client_connections: &Arc<DashMap<String, Connection>>,
                                                            server_connection: &mut Option<Connection>) -> Result<(), Box<dyn Error>> {
        // if expected_num_accepts_uni != self.next_receive_id {
        //     return Err(Box::new(ConnectionError::new("expected_num_accepts_uni does not match number of registered receive packets")));
        // }
        
        let client_addr = match client_addr.take() {
            None => { "None".to_string() }
            Some(s) => { s.into() }
        };
        let server_addr = server_addr.into();
        println!("Initiating connection with is_server={}, num_incoming_streams={}, num_outgoing_streams={}, server_addr={}, client_addr={}", is_server, num_incoming_streams, num_outgoing_streams, server_addr, client_addr);
        // TODO: assert num streams equals registered
        let server_addr = server_addr.parse().unwrap();
        
        if is_server {
            let (endpoint, server_cert) = make_server_endpoint(server_addr)?;

            // TODO: use synchronous blocks during read and write of client_connections
            // Single connection
            for _ in 0..wait_for_clients {
                let incoming_conn = endpoint.accept().await.unwrap();
                let conn = incoming_conn.await.unwrap();
                let addr = conn.remote_address();
                if client_connections.contains_key(&addr.to_string()) {
                    panic!("Client with addr={} was already connected", addr);
                }
                println!("[server] connection accepted: addr={}", conn.remote_address());
                let (server_send_streams, recv_streams) = PacketManager::open_streams_for_connection(addr.to_string(), &conn, num_incoming_streams, num_outgoing_streams).await?;
                let res = PacketManager::spawn_receive_thread(&addr.to_string(), recv_streams, runtime.as_ref()).unwrap();
                rx.write().await.push((addr.to_string(), res));
                send_streams.write().await.push((addr.to_string(), server_send_streams));
                //self.recv_streams.write().await.push((addr.to_string(), recv_streams));
                client_connections.insert(client_addr.clone(), conn);
            }
            
            if expected_num_clients.is_none() || expected_num_clients.unwrap() > wait_for_clients {
                let client_connections = client_connections.clone();
                let mut arc_send_streams = send_streams.clone();
                let mut arc_rx = rx.clone();
                //let mut arc_recv_streams = self.recv_streams.clone();
                let arc_runtime = Arc::clone(runtime);
                let accept_client_task = async move {
                    match expected_num_clients {
                        None => {
                            loop {
                                println!("[server] Waiting for more clients...");
                                let incoming_conn = endpoint.accept().await.unwrap();
                                let conn = incoming_conn.await.unwrap();
                                let addr = conn.remote_address();
                                if client_connections.contains_key(&addr.to_string()) {
                                    panic!("Client with addr={} was already connected", addr);
                                }
                                println!("[server] connection accepted: addr={}", conn.remote_address());
                                let (send_streams, recv_streams) = PacketManager::open_streams_for_connection(addr.to_string(), &conn, num_incoming_streams, num_outgoing_streams).await.unwrap();
                                let res = PacketManager::spawn_receive_thread(&addr.to_string(), recv_streams, arc_runtime.as_ref()).unwrap();
                                arc_rx.write().await.push((addr.to_string(), res));
                                arc_send_streams.write().await.push((addr.to_string(), send_streams));
                                client_connections.insert(addr.to_string(), conn);
                            }
                        }
                        Some(expected_num_clients) => {
                            for i in 0..(expected_num_clients - wait_for_clients) {
                                println!("[server] Waiting for client #{}", i + wait_for_clients);
                                let incoming_conn = endpoint.accept().await.unwrap();
                                let conn = incoming_conn.await.unwrap();
                                let addr = conn.remote_address();
                                if client_connections.contains_key(&addr.to_string()) {
                                    panic!("Client with addr={} was already connected", addr);
                                }
                                println!("[server] connection accepted: addr={}", conn.remote_address());
                                let (send_streams, recv_streams) = PacketManager::open_streams_for_connection(addr.to_string(), &conn, num_incoming_streams, num_outgoing_streams).await.unwrap();
                                let res = PacketManager::spawn_receive_thread(&addr.to_string(), recv_streams, arc_runtime.as_ref()).unwrap();
                                arc_rx.write().await.push((addr.to_string(), res));
                                arc_send_streams.write().await.push((addr.to_string(), send_streams));
                                client_connections.insert(addr.to_string(), conn);
                            }
                        }
                    }
                    Ok::<(), Box<ConnectionError>>(())
                };

                // TODO: save this value
                let accept_client_thread = match runtime.as_ref() {
                    None => {
                        tokio::spawn(accept_client_task)
                    }
                    Some(runtime) => {
                        runtime.spawn(accept_client_task)
                    }
                };
            }
        } else {
            // Bind this endpoint to a UDP socket on the given client address.
            let endpoint = make_client_endpoint(client_addr.parse().unwrap(), &[])?;

            // Connect to the server passing in the server name which is supposed to be in the server certificate.
            let conn = endpoint.connect(server_addr, "hostname")?.await?;
            let addr = conn.remote_address();
            println!("[client] connected: addr={}", addr);
            let (client_send_streams, recv_streams) = PacketManager::open_streams_for_connection(addr.to_string(), &conn, num_incoming_streams, num_outgoing_streams).await?;
            let res = PacketManager::spawn_receive_thread(&addr.to_string(), recv_streams, runtime.as_ref()).unwrap();
            rx.write().await.push((addr.to_string(), res));
            send_streams.write().await.push((addr.to_string(), client_send_streams));
            //self.recv_streams.write().await.push((addr.to_string(), recv_streams));
            let _ = server_connection.insert(conn);
        }
        
        Ok(())
    }
    
    async fn open_streams_for_connection(addr: String, conn: &Connection, num_incoming_streams: u32, num_outgoing_streams: u32) -> Result<(DashMap<u32, SendStream>, DashMap<u32, RecvStream>), Box<dyn Error>> {
        let mut send_streams = DashMap::new();
        let mut recv_streams = DashMap::new();
        // Note: Packets are not sent immediately upon the write.  The thread needs to be kept
        // open so that the packets can actually be sent over the wire to the client.
        for i in 0..num_outgoing_streams {
            println!("Opening outgoing stream for addr={} packet id={}", addr, i);
            let mut send_stream = conn
                .open_uni()
                .await?;
            println!("Writing packet to {} for packet id {}", addr, i);
            send_stream.write_u32(i).await?;
            send_streams.insert(i, send_stream);
        }

        for i in 0..num_incoming_streams {
            println!("Accepting incoming stream from {} for packet id {}", addr, i);
            let mut recv = conn.accept_uni().await?;
            println!("Validating incoming packet from {} id {}", addr, i);
            let id = recv.read_u32().await?;
            println!("Received incoming packet from {} with packet id {}", addr, id);
            // if id >= self.next_receive_id {
            //     return Err(Box::new(ConnectionError::new(format!("Received unexpected packet ID {} from server", id))));
            // }

            recv_streams.insert(i, recv);
        }

        Ok((send_streams, recv_streams))
    }
    
    fn spawn_receive_thread<S: Into<String>>(addr: S, recv_streams: DashMap<u32, RecvStream>, runtime: &Option<Runtime>) -> Result<DashMap<u32, (Receiver<Bytes>, JoinHandle<()>)>, Box<dyn Error>> {
        let mut rxs = DashMap::new();
        println!("Spawning receive thread for addr={} for {} ids", addr.into(), recv_streams.len());
        for (id, mut recv_stream) in recv_streams.into_iter() {
            let (tx, rx) = mpsc::channel(100);

            let task = async move {
                let mut partial_chunk: Option<Bytes> = None;
                loop {
                    // TODO: relay error message
                    // TODO: configurable size limit
                    let chunk = recv_stream.read_chunk(usize::MAX, true).await.unwrap();
                    match chunk {
                        None => {
                            // TODO: Error
                            println!("Receive stream closed, got None when reading chunks");
                            break;
                        }
                        Some(chunk) => {
                            println!("Received chunked packets for id={}, length={}", id, chunk.bytes.len());
                            let bytes;
                            match partial_chunk.take() {
                                None => {
                                    bytes = chunk.bytes;
                                }
                                Some(part) => {
                                    bytes = Bytes::from([part, chunk.bytes].concat());
                                }
                            }

                            // TODO: Make trace log
                            println!("Received bytes: {:?}", bytes);
                            let boundaries: Vec<usize> = bytes.windows(FRAME_BOUNDARY.len()).enumerate().filter(|(_, w)| matches!(*w, FRAME_BOUNDARY)).map(|(i, _)| i).collect();
                            let mut offset = 0;
                            for i in boundaries.iter() {
                                // Reached end of bytes
                                if offset >= bytes.len() {
                                    break;
                                }
                                let frame = bytes.slice(offset..*i);
                                match partial_chunk.take() {
                                    None => {
                                        if matches!(frame.as_ref(), FRAME_BOUNDARY) {
                                            println!("Found a dangling FRAME_BOUNDARY in packet frame.  This most likely is a bug in the networking library!")
                                        } else {
                                            println!("Sending length {}", frame.len());
                                            tx.send(frame).await.unwrap();
                                        }
                                    },
                                    Some(part) => {
                                        let reconstructed_frame = Bytes::from([part, frame].concat());
                                        if matches!(reconstructed_frame.as_ref(), FRAME_BOUNDARY) {
                                            println!("Found a dangling FRAME_BOUNDARY in packet frame.  This most likely is a bug in the networking library!")
                                        } else {
                                            println!("Sending reconstructed length {}", reconstructed_frame.len());
                                            tx.send(reconstructed_frame).await.unwrap();
                                        }
                                    }
                                }
                                offset = i + FRAME_BOUNDARY.len();
                            }

                            if boundaries.is_empty() || (offset + FRAME_BOUNDARY.len() != bytes.len() - 1) {
                                let prefix_part = bytes.slice(offset..bytes.len());
                                match partial_chunk.take() {
                                    None => {
                                        partial_chunk = Some(prefix_part);
                                    }
                                    Some(part) => {
                                        partial_chunk = Some(Bytes::from([part, prefix_part].concat()))
                                    }
                                }
                            }
                        }
                    }
                }
            };

            let receive_thread: JoinHandle<()> = match runtime.as_ref() {
                None => {
                    tokio::spawn(task)
                }
                Some(runtime) => {
                    runtime.spawn(task)
                }
            };

            rxs.insert(id, (rx, receive_thread));
        }
        
        Ok(rxs)
    }
    
    pub fn register_receive_packet<T: Packet + 'static>(&mut self, packet_builder: impl PacketBuilder<T> + 'static + Sync + Send + Copy) -> Result<(), ReceiveError> {
        self.validate_packet_is_new::<T>(false)?;
        let packet_type_id = TypeId::of::<T>();
        self.receive_packets.insert(self.next_receive_id, packet_type_id);
        self.recv_packet_builders.insert(packet_type_id, Box::new(packet_builder));
        println!("Registered Receive packet with id={}, type={}", self.next_receive_id, type_name::<T>());
        self.next_receive_id += 1;
        Ok(())
    }
    
    pub fn register_send_packet<T: Packet + 'static>(&mut self) -> Result<(), ReceiveError> {
        self.validate_packet_is_new::<T>(true)?;
        self.send_packets.insert(self.next_send_id, TypeId::of::<T>());
        println!("Registered Send packet with id={}, type={}", self.next_send_id, type_name::<T>());
        self.next_send_id += 1;
        Ok(())
    }

    pub fn received<T: Packet + 'static, U: PacketBuilder<T> + 'static>(&mut self, blocking: bool) -> Result<Option<Vec<T>>, ReceiveError> {
        self.validate_for_received::<T>()?;
        match self.runtime.as_ref() {
            None => {
                panic!("PacketManager does not have a runtime instance associated with it.  Did you mean to call async_received()?");
            }
            Some(runtime) => {
                runtime.block_on(PacketManager::async_received_helper::<T, U>(blocking, &self.receive_packets, &self.recv_packet_builders, &self.rx))
            }
        }
    }

    pub async fn async_received<T: Packet + 'static, U: PacketBuilder<T> + 'static>(&mut self, blocking: bool) -> Result<Option<Vec<T>>, ReceiveError> {
        if self.runtime.is_some() {
            panic!("PacketManager has a runtime instance associated with it.  If you are using async_received(), make sure you create the PacketManager using new_async(), not new()");
        }
        self.validate_for_received::<T>()?;
        PacketManager::async_received_helper::<T, U>(blocking, &self.receive_packets, &self.recv_packet_builders, &self.rx).await
    }
    
    // Assumes does not have more than one client to send to, should be checked by callers
    async fn async_received_helper<T: Packet + 'static, U: PacketBuilder<T> + 'static>(blocking: bool,
                                                                                       receive_packets: &BiMap<u32, TypeId>,
                                                                                       recv_packet_builders: &HashMap<TypeId, Box<dyn Any + Send + Sync>>,
                                                                                       rx: &Arc<RwLock<Vec<(String, DashMap<u32, (Receiver<Bytes>, JoinHandle<()>)>)>>>) -> Result<Option<Vec<T>>, ReceiveError> {
        let packet_type_id = TypeId::of::<T>();
        let id = receive_packets.get_by_right(&packet_type_id).unwrap();
        let (addr, rxs) = &rx.read().await[0];
        let rx = &mut rxs.get_mut(id).unwrap().0;
        let mut res: Vec<T> = Vec::new();
        let packet_builder: &U = recv_packet_builders.get(&TypeId::of::<T>()).unwrap().downcast_ref::<U>().unwrap();
        
        // If blocking, wait for the first packet
        if blocking {
            match rx.recv().await {
                None => { return Err(ReceiveError::new(format!("Channel for packet type {} closed unexpectedly!", type_name::<T>()))); }
                Some(bytes) => {
                    PacketManager::receive_bytes::<T, U>(bytes, packet_builder, &mut res)?;
                }
            }
        }
        
        // Loop for any subsequent packets
        loop {
            match rx.try_recv() {
                Ok(bytes) => {
                    PacketManager::receive_bytes::<T, U>(bytes, packet_builder, &mut res)?;
                }
                Err(e) => {
                    match e {
                        TryRecvError::Empty => { break; }
                        TryRecvError::Disconnected => {
                            return Err(ReceiveError::new(format!("Receiver channel for type {} was disconnected", type_name::<T>())));
                        }
                    }
                }
            }
        }

        if res.is_empty() {
            return Ok(None);
        }
        println!("Fetched {} received packets of type={}, id={}, for addr={}", res.len(), type_name::<T>(), id, addr);
        Ok(Some(res))
    }
    
    fn validate_for_received<T: Packet + 'static>(&self) -> Result<Option<Vec<T>>, ReceiveError> {
        if self.has_more_than_one_remote() {
            return Err(ReceiveError::new(format!("async_received()/received() was called for packet {}, but there is more than one client.  Did you mean to call async_received_all()/received_all()?", type_name::<T>())))
        }

        self.validate_packet_was_registered::<T>(false)?;
        Ok(None)
    }

    pub fn send<T: Packet + 'static>(&mut self, packet: T) -> Result<(), SendError> {
        self.validate_for_send::<T>()?;
        match self.runtime.as_ref() {
            None => {
                panic!("PacketManager does not have a runtime instance associated with it.  Did you mean to call async_received()?");
            }
            Some(runtime) => {
                runtime.block_on(PacketManager::async_send_helper::<T>(packet, &self.send_packets, &self.send_streams))
            }
        }
    }

    pub async fn async_send<T: Packet + 'static>(&mut self, packet: T) -> Result<(), SendError> {
        if self.runtime.is_some() {
            panic!("PacketManager has a runtime instance associated with it.  If you are using async_send(), make sure you create the PacketManager using new_async(), not new()");
        }
        self.validate_for_send::<T>()?;
        PacketManager::async_send_helper::<T>(packet, &self.send_packets, &self.send_streams).await
    }
    
    async fn async_send_helper<T: Packet + 'static>(packet: T, send_packets: &BiMap<u32, TypeId>,
                                                    send_streams: &Arc<RwLock<Vec<(String, DashMap<u32, SendStream>)>>>,) -> Result<(), SendError> {
        let bytes = packet.to_bytes();
        let packet_type_id = TypeId::of::<T>();
        let id = send_packets.get_by_right(&packet_type_id).unwrap();
        let (addr, send_streams) = &send_streams.read().await[0];
        let mut send_stream = send_streams.get_mut(id).unwrap();
        // TODO: Make trace log
        println!("Sending bytes to {}: {:?}", addr, bytes);
        send_stream.write_chunk(bytes).await.unwrap();
        send_stream.write_all(FRAME_BOUNDARY).await.unwrap();
        println!("Sent packet to {} with id={}, type={}", addr, id, type_name::<T>());
        Ok(())
    }
    
    fn validate_for_send<T: Packet + 'static>(&self) -> Result<(), SendError> {
        if self.has_more_than_one_remote() {
            return Err(SendError::new(format!("async_send()/send() was called for packet {}, but there is more than one client.  Did you mean to call async_broadcast()/broadcast()?", type_name::<T>())))
        }
        
        Ok(())
    }
    
    fn validate_packet_is_new<T: Packet + 'static>(&self, is_send: bool) -> Result<(), ReceiveError> {
        if (is_send && self.send_packets.contains_right(&TypeId::of::<T>())) || !is_send && self.receive_packets.contains_right(&TypeId::of::<T>()) {
            return Err(ReceiveError::new(format!("Type '{}' was already registered!", type_name::<T>())))
        } 
        Ok(())
    }
    
    fn validate_packet_was_registered<T: Packet + 'static>(&self, is_send: bool) -> Result<(), ReceiveError> {
        if is_send {
            if !self.send_packets.contains_right(&TypeId::of::<T>()) {
                return Err(ReceiveError::new(format!("Type '{}' was never registered!  Did you forget to call register_send_packet()?", type_name::<T>())))
            }
        } else {
            if !self.receive_packets.contains_right(&TypeId::of::<T>()) {
                return Err(ReceiveError::new(format!("Type '{}' was never registered!  Did you forget to call register_receive_packet()?", type_name::<T>())))
            }
        }
        Ok(())
    }
    
    #[inline]
    fn receive_bytes<T: Packet + 'static, U: PacketBuilder<T> + 'static>(bytes: Bytes, packet_builder: &U, res: &mut Vec<T>) -> Result<(), ReceiveError> {
        if bytes.is_empty() {
            return Err(ReceiveError::new(format!("Received empty bytes for packet type={}!", type_name::<T>())));
        }
        println!("Received packet #{} for type {}", res.len(), type_name::<T>());
        let packet = packet_builder.read(bytes).unwrap();
        res.push(packet);
        Ok(())
    }

    // Either we are a single client talking to a single server, or a server talking to potentially multiple clients
    #[inline]
    fn has_more_than_one_remote(&self) -> bool {
        self.server_connection.is_none() && self.client_connections.len() > 1
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use networking::packet::PacketManager;
    use networking_macros::bincode_packet;

    use crate as networking;

    #[bincode_packet]
    #[derive(Debug, PartialEq, Eq)]
    struct Test {
        id: i32
    }

    #[bincode_packet]
    #[derive(Debug, PartialEq, Eq)]
    struct Other {
        name: String,
        id: i32
    }

    // TODO: Test sync versions
    #[tokio::test]
    async fn receive_packet_e2e_async() {
        let mut manager = PacketManager::new_for_async();
        
        let (tx, mut rx) = mpsc::channel(100);
        let server_addr = "127.0.0.1:5000";
        let client_addr = "127.0.0.1:5001";
        
        // Server
        let server = tokio::spawn(async move {
            let mut m = PacketManager::new_for_async();
            assert!(m.register_send_packet::<Test>().is_ok());
            assert!(m.register_send_packet::<Other>().is_ok());
            assert!(m.register_receive_packet::<Test>(TestPacketBuilder).is_ok());
            assert!(m.register_receive_packet::<Other>(OtherPacketBuilder).is_ok());
            assert!(m.async_init_connections(true, 2, 2, server_addr, None, 1, None).await.is_ok());
            
            for _ in 0..100 {
                assert!(m.async_send::<Test>(Test { id: 5 }).await.is_ok());
                assert!(m.async_send::<Test>(Test { id: 8 }).await.is_ok());
                assert!(m.async_send::<Other>(Other { name: "spoorn".to_string(), id: 4 }).await.is_ok());
                assert!(m.async_send::<Other>(Other { name: "kiko".to_string(), id: 6 }).await.is_ok());
                
                let test_res = m.async_received::<Test, TestPacketBuilder>(true).await;
                assert!(test_res.is_ok());
                let unwrapped = test_res.unwrap();
                assert!(unwrapped.is_some());
                assert_eq!(unwrapped.unwrap(), vec![Test { id: 6 }, Test { id: 9 }]);
                let other_res = m.async_received::<Other, OtherPacketBuilder>(true).await;
                assert!(other_res.is_ok());
                let unwrapped = other_res.unwrap();
                assert!(unwrapped.is_some());
                assert_eq!(unwrapped.unwrap(), vec![Other { name: "mango".to_string(), id: 1 }, Other { name: "luna".to_string(), id: 3 }]);
            }
            
            rx.recv().await;
            // loop {
            //     // Have to use tokio's sleep so it can yield to the tokio executor
            //     // https://stackoverflow.com/questions/70798841/why-does-a-tokio-thread-wait-for-a-blocking-thread-before-continuing?rq=1
            //     //sleep(Duration::from_millis(100)).await;
            // }
        });
        
        // Client
        assert!(manager.register_receive_packet::<Test>(TestPacketBuilder).is_ok());
        assert!(manager.register_receive_packet::<Other>(OtherPacketBuilder).is_ok());
        assert!(manager.register_send_packet::<Test>().is_ok());
        assert!(manager.register_send_packet::<Other>().is_ok());
        let client = manager.async_init_connections(false, 2, 2, server_addr, Some(client_addr), 0, None).await;
        println!("{:#?}", client);
        
        assert!(client.is_ok());
        
        for _ in 0..100 {
            // Send packets
            assert!(manager.async_send::<Test>(Test { id: 6 }).await.is_ok());
            assert!(manager.async_send::<Test>(Test { id: 9 }).await.is_ok());
            assert!(manager.async_send::<Other>(Other { name: "mango".to_string(), id: 1 }).await.is_ok());
            assert!(manager.async_send::<Other>(Other { name: "luna".to_string(), id: 3 }).await.is_ok());
            
            let test_res = manager.async_received::<Test, TestPacketBuilder>(true).await;
            assert!(test_res.is_ok());
            let unwrapped = test_res.unwrap();
            assert!(unwrapped.is_some());
            assert_eq!(unwrapped.unwrap(), vec![Test { id: 5 }, Test { id: 8 }]);
            let other_res = manager.async_received::<Other, OtherPacketBuilder>(true).await;
            assert!(other_res.is_ok());
            let unwrapped = other_res.unwrap();
            assert!(unwrapped.is_some());
            assert_eq!(unwrapped.unwrap(), vec![Other { name: "spoorn".to_string(), id: 4 }, Other { name: "kiko".to_string(), id: 6 }]);
        }
        
        tx.send(0).await.unwrap();
        assert!(server.await.is_ok());
    }

    #[test]
    fn test_register_send_packet() {
        let mut manager = PacketManager::new();
        assert!(manager.validate_packet_is_new::<Test>(true).is_ok());
        assert!(manager.register_send_packet::<Test>().is_ok());
        assert!(manager.validate_packet_is_new::<Test>(true).is_err());
        assert!(manager.register_send_packet::<Test>().is_err());
    }
}