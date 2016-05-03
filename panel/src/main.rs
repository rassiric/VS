extern crate mio;

use mio::*;
use std::net::SocketAddr;
use mio::tcp::*;
use std::collections::HashMap;
use std::io::*;
use std::fs::File;
use std::time::Duration;

const SERVER_TOKEN: Token = Token(0);
const CLI_TOKEN: Token = Token(1);
const PRINT_TIMEOUT_MS : u64 = 10000;
const CONTINUE_DELAY_MS : u64 = 1000;

struct Netserver{
    socket: TcpListener,
    clients: HashMap<Token, Printerpart>,
    tokencounter: usize,
    continuedelay: Option<mio::Timeout>
}

#[derive(Debug)]
#[derive(PartialEq)]
enum Subsystem {
    Printhead,
    Material
}

struct Printerpart {
    id: usize,
    socket: TcpStream,
    parttype: Subsystem,
    blueprint: Option<File>,
    timeoutid: Option<mio::Timeout>,
    matempty: bool
}

impl Printerpart {
    fn new(mut socket: TcpStream) -> Printerpart{
        let mut buf = [0;1];
        let ptype;
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
            id: 0,
            socket: socket,
            parttype: ptype,
            blueprint: None,
            timeoutid: None,
            matempty: false
        }
    }
}

impl Netserver {
    fn check_mat_status(&self) -> bool {
        for part in self.clients.values() {
            if part.parttype == Subsystem::Material && part.matempty {
                return false;
            }
        }
        return true;
    }
}

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
                self.clients.get_mut(&new_token).unwrap().id = self.tokencounter; //inform Printerpart about its ID
                eventloop.register(&self.clients[&new_token].socket,
                                    new_token, EventSet::readable(),
                                    PollOpt::edge()).unwrap();
            },
            CLI_TOKEN => {
                let mut input = String::new();
                stdin().read_line(&mut input).unwrap();
                match input.trim() {
                    "p" => {
                        let mut printhead2use : Option<&mut Printerpart> = None;
                        if !self.check_mat_status() {
                            println!("Job discarded: Please refill material containers first!");
                            return;
                        }

                        for (_, printpart) in self.clients.iter_mut() {
                            if printpart.parttype == Subsystem::Printhead
                                && printpart.blueprint.is_none()
                            {
                                printhead2use = Some(printpart);
                                break;
                            }
                        }
                        if printhead2use.is_none() {
                            println!("Printhead[s] busy");
                            return;
                        }
                        print3d(printhead2use.unwrap(), eventloop);
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
                let mat_available = self.check_mat_status();
                let mut client = self.clients.get_mut(&token).unwrap();
                let mut buf = [0];
                let result;
                loop {
                    result = match client.socket.try_read(&mut buf) {
                        Err(_) => unreachable!("Error while receiving client data"),
                        Ok(None) => continue,
                        Ok(Some(_)) => buf[0]
                    };
                    break;
                };
                if client.parttype == Subsystem::Printhead {
                    eventloop.clear_timeout(&client.timeoutid.as_mut().expect("Unexpected printhead message!"));
                    client.timeoutid = None;
                    match result {
                        1 => {
                            if mat_available {
                                continue3dprint(client, eventloop);
                            }
                            else {
                                println!("Pausing print until material is refilled");
                            }
                        },
                        255 => {
                            println!("Printhead problem, aborting print");
                            client.blueprint = None;
                        },
                        _ => unreachable!("Unknown printhead status!")
                    };
                } else {
                    match result {
                        255 => {
                            println!("Material container {} is nearly empty, pausing...",token.as_usize());
                            client.matempty = true;
                        },
                        1 => {
                            println!("Material container {} refilled",token.as_usize());
                            client.matempty = false;
                            if self.continuedelay.is_some() {
                                eventloop.clear_timeout(self.continuedelay.as_mut().expect(""));
                            }
                            self.continuedelay = Some(eventloop.timeout(0,
                                Duration::from_millis(CONTINUE_DELAY_MS)).unwrap());
                        },
                        _ => unreachable!("Unknown material status!")
                    }
                }
            }
        }
    }

    fn timeout(&mut self, eventloop: &mut EventLoop<Netserver>, timeout_token: usize) {
        match(timeout_token) {
            0 => { //Timeout id 0 is check for continue
                if self.check_mat_status() {
                    println!("All material containers refilled!");
                    for (_, printerpart) in self.clients.iter_mut() {
                        if printerpart.parttype == Subsystem::Printhead &&
                            printerpart.blueprint.is_some() {
                            println!("Continuing on printhead {}", printerpart.id );
                            continue3dprint(printerpart, eventloop);
                        }
                    }
                }
                else {
                    println!("Still missing material...");
                }
            }
            _ => {
                println!("Timeout while printing, aborting...");
                self.clients.get_mut(&Token(timeout_token)).unwrap().blueprint = None;
            }
        };
    }
}

fn print3d(printhead : &mut Printerpart, eventloop: &mut EventLoop<Netserver>) {
    printhead.blueprint = Some(File::open("modell.3dbp").unwrap());

    //Read & check Magic number
    let mut magic = [0;4];
    printhead.blueprint.as_mut().expect("").read_exact(&mut magic).unwrap();
    for i in 0..4 {
        if magic[i] != b"RBAM"[i] {
            unreachable!("Invalid blueprint magic");
        }
    }
    //b vor string macht ascii byte array literal

    continue3dprint(printhead, eventloop);
}

fn continue3dprint(printhead : &mut Printerpart, eventloop: &mut EventLoop<Netserver>) {
    //ToDo: check if currently printing
    //bei UDP: aktuellen Befehl hier beim printhead speichern, um erneut senden zu können
    let mut commandid = [0;1];

    match printhead.blueprint.as_mut().expect("No blueprint in progess!").read_exact(&mut commandid) {
        Err(_) => {
            println!("Blueprint finished!");
            printhead.blueprint = None;
            return
        },
        _ => {}
    }
    let mut bp = printhead.blueprint.as_mut().expect("No blueprint in progess!");

    printhead.socket.write(&commandid).unwrap();

    match commandid[0] {
        1 | 2 => { //8byte params
             let mut params = [0;8];
             bp.read_exact(&mut params).unwrap();
             printhead.socket.write(&params).unwrap();
        },
        3 => { //16byte params
            let mut params = [0;16];
            bp.read_exact(&mut params).unwrap();
            printhead.socket.write(&params).unwrap();
        }
        _ => {
            unreachable!("Unknown blueprint command");
        }
    };;

    printhead.timeoutid = Some(eventloop.timeout(printhead.id, Duration::from_millis(PRINT_TIMEOUT_MS)).unwrap());
}

fn main() {
    println!("VS-Fab 3D Printer Panel - Ramiz Bahrami(736861), Adrian Müller(734922)");
    println!("Welcome! Your options are: ..."); //TODO

    let mut eventloop = EventLoop::new().unwrap();

    let address = "0.0.0.0:18000".parse::<SocketAddr>().unwrap();
    let mut server = Netserver {
            socket: TcpListener::bind(&address).unwrap(),
            tokencounter : 2,
            clients: HashMap::new(),
            continuedelay: None
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
