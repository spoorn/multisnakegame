use std::any::{Any, type_name, TypeId};
use std::error::Error;
use std::fmt::{Debug, Display};
use std::time::Duration;

use bimap::BiMap;
use bytes::Bytes;
use crossbeam_queue::SegQueue;
use derive_more::Display;
use hashbrown::HashMap;
use quinn::{Connection, RecvStream, SendStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::Receiver;
use tokio::time::sleep;

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

#[derive(Debug, Clone, thiserror::Error, Display, ErrorMessageNew)]
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

pub struct PacketManager {
    receive_packets: BiMap<u32, TypeId>,
    send_packets: BiMap<u32, TypeId>,
    receive: HashMap<TypeId, SegQueue<Bytes>>,
    recv_packet_builders: HashMap<TypeId, Box<dyn Any + Send>>,
    recv_streams: HashMap<u32, RecvStream>,
    send_streams: HashMap<u32, SendStream>,
    rx: HashMap<TypeId, Receiver<Bytes>>,
    next_receive_id: u32,
    next_send_id: u32
}

impl PacketManager {
    
    pub fn new() -> Self {
        PacketManager {
            receive_packets: BiMap::new(),
            send_packets: BiMap::new(),
            receive: HashMap::new(),
            recv_packet_builders: HashMap::new(),
            recv_streams: HashMap::new(),
            send_streams: HashMap::new(),
            rx: HashMap::new(),
            next_receive_id: 0,
            next_send_id: 0
        }
    }

    pub async fn init_connection(&mut self, is_server: bool, num_incoming_streams: u32, num_outgoing_streams: u32) -> Result<(), Box<dyn Error>> {
        // if expected_num_accepts_uni != self.next_receive_id {
        //     return Err(Box::new(ConnectionError::new("expected_num_accepts_uni does not match number of registered receive packets")));
        // }
        
        // TODO: assert num streams equals registered
        let server_addr = "127.0.0.1:5000".parse().unwrap();
        let client_addr = "127.0.0.1:5001".parse().unwrap();
        
        let conn: Connection;;
        
        if is_server {
            let (endpoint, server_cert) = make_server_endpoint(server_addr)?;

            // Single connection
            let incoming_conn = endpoint.accept().await.unwrap();
            conn = incoming_conn.await.unwrap();
            println!("[server] connection accepted: addr={}", conn.remote_address());
        } else {
            // Bind this endpoint to a UDP socket on the given client address.
            let mut endpoint = make_client_endpoint(client_addr, &[])?;

            // Connect to the server passing in the server name which is supposed to be in the server certificate.
            conn = endpoint.connect(server_addr, "localhost")?.await?;
            println!("[client] connected: addr={}", conn.remote_address());
        }

        // Note: Packets are not sent immediately upon the write.  The thread needs to be kept
        // open so that the packets can actually be sent over the wire to the client.
        for i in 0..num_outgoing_streams {
            println!("Opening outgoing stream for packet id {}", i);
            let mut send_stream = conn
                .open_uni()
                .await?;
            println!("Writing packet id {}", i);
            send_stream.write_u32(i).await?;
            self.send_streams.insert(i, send_stream);
        }

        for i in 0..num_incoming_streams {
            println!("Accepting incoming stream for packet id {}", i);
            let mut recv = conn.accept_uni().await?;
            println!("Validating incoming packet id {}", i);
            let id = recv.read_u32().await?;
            println!("Received incoming packet id {}", id);
            // if id >= self.next_receive_id {
            //     return Err(Box::new(ConnectionError::new(format!("Received unexpected packet ID {} from server", id))));
            // }

            self.recv_streams.insert(i, recv);

            //self.receive.insert(*self.receive_packets.get_by_left(&id).unwrap(), SegQueue::new());
            // assert return of above is None
            // TODO: Assert receivers exists
        }
        
        Ok(())
    }
    
    pub fn register_receive_packet<T: Packet + 'static>(&mut self, packet_builder: impl PacketBuilder<T> + 'static + Sync + Send + Copy) -> Result<(), ReceiveError> {
        self.validate_packet_is_new::<T>(false)?;
        let packet_type_id = TypeId::of::<T>();
        self.receive_packets.insert(self.next_receive_id, packet_type_id);
        self.recv_packet_builders.insert(packet_type_id, Box::new(packet_builder));
        self.receive.insert(packet_type_id, SegQueue::new());  // TODO: validate return is None
        
        match self.recv_streams.remove(&self.next_receive_id) {
            None => {
                return Err(ReceiveError::new(format!("recv stream does not exist for packet type={}, id={}.  Did you forget to call init_connection() on your PacketManager?", type_name::<T>(), self.next_receive_id)));
            }
            Some(mut recv_stream) => {
                let (tx, rx) = mpsc::channel(100);

                // TODO: Add receive_thread to rx for validations
                self.rx.insert(packet_type_id, rx);
                let receive_thread = tokio::spawn(async move {
                    let mut partial_chunk: Option<Bytes> = None;
                    loop {
                        // TODO: relay error message
                        // TODO: configurable size limit
                        let chunk = recv_stream.read_chunk(usize::MAX, true).await.unwrap();
                        match chunk {
                            None => {
                                // TODO: Error
                                break;
                            }
                            Some(chunk) => {
                                let bytes;
                                match partial_chunk.take() {
                                    None => {
                                        bytes = chunk.bytes;
                                    }
                                    Some(part) => {
                                        bytes = Bytes::from([part, chunk.bytes].concat());
                                    }
                                }
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
                                            println!("tx whole bytes");
                                            tx.send(frame).await.unwrap();
                                        },
                                        Some(part) => {
                                            println!("tx partial bytes");
                                            let reconstructed_frame = Bytes::from([part, frame].concat());
                                            tx.send(reconstructed_frame).await.unwrap();
                                        }
                                    }
                                    offset = i + FRAME_BOUNDARY.len();
                                }

                                if boundaries.is_empty() || (offset + FRAME_BOUNDARY.len() != bytes.len() - 1) {
                                    println!("Dangling prefix part at end of stream");
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
                });

                self.next_receive_id += 1;
                Ok(())
            }
        }
    }
    
    pub fn register_send_packet<T: Packet + 'static>(&mut self) -> Result<(), ReceiveError> {
        self.validate_packet_is_new::<T>(true)?;
        self.send_packets.insert(self.next_send_id, TypeId::of::<T>());
        self.next_send_id += 1;
        Ok(())
    }
    
    pub async fn received<T: Packet + 'static, U: PacketBuilder<T> + 'static>(&mut self, blocking: bool) -> Result<Option<Vec<T>>, ReceiveError> {
        self.validate_packet_was_registered::<T>(false)?;
        let packet_type_id = TypeId::of::<T>();
        let rx = self.rx.get_mut(&packet_type_id).unwrap();
        let mut res: Vec<T> = Vec::new();
        let packet_builder: &U = self.recv_packet_builders.get(&TypeId::of::<T>()).unwrap().downcast_ref::<U>().unwrap();

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

        Ok(Some(res))
    }
    
    pub async fn send<T: Packet + 'static>(&mut self, packet: T) -> Result<(), SendError> {
        let bytes = packet.to_bytes();
        let packet_type_id = TypeId::of::<T>();
        let id = self.send_packets.get_by_right(&packet_type_id).unwrap();
        let send_stream = self.send_streams.get_mut(id).unwrap();
        send_stream.write_chunk(bytes).await.unwrap();
        send_stream.write_all(FRAME_BOUNDARY).await.unwrap();
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
            return Err(ReceiveError::new("Received empty bytes!"));
        }
        println!("Received packet #{} for type {}", res.len(), type_name::<T>());
        let packet = packet_builder.read(bytes).unwrap();
        res.push(packet);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::time::Duration;

    use bytes::Bytes;
    use tokio::sync::mpsc;
    use tokio::time::sleep;
    use crate::packet::{Packet, PacketBuilder, PacketManager};

    enum MovementPacket {
        TURN(Test),
        STOP(Other)
    }
    
    enum ActionPacket {
        EAT,
        SLEEP
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Test {
        id: i32
    }
    
    impl Packet for Test {
        fn to_bytes(self) -> Bytes {
            Bytes::copy_from_slice(&self.id.to_ne_bytes())
        }
    }
    
    #[derive(Copy, Clone)]
    struct TestBuilder;
    impl PacketBuilder<Test> for TestBuilder {
    
        fn read(&self, bytes: Bytes) -> Result<Test, Box<dyn Error>> {
            Ok(Test{id: i32::from_ne_bytes(bytes[..].try_into().unwrap())})
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct Other {
        name: String,
        id: i32
    }

    impl Packet for Other {
        fn to_bytes(self) -> Bytes {
            Bytes::from([Bytes::from(self.name), Bytes::copy_from_slice(&self.id.to_ne_bytes())].concat())
        }
    }

    #[derive(Copy, Clone)]
    struct OtherBuilder;
    impl PacketBuilder<Other> for OtherBuilder {

        fn read(&self, bytes: Bytes) -> Result<Other, Box<dyn Error>> {
            Ok(Other{name: std::str::from_utf8(&bytes[..bytes.len()-4]).unwrap().to_string(), id: i32::from_ne_bytes(bytes[bytes.len()-4..].try_into().unwrap())})
        }
    }

    #[tokio::test]
    async fn receive_packet_e2e() {
        let mut manager = PacketManager::new();
        
        let (tx, mut rx) = mpsc::channel(100);
        
        // Server
        let server = tokio::spawn(async move {
            let mut m = PacketManager::new();
            m.init_connection(true, 2, 2).await;
            assert!(m.register_send_packet::<Test>().is_ok());
            assert!(m.register_send_packet::<Other>().is_ok());
            assert!(m.register_receive_packet::<Test>(TestBuilder).is_ok());
            assert!(m.register_receive_packet::<Other>(OtherBuilder).is_ok());
            
            for _ in 0..100 {
                assert!(m.send::<Test>(Test { id: 5 }).await.is_ok());
                assert!(m.send::<Test>(Test { id: 8 }).await.is_ok());
                assert!(m.send::<Other>(Other { name: "spoorn".to_string(), id: 4 }).await.is_ok());
                assert!(m.send::<Other>(Other { name: "kiko".to_string(), id: 6 }).await.is_ok());
                
                // TODO: uncomment and debug timeout issue
                let test_res = m.received::<Test, TestBuilder>(true).await;
                assert!(test_res.is_ok());
                let unwrapped = test_res.unwrap();
                assert!(unwrapped.is_some());
                assert_eq!(unwrapped.unwrap(), vec![Test { id: 6 }, Test { id: 9 }]);
                let other_res = m.received::<Other, OtherBuilder>(true).await;
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
        let client = manager.init_connection(false, 2, 2).await;
        println!("{:#?}", client);
        
        assert!(client.is_ok());
        assert!(manager.register_receive_packet::<Test>(TestBuilder).is_ok());
        assert!(manager.register_receive_packet::<Other>(OtherBuilder).is_ok());
        assert!(manager.register_send_packet::<Test>().is_ok());
        assert!(manager.register_send_packet::<Other>().is_ok());
        
        for _ in 0..100 {
            // Send packets
            assert!(manager.send::<Test>(Test { id: 6 }).await.is_ok());
            assert!(manager.send::<Test>(Test { id: 9 }).await.is_ok());
            assert!(manager.send::<Other>(Other { name: "mango".to_string(), id: 1 }).await.is_ok());
            assert!(manager.send::<Other>(Other { name: "luna".to_string(), id: 3 }).await.is_ok());
            
            let test_res = manager.received::<Test, TestBuilder>(true).await;
            assert!(test_res.is_ok());
            let unwrapped = test_res.unwrap();
            assert!(unwrapped.is_some());
            assert_eq!(unwrapped.unwrap(), vec![Test { id: 5 }, Test { id: 8 }]);
            let other_res = manager.received::<Other, OtherBuilder>(true).await;
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