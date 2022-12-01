use std::any::{Any, type_name, TypeId};
use std::error::Error;
use std::fmt::{Debug, Display};
use std::time::Duration;

use bevy::utils::label::DynEq;
use bimap::BiMap;
use bytes::Bytes;
use crossbeam_queue::SegQueue;
use derive_more::Display;
use hashbrown::HashMap;
use quinn::{RecvStream, SendStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::time::sleep;

use networking::ErrorMessageNew;

use crate::networking::quinn_helpers::{make_client_endpoint, make_server_endpoint};

const FRAME_BOUNDARY: &[u8] = b"AAAAAA031320050421";

pub trait Packet {
    fn to_bytes(self) -> Bytes;
    
    // https://stackoverflow.com/questions/33687447/how-to-get-a-reference-to-a-concrete-type-from-a-trait-object
    // fn as_any(self: Self) -> Box<dyn Any>;
}

pub trait PacketBuilder<T: Packet + 'static> {
    
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
    //send: HashMap<TypeId, SegQueue<Box<dyn Packet + Send>>>,
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

    pub async fn init_connection(&mut self, is_server: bool, expected_num_accepts_uni: u32) -> Result<(), Box<dyn Error>> {
        // if expected_num_accepts_uni != self.next_receive_id {
        //     return Err(Box::new(ConnectionError::new("expected_num_accepts_uni does not match number of registered receive packets")));
        // }
        let server_addr = "127.0.0.1:5000".parse().unwrap();
        let client_addr = "127.0.0.1:5001".parse().unwrap();
        
        if is_server {
            let (endpoint, server_cert) = make_server_endpoint(server_addr)?;

            // Single connection
            let incoming_conn = endpoint.accept().await.unwrap();
            let conn = incoming_conn.await.unwrap();
            println!("[server] connection accepted: addr={}", conn.remote_address());

            // Note: Packets are not sent immediately upon the write.  The thread needs to be kept
            // open so that the packets can actually be sent over the wire to the client.
            for i in 0..expected_num_accepts_uni {
                let mut send_stream = conn
                    .open_uni()
                    .await?;
                println!("opened");
                send_stream.write_u32(i).await?;
                println!("write");
                self.send_streams.insert(i, send_stream);
                
                // let (tx, mut rx) = mpsc::channel(100);
                // // TODO: Add tx to senders
                // self.send.insert(i, tx);
                // let send_thread = tokio::spawn(async move {
                //     loop {
                //         println!("### SEND THREAD");
                //         match rx.try_recv() {
                //             Ok(bytes) => {
                //                 println!("Writing bytes");
                //                 send_stream.write_chunk(bytes).await.unwrap();
                //             }
                //             Err(e) => {
                //                 match e {
                //                     TryRecvError::Empty => {
                //                         sleep(Duration::from_millis(1000)).await;
                //                     }
                //                     TryRecvError::Disconnected => {
                //                         return Box::new(SendError::new(format!("Packet sender disconnected for packet {}", i)));
                //                     }
                //                 }
                //             }
                //         }
                //     } 
                // });
            }
            
            // loop {
            //     // Loop to keep server alive
            //     sleep(Duration::from_millis(10000)).await;
            // }
        } else {
            // Bind this endpoint to a UDP socket on the given client address.
            let mut endpoint = make_client_endpoint(client_addr, &[])?;

            // Connect to the server passing in the server name which is supposed to be in the server certificate.
            let connection = endpoint.connect(server_addr, "localhost")?.await?;
            println!("[client] connected: addr={}", connection.remote_address());

            for i in 0..expected_num_accepts_uni {
                println!("### waiting");
                let mut recv = connection.accept_uni().await?;
                println!("### connection");
                let id = recv.read_u32().await?;
                println!("read {}", id);
                // if id >= self.next_receive_id {
                //     return Err(Box::new(ConnectionError::new(format!("Received unexpected packet ID {} from server", id))));
                // }

                self.recv_streams.insert(i, recv);

                //self.receive.insert(*self.receive_packets.get_by_left(&id).unwrap(), SegQueue::new());
                // assert return of above is None
                // TODO: Assert receivers exists
            }

            println!("[client] Created connection!");
        }
        
        Ok(())
    }
    
    pub fn register_receive_packet<T: Packet + 'static>(&mut self, packet_builder: impl PacketBuilder<T> + 'static + Sync + Send + Copy) -> Result<(), ReceiveError> {
        self.validate_packet_is_new::<T>(false)?;
        let packet_type_id = TypeId::of::<T>();
        self.receive_packets.insert(self.next_receive_id, packet_type_id);
        self.recv_packet_builders.insert(packet_type_id, Box::new(packet_builder));
        self.receive.insert(packet_type_id, SegQueue::new());  // TODO: validate return is None
        
        let mut recv_stream = self.recv_streams.remove(&self.next_receive_id).unwrap();
        let (tx, rx) = mpsc::channel(100);
        
        // TODO: Add receive_thread to rx for validations
        self.rx.insert(packet_type_id, rx);
        let receive_thread = tokio::spawn(async move {
            let mut partial_chunk: Option<Bytes> = None;
            loop {
                println!("### RECEIVE THREAD");
                // TODO: relay error message
                // TODO: configurable size limit
                let chunk = recv_stream.read_chunk(usize::MAX, true).await.unwrap();
                println!("### GOT DATA");
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
    
    pub fn register_send_packet<T: Packet + 'static>(&mut self) -> Result<(), ReceiveError> {
        self.validate_packet_is_new::<T>(true)?;
        self.send_packets.insert(self.next_send_id, TypeId::of::<T>());
        self.next_send_id += 1;
        Ok(())
    }
    
    pub async fn received<T: Packet + 'static, U: PacketBuilder<T> + 'static>(&mut self, blocking: bool) -> Result<Option<Vec<T>>, ReceiveError> {
        self.validate_packet_was_registered::<T>(false)?;
        let packet_type_id = TypeId::of::<T>();
        let queue = self.receive.get(&packet_type_id);
        if queue.is_none() {
            let queue: SegQueue<Bytes> = SegQueue::new();
            self.receive.insert(TypeId::of::<T>(), queue);
        }
        let queue = self.receive.get(&packet_type_id).unwrap();
        // TODO: simplify
        let rx = self.rx.get_mut(&packet_type_id).unwrap();
        loop {
            let receiver = self.recv_packet_builders.remove(&TypeId::of::<T>()).unwrap();
            println!("try receive");
            match rx.try_recv() {
                Ok(bytes) => {
                    println!("Received bytes");
                    queue.push(bytes);
                    self.recv_packet_builders.insert(packet_type_id, receiver);
                }
                Err(e) => {
                    self.recv_packet_builders.insert(packet_type_id, receiver);
                    match e {
                        TryRecvError::Empty => {
                            // TODO: allow blocking
                            if blocking && queue.is_empty() {
                                println!("sleeping");
                                // Have to use tokio's sleep so it can yield to the tokio executor
                                // https://stackoverflow.com/questions/70798841/why-does-a-tokio-thread-wait-for-a-blocking-thread-before-continuing?rq=1
                                sleep(Duration::from_millis(1000)).await;
                                println!("woke up");
                            } else {
                                break;
                            }
                        }
                        TryRecvError::Disconnected => {
                            return Err(ReceiveError::new(format!("Receiver channel for type {} was disconnected", type_name::<T>())));
                        }
                    }
                }
            }
        }
        
        match self.receive.get(&packet_type_id) {
            None => { return Err(ReceiveError::new(format!("Receive queue did not contain type {}", type_name::<T>()))); }
            Some(packets) => {
                let size = packets.len();
                if size == 0 {
                    return Ok(None);
                }
                
                let mut res: Vec<T> = Vec::new();
                let receiver = self.recv_packet_builders.remove(&TypeId::of::<T>()).unwrap();
                for _ in 0..size {
                    let e = packets.pop();
                    match e {
                        None => { return Err(ReceiveError::new(format!("Queue for type {} contained empty Packet", type_name::<T>()))) }
                        Some(packet_bytes) => {
                            let packet_builder: &U = receiver.downcast_ref::<U>().unwrap();
                            let packet = packet_builder.read(packet_bytes).unwrap();
                            res.push(packet);
                            // if p.is::<T>() {
                            //     let x = p.downcast::<T>().unwrap();
                            //     res.push(*x);
                            // } else {
                            //     return Err(ReceiveError::new("Packet was of incorrect type, this should not have happened!"))
                            // }
                           
                        }
                    }
                }
                
                self.recv_packet_builders.insert(TypeId::of::<T>(), receiver);
                Ok(Some(res))
            }
        }
    }
    
    pub async fn send<T: Packet + 'static>(&mut self, packet: T) -> Result<(), SendError> {
        let bytes = packet.to_bytes();
        let packet_type_id = TypeId::of::<T>();
        let id = self.send_packets.get_by_right(&packet_type_id).unwrap();
        let send_stream = self.send_streams.get_mut(id).unwrap();
        send_stream.write_chunk(bytes).await.unwrap();
        send_stream.write_all(FRAME_BOUNDARY).await.unwrap();
        println!("Sent packet");
        Ok(())
    }
    
    fn receive_packet<T: Packet + 'static>(&mut self, packet: T) {
        let validate = self.validate_packet_was_registered::<T>(false);
        if let Err(e) = validate {
            panic!("{}", e.message);
        }
        match self.receive.get(&TypeId::of::<T>()) {
            None => {
                let queue: SegQueue<Bytes> = SegQueue::new();
                queue.push(packet.to_bytes());
                self.receive.insert(TypeId::of::<T>(), queue);
            }
            Some(queue) => {
                queue.push(packet.to_bytes());
            }
        }
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
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::time::Duration;

    use bytes::Bytes;
    use tokio::time::sleep;

    use crate::networking::packet::{Packet, PacketBuilder, PacketManager};

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
        let server = tokio::spawn(async {
            let mut m = PacketManager::new();
            m.init_connection(true, 2).await;
            println!("Done init server");
            assert!(m.register_send_packet::<Test>().is_ok());
            assert!(m.register_send_packet::<Other>().is_ok());
            assert!(m.send::<Test>(Test { id: 5 }).await.is_ok());
            assert!(m.send::<Test>(Test { id: 8 }).await.is_ok());
            assert!(m.send::<Other>(Other { name: "spoorn".to_string(), id: 4 }).await.is_ok());
            assert!(m.send::<Other>(Other { name: "kiko".to_string(), id: 6 }).await.is_ok());
            loop {
                sleep(Duration::from_millis(10000)).await;
            }
        });
        let client = manager.init_connection(false, 2).await;
        println!("{:#?}", client);
        
        assert!(client.is_ok());
        assert!(manager.register_receive_packet::<Test>(TestBuilder).is_ok());
        assert!(manager.register_receive_packet::<Other>(OtherBuilder).is_ok());
        
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

    #[test]
    fn test_register_send_packet() {
        let mut manager = PacketManager::new();
        assert!(manager.validate_packet_is_new::<Test>(true).is_ok());
        assert!(manager.register_send_packet::<Test>().is_ok());
        assert!(manager.validate_packet_is_new::<Test>(true).is_err());
        assert!(manager.register_send_packet::<Test>().is_err());
    }
}