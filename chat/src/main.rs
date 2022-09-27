use byteorder::{ByteOrder, NetworkEndian};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::env;
use std::error;
use std::io::Read;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;
use std::thread;

#[derive(Debug)]
struct Message {
    from: u16,
    to: u16,
    content: Vec<u8>,
}

#[derive(Debug)]
enum Packet {
    UserList,
    Say(Message),
}

#[derive(Debug)]
enum MyError {
    UnknownPacketType,
    PacketTooLong,
}

impl std::error::Error for MyError {}

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            MyError::UnknownPacketType => "unknown packet type",
            MyError::PacketTooLong => "packet too long",
        };
        write!(f, "{}", msg)
    }
}

lazy_static! {
    static ref NEXT_USER_ID: AtomicU16 = AtomicU16::new(1);
    static ref SESSIONS: Mutex<HashMap<u16, TcpStream>> = Mutex::new(HashMap::new());
}

fn decode_packet(buf: &[u8]) -> Result<Packet, Box<dyn error::Error>> {
    let packet_type = buf[2];
    match packet_type {
        0 => Ok(Packet::UserList),
        1 => Ok(Packet::Say(Message {
            from: NetworkEndian::read_u16(&buf[3..5]),
            to: NetworkEndian::read_u16(&buf[5..7]),
            content: (&buf[7..]).into(),
        })),
        _ => Err(Box::new(MyError::UnknownPacketType)),
    }
}

fn encode_packet(packet: &Packet, buf: &mut [u8]) -> usize {
    let (length, packet_type) = match packet {
        Packet::UserList => (1, 0),
        Packet::Say(ref message) => {
            NetworkEndian::write_u16(&mut buf[3..5], message.from);
            NetworkEndian::write_u16(&mut buf[5..7], message.to);
            (&mut buf[7..message.content.len() + 7]).copy_from_slice(&message.content);
            (1 + 2 + 2 + message.content.len() as u16, 1)
        }
    };

    NetworkEndian::write_u16(&mut *buf, length);
    buf[2] = packet_type;
    (length + 2) as usize
}

fn receive_packet(stream_receive: &mut TcpStream) -> Result<Packet, Box<dyn error::Error>> {
    let mut buf: Box<[u8; 1024]> = Box::new([0; 1024]);
    stream_receive.read_exact(&mut buf[0..2])?;
    let packet_length = NetworkEndian::read_u16(&buf[0..2]);
    if (packet_length + 2) > 1024 {
        return Err(Box::new(MyError::PacketTooLong));
    }
    stream_receive.read_exact(&mut buf[2..(packet_length + 2) as usize])?;

    decode_packet(&buf[0..(packet_length + 2) as usize])
}

fn server_main() {
    let listerner = TcpListener::bind("0.0.0.0:2319").unwrap();
    let (tx, rx): (Sender<Message>, Receiver<Message>) = mpsc::channel();

    thread::spawn(move || {
        for message in rx.into_iter() {
            let p = Packet::Say(message);
            let mut buf: Box<[u8; 1024]> = Box::new([0; 1024]);
            let packet_length = encode_packet(&p, &mut *buf);
            let s = SESSIONS.lock().unwrap();

            for (_, mut stream) in &*s {
                if let Err(error) = stream.write_all(&(*buf)[0..packet_length]) {
                    println!("ERROR writing to client: {}", error);
                }
            }
        }
    });

    for stream in listerner.incoming() {
        match stream {
            Ok(mut stream) => {
                let user_id = NEXT_USER_ID.fetch_add(1, Ordering::SeqCst);
                {
                    let mut s = SESSIONS.lock().unwrap();
                    s.insert(user_id, stream.try_clone().unwrap());
                }

                let tx = tx.clone();

                thread::spawn(move || loop {
                    let packet = receive_packet(&mut stream);
                    match packet {
                        Ok(packet) => match packet {
                            Packet::UserList => {}
                            Packet::Say(mut message) => {
                                message.from = user_id;
                                tx.send(message).unwrap();
                            }
                        },
                        Err(error) => {
                            let mut s = SESSIONS.lock().unwrap();
                            s.remove(&user_id);
                            println!("ERROR receiving packet: {}", error);
                            return;
                        }
                    }
                });
            }
            Err(e) => {
                println!("Error stream: {}", e);
            }
        }
    }
}

fn client_main() {
    match TcpStream::connect("localhost:2319") {
        Ok(mut stream) => {
            println!("Connected to server.");

            let mut stream_receive = stream.try_clone().unwrap();
            thread::spawn(move || loop {
                let packet = receive_packet(&mut stream_receive);
                match packet {
                    Ok(packet) => match packet {
                        Packet::UserList => {}
                        Packet::Say(message) => {
                            println!(
                                "[{} to {}: {}]",
                                message.from,
                                message.to,
                                String::from_utf8(message.content).unwrap()
                            );
                            eprint!(">>>");
                        }
                    },
                    Err(error) => {
                        println!("ERROR receiving packet: {}", error);
                        return;
                    }
                }
            });

            loop {
                eprint!(">>>");
                let mut content = String::new();
                std::io::stdin().read_line(&mut content).unwrap();
                let content = content.trim();
                if content.len() == 0 {
                    continue;
                }
                let p = Packet::Say(Message {
                    from: 0,
                    to: 0,
                    content: content.as_bytes().to_vec(),
                });

                let mut buf: Box<[u8; 1024]> = Box::new([0; 1024]);
                let packet_length = encode_packet(&p, &mut *buf);

                if let Err(error) = stream.write_all(&(*buf)[0..packet_length]) {
                    println!("ERROR sending to server: {}", error);
                    return;
                }
            }
        }
        Err(e) => {
            println!("ERROR connecting: {}", e);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && args[1] == "-s" {
        server_main();
    }
    client_main();
}
