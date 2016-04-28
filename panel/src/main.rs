extern crate mio;

//mod Netserver;

use std::thread;
use mio::*;
use std::net::SocketAddr;
use mio::tcp::*;
use std::collections::HashMap;


struct Netserver{
    socket: TcpListener,
    clients: HashMap<Token, Printerpart>,
    tokencounter: usize
}

#[derive(Debug)]
enum Subsystem {
    Invalid,
    Printhead,
    Material
}

struct Printerpart{
    socket: TcpStream,
    parttype: Subsystem
}

impl Printerpart{
    fn new(mut socket: TcpStream) -> Printerpart{
        let mut buf = [0;1];
        let mut ptype = Subsystem::Invalid;
        loop {
            ptype = match socket.try_read(&mut buf) {
                Err(e) => unreachable!("Error while handshaking with new client"),
                Ok(None) => continue,
                Ok(Some(_)) => {
                    match buf[0] {
                        1 => Subsystem::Printhead,
                        2 => Subsystem::Material,
                        _ => unreachable!("Error while handshaking with new client")
                    }
                }
            };
            break;
        };
        println!("{:?}",ptype);
        Printerpart {
            socket: socket,
            parttype: ptype
        }
    }
}

const SERVER_TOKEN: Token = Token(0);

impl Handler for Netserver {

    type Timeout = usize;
    type Message = ();

    fn ready(&mut self, eventloop: &mut EventLoop<Netserver>,
             token: Token, events: EventSet)
    {
        match token {
            SERVER_TOKEN => {
                let clientsocket = match self.socket.accept() {
                    Err(e) => {
                        println!("Accept error: {}", e);
                        return;
                    },
                    Ok(None) => unreachable!("Accept has returned 'None'"),
                    Ok(Some((sock, addr))) => sock
                };

                self.tokencounter += 1;
                let new_token = Token(self.tokencounter);

                self.clients.insert(new_token, Printerpart::new(clientsocket));
                eventloop.register(&self.clients[&new_token].socket,
                                    new_token, EventSet::readable(),
                                    PollOpt::edge() | PollOpt::oneshot()).unwrap();
            }
            token => {
                let mut client = self.clients.get_mut(&token).unwrap();
                eventloop.reregister(&client.socket, token, EventSet::readable(),
                                      PollOpt::edge() | PollOpt::oneshot()).unwrap();
            }
        }
    }
}



fn main() {
    println!("VS-Fab 3D Printer Panel - Ramiz Bahrami(736861), Adrian Müller(734922)");

    let mut eventloop = EventLoop::new().unwrap();

    let address = "0.0.0.0:18000".parse::<SocketAddr>().unwrap();
    let mut server = Netserver {
            socket: TcpListener::bind(&address).unwrap(),
            tokencounter : 1,
            clients: HashMap::new()
    };

    eventloop.register(&server.socket,
                        SERVER_TOKEN,
                        EventSet::readable(),
                        PollOpt::edge()).unwrap();


    eventloop.run(&mut server).unwrap();



    println!("ABC");
}
