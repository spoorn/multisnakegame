use std::any::{Any, type_name, TypeId};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;
use std::time::Duration;
use bevy::utils::label::DynEq;

use bimap::BiMap;
use bytes::Bytes;
use crossbeam_queue::SegQueue;
use hashbrown::HashMap;
use quinn::RecvStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::sleep;
use crate::networking::quinn_helpers::{make_client_endpoint, make_server_endpoint};

pub trait Packet {
    fn to_bytes(self) -> Bytes;
    
    fn to_string(&self) -> String {
        format!("{}", type_name::<Self>())
    }
    
    // https://stackoverflow.com/questions/33687447/how-to-get-a-reference-to-a-concrete-type-from-a-trait-object
    fn as_any(self: Self) -> Box<dyn Any>;
}

pub trait PacketBuilder<T: Packet + 'static> {
    
    fn read(&self, bytes: Bytes) -> Result<T, Box<dyn Error>>;
}

pub trait PacketReceiver {
    type Item: Packet;
    
    fn receive(&mut self, packet: Self::Item) -> Result<(), Box<dyn Error>>;
}

pub trait PacketSender {
    type Item: Packet;
    
    fn get_next_payloads(&mut self) -> Option<&[Self::Item]>; 
}

#[derive(Debug, Clone, thiserror::Error)]
pub struct ConnectionError {
    message: String
}

impl ConnectionError {
    fn new<S: Into<String>>(message: S) -> Self {
        ConnectionError {
            message: message.into()
        }
    }
}

impl Display for ConnectionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Clone)]
pub struct ReceiveError {
    message: String
}

impl ReceiveError {
    fn new<S: Into<String>>(message: S) -> Self {
        ReceiveError {
            message: message.into()
        }
    }
}

impl Display for ReceiveError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Clone)]
pub struct SendError {
    message: String
}

impl SendError {
    fn new<S: Into<String>>(message: S) -> Self {
        SendError {
            message: message.into()
        }
    }
}

impl Display for SendError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub struct PacketManager {
    receive_packets: BiMap<u32, TypeId>,
    send_packets: BiMap<u32, TypeId>,
    receive: HashMap<TypeId, SegQueue<Bytes>>,
    send: HashMap<TypeId, SegQueue<Box<dyn Packet + Send>>>,
    recv_packet_builders: HashMap<TypeId, Box<dyn Any + Send>>,
    recv_streams: HashMap<u32, RecvStream>,
    next_receive_id: u32,
    next_send_id: u32
}

impl PacketManager {
    
    pub fn new() -> Self {
        PacketManager {
            receive_packets: BiMap::new(),
            send_packets: BiMap::new(),
            receive: HashMap::new(),
            send: HashMap::new(),
            recv_packet_builders: HashMap::new(),
            recv_streams: HashMap::new(),
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
                let mut send = conn
                    .open_uni()
                    .await?;
                println!("opened");
                send.write_u32(i).await?;
                println!("write");
            }
            
            loop {
                // Loop to keep server alive
                sleep(Duration::from_millis(10000)).await;
            }
        } else {
            // Bind this endpoint to a UDP socket on the given client address.
            let mut endpoint = make_client_endpoint(client_addr, &[])?;

            // Connect to the server passing in the server name which is supposed to be in the server certificate.
            let connection = endpoint.connect(server_addr, "localhost")?.await?;
            println!("[client] connected: addr={}", connection.remote_address());

            for _ in 0..expected_num_accepts_uni {
                println!("### waiting");
                let mut recv = connection.accept_uni().await?;
                println!("### connection");
                let id = recv.read_u32().await?;
                println!("read {}", id);
                // if id >= self.next_receive_id {
                //     return Err(Box::new(ConnectionError::new(format!("Received unexpected packet ID {} from server", id))));
                // }

                self.recv_streams.insert(id, recv);

                //self.receive.insert(*self.receive_packets.get_by_left(&id).unwrap(), SegQueue::new());
                // assert return of above is None
                // TODO: Assert receivers exists

                // let receive_thread = tokio::spawn(async move {
                //     
                //     loop {
                //         // TODO: relay error message
                //         // TODO: configurable size limit
                //         let chunk = recv.read_chunk(usize::MAX, true).await.unwrap();
                //         match chunk {
                //             None => { break; }
                //             Some(chunk) => {
                //                 let bytes = chunk.bytes;
                //                 let packet_type = self.receive_packets.get_by_left(&0).unwrap().into();
                //                 let packet_builder = self.recv_packet_builders.get(self.receive_packets.get_by_left(&0).unwrap()).unwrap();
                //                 let packet = packet_builder.downcast_ref::<dyn PacketBuilder>();
                //                 //println!("{}", packet.to_string());
                //             }
                //         }
                //     }
                // });
            }

            // while let Ok(mut recv) = connection.accept_uni().await {
            //     loop {
            //         let chunk = recv.read_chunk(usize::MAX, true).await?;
            // 
            //         match chunk {
            //             None => { break; }
            //             Some(chunk) => {
            //                 //let splits: Vec<_> = chunk.bytes.split(|&e| e == b"AAAAA").filter(|v| !v.is_empty()).collect();
            //                 //for split in splits {
            //                 //     let str = std::str::from_utf8(&chunk.bytes).unwrap();
            //                 //     println!("{:?}", str);
            //                 // }
            //             }
            //         }
            //     }
            // 
            //     // Because it is a unidirectional stream, we can only receive not send back.
            //     // let bytes = recv.read_to_end(usize::MAX).await?;
            //     // let str = std::str::from_utf8(&bytes).unwrap();
            //     // println!("{:?}", str);
            // }

            println!("[client] Created connection!");

            // Give the server has a chance to clean up
            //endpoint.wait_idle().await;
        }
        
        Ok(())
    }
    
    pub fn register_receive_packet<T: Packet + 'static>(&mut self, packet_builder: impl PacketBuilder<T> + 'static + Sync + Send + Copy) -> Result<(), ReceiveError> {
        self.validate_packet_is_new::<T>(false)?;
        let packet_type_id = TypeId::of::<T>();
        self.receive_packets.insert(self.next_receive_id, packet_type_id);
        self.recv_packet_builders.insert(packet_type_id, Box::new(packet_builder));
        self.receive.insert(packet_type_id, SegQueue::new());  // TODO: validate return is None

        let arc_packet_builder = Arc::new(packet_builder);
        println!("{:#?}", self.next_receive_id);
        let mut recv_stream = self.recv_streams.remove(&self.next_receive_id).unwrap();
        let receive_thread = tokio::spawn(async move {
            loop {
                // TODO: relay error message
                // TODO: configurable size limit
                let chunk = recv_stream.read_chunk(usize::MAX, true).await.unwrap();
                match chunk {
                    None => { break; }
                    Some(chunk) => {
                        let bytes = chunk.bytes;
                        let packet = arc_packet_builder.read(bytes).unwrap();
                        println!("{}", packet.to_string());
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
    
    pub fn received<T: Packet + 'static, U: PacketBuilder<T> + 'static>(&mut self) -> Result<Option<Vec<T>>, ReceiveError> {
        self.validate_packet_was_registered::<T>(false)?;
        let queue = self.receive.get(&TypeId::of::<T>());
        match queue {
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
    
    pub fn send<T: Packet + 'static>(&mut self, packet: T) -> Result<(), SendError> {
        let bytes = packet.to_bytes();
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
        if (is_send && self.send_packets.contains_right(&TypeId::of::<T>())) || self.receive_packets.contains_right(&TypeId::of::<T>()) {
            return Err(ReceiveError { message: format!("Type '{}' was already registered!", type_name::<T>()) })
        } 
        Ok(())
    }
    
    fn validate_packet_was_registered<T: Packet + 'static>(&self, is_send: bool) -> Result<(), ReceiveError> {
        if is_send {
            if !self.send_packets.contains_right(&TypeId::of::<T>()) {
                return Err(ReceiveError { message: format!("Type '{}' was never registered!  Did you forget to call register_send_packet()?", type_name::<T>()) })

            }
        } else {
            if !self.receive_packets.contains_right(&TypeId::of::<T>()) {
                return Err(ReceiveError { message: format!("Type '{}' was never registered!  Did you forget to call register_receive_packet()?", type_name::<T>()) })
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::error::Error;
    use bytes::Bytes;

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

        fn as_any(self: Self) -> Box<dyn Any> {
            Box::new(self)
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

        fn as_any(self: Self) -> Box<dyn Any> {
            Box::new(self)
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
        });
        let client = manager.init_connection(false, 2).await;
        println!("{:#?}", client);
        
        assert!(client.is_ok());
        assert!(manager.register_receive_packet::<Test>(TestBuilder).is_ok());
        assert!(manager.register_receive_packet::<Other>(OtherBuilder).is_ok());
        manager.receive_packet::<Test>(Test { id: 5 });
        manager.receive_packet::<Test>(Test { id: 8 });
        manager.receive_packet::<Other>(Other { name: "spoorn".to_string(), id: 4 });
        manager.receive_packet::<Other>(Other { name: "kiko".to_string(), id: 6 });
        let test_res = manager.received::<Test, TestBuilder>();
        assert!(test_res.is_ok());
        let unwrapped = test_res.unwrap();
        assert!(unwrapped.is_some());
        assert_eq!(unwrapped.unwrap(), vec![Test { id: 5 }, Test { id: 8 }]);
        let other_res = manager.received::<Other, OtherBuilder>();
        assert!(other_res.is_ok());
        let unwrapped = other_res.unwrap();
        assert!(unwrapped.is_some());
        assert_eq!(unwrapped.unwrap(), vec![Other { name: "spoorn".to_string(), id: 4 }, Other { name: "kiko".to_string(), id: 6 }]);
    }
    
    #[test]
    fn test_register_receive_packet() {
        let mut manager = PacketManager::new();
        assert!(manager.validate_packet_is_new::<Test>(false).is_ok());
        assert!(manager.register_receive_packet::<Test>(TestBuilder).is_ok());
        assert!(manager.validate_packet_is_new::<Test>(false).is_err());
        assert!(manager.register_receive_packet::<Test>(TestBuilder).is_err());
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

// pub struct HandleRegister {
//     receivers: HashMap<u32, Box<dyn PacketReceiver<Item=dyn Packet>>>,
//     senders: Vec<(u32, Box<dyn PacketSender<Item=dyn Packet>>)>,
//     next_receiver_id: u32,
// }
// 
// impl HandleRegister {
//     pub fn new() -> Self {
//         HandleRegister {
//             receivers: HashMap::new(),
//             senders: Vec::new(),
//             next_receiver_id: 0
//         }
//     }
//     
//     pub fn register_receiver(&mut self, receiver: impl PacketReceiver) -> Result<u32, Box<dyn Error>> {
//         let receiver_id = self.next_receiver_id;
//         self.receivers.insert(receiver_id, Box::new(receiver));
//         self.next_receiver_id += 1;
//         Ok(receiver_id)
//     }
//     
//     pub fn register_sender(&mut self, sender: impl PacketSender) -> Result<u32, Box<dyn Error>> {
//         return Ok(0)
//     }
// }