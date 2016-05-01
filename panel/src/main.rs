extern crate mio;

//mod Netserver;

use mio::*;
use std::net::SocketAddr;
use mio::tcp::*;
use std::collections::HashMap;
use std::io::*;
use std::fs::File;


struct Netserver{
    socket: TcpListener,
    clients: HashMap<Token, Printerpart>,
    tokencounter: usize
}

#[derive(Debug)]
#[derive(PartialEq)]
enum Subsystem {
    Printhead,
    Material
}

struct Printerpart{
    socket: TcpStream,
    parttype: Subsystem,
    blueprint: Option<File>
}

impl Printerpart{
    fn new(mut socket: TcpStream) -> Printerpart{
        let mut buf = [0;1];
        let mut ptype;
        loop {
            ptype = match socket.try_read(&mut buf) {
                Err(_) => unreachable!("Error while handshaking with new client"),
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
            parttype: ptype,
            blueprint: None
        }
    }
}

const SERVER_TOKEN: Token = Token(0);
const CLI_TOKEN: Token = Token(1);

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
                    Ok(Some((sock,_))) => sock
                };

                self.tokencounter += 1;
                let new_token = Token(self.tokencounter);

                self.clients.insert(new_token, Printerpart::new(clientsocket));
                eventloop.register(&self.clients[&new_token].socket,
                                    new_token, EventSet::readable(),
                                    PollOpt::edge() | PollOpt::oneshot()).unwrap();
            },
            CLI_TOKEN => {
                let mut input = String::new();
                stdin().read_line(&mut input).unwrap();
                match input.trim() {
                    "p" => {
                        let mut printhead2use : Option<&mut Printerpart> = None;
                        for (_, printpart) in self.clients.iter_mut() {
                            if(printpart.parttype == Subsystem::Printhead
                                && printpart.blueprint.is_none())
                            {
                                printhead2use = Some(printpart);
                                break;
                            }
                        }
                        if(printhead2use.is_none()) {
                            println!("Printhead[s] busy");
                            return;
                        }

                        print3D(printhead2use.unwrap());
                    },
                    "q" => {
                        eventloop.shutdown();
                    },
                    _ => {
                        println!("Unknown input");
                    }
                }
            },
            token => {
                let mut client = self.clients.get_mut(&token).unwrap();

                eventloop.reregister(&client.socket, token, EventSet::readable(),
                                      PollOpt::edge() | PollOpt::oneshot()).unwrap();
            }
        }
    }
}

fn print3D(printhead : &mut Printerpart) {

    //printhead.blueprint = Some(File::open("modell.3dbp").unwrap());
    //Read & check Magic number
    //Read first command
    //Find printhead network connection
    //Send first command to printhead
    //10byte
    let _ = printhead.socket.write(&[1,42,0,0,0,1,1,1,1]);
}

fn continue3Dprint() {

}

fn main() {
    println!("VS-Fab 3D Printer Panel - Ramiz Bahrami(736861), Adrian Müller(734922)");
    println!("Welcome! Your options are: ..."); //TODO

    let mut eventloop = EventLoop::new().unwrap();

    let address = "0.0.0.0:18000".parse::<SocketAddr>().unwrap();
    let mut server = Netserver {
            socket: TcpListener::bind(&address).unwrap(),
            tokencounter : 2,
            clients: HashMap::new()
    };

    eventloop.register(&server.socket,
                        SERVER_TOKEN,
                        EventSet::readable(),
                        PollOpt::edge()).unwrap();

    let stdin = mio::Io::from_raw_fd(0);
    eventloop.register(&stdin,
                        CLI_TOKEN,
                        EventSet::readable(),
                        PollOpt::level()).unwrap();

    eventloop.run(&mut server).unwrap();



    println!("ABC");
}
