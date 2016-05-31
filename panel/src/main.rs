extern crate mio;
extern crate time;

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

static mut BenchWatchStopTime : u64 = 0;

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
    matempty: bool,
    matid: i32,
    benchmarkcnt: i32
}

impl Printerpart {
    fn new(mut socket: TcpStream) -> Printerpart{
        let mut buf = [0];
        let ptype;
        loop {
            ptype = match socket.try_read(&mut buf) {
                Err(_) => unreachable!("Error while handshaking with new client"),
                Ok(None) => continue,
                Ok(Some(_)) => {
                    match buf[0] {
                        0 => unreachable!("Sth tried to register as an invalid printerpart!"),
                        1 => Subsystem::Printhead,
                        _ => Subsystem::Material
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
            matempty: false,
            matid: (buf[0] as i32) - 2,
            benchmarkcnt: 0
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
             token: Token, _: EventSet)
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
                        let matreq : u8;
                        let matid : i32;

                        {
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

                            match printhead2use {
                                None => {
                                    println!("Printhead[s] busy");
                                    return;
                                },
                                Some(mut prnt_head) => {
                                    println!("Sending job to printhead({})", prnt_head.id);
                                    load_blueprint(prnt_head, eventloop);

                                    matid = prnt_head.matid;
                                    matreq = continue3dprint(prnt_head, eventloop);
                                }
                            }

                        }
                        if matreq > 0 {
                            let mut matcontainer : Option<&mut Printerpart> = None;
                            for (_, printpart) in self.clients.iter_mut() {
                                if printpart.parttype == Subsystem::Material
                                    && printpart.matid == matid
                                {
                                    matcontainer = Some(printpart);
                                    break;
                                }
                            }
                            if matcontainer.is_none() {
                                unreachable!("Material not available!");
                            }
                            matcontainer.unwrap().socket.write(&[matreq]).unwrap();
                        }
                    },
                    "b" => {
                        let mut printhead2use : Option<&mut Printerpart> = None;

                        for (_, printpart) in self.clients.iter_mut() {
                            if printpart.parttype == Subsystem::Printhead
                                && printpart.blueprint.is_none()
                            {
                                printhead2use = Some(printpart);
                                break;
                            }
                        }

                        match printhead2use {
                            None => {
                                println!("Printhead[s] busy");
                                return;
                            },
                            Some(mut prnt_head) => {
                                println!("Benchmarking printhead({})", prnt_head.id);
                                prnt_head.benchmarkcnt = 10000;
                                unsafe{BenchWatchStopTime = time::precise_time_ns();}
                                prnt_head.socket.write(&[1,57,5,0,0,0]).unwrap();
                                prnt_head.timeoutid = Some(eventloop.timeout(prnt_head.id, Duration::from_millis(PRINT_TIMEOUT_MS)).unwrap());
                            }
                        }
                    }
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

                let mut matreq : u8 = 0;
                let mut matid :i32;
                {
                    let mut client = self.clients.get_mut(&token).unwrap();
                    let mut buf = [0];
                    matid = client.matid;
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
                        matid = client.matid;
                        if client.benchmarkcnt > 0 {
                            client.benchmarkcnt -= 1;
                            if client.benchmarkcnt == 0 {
                                unsafe{println!("Benchmark finished, time: {}ms",
                                    ((time::precise_time_ns() - BenchWatchStopTime) as f32)/1000000.0);}
                                return;
                            }
                            client.socket.write(&[1,57,5,0,0,0]).unwrap();
                            client.timeoutid = Some(eventloop.timeout(client.id, Duration::from_millis(PRINT_TIMEOUT_MS)).unwrap());
                            return;
                        }
                        matreq = match result {
                            1 => {
                                if mat_available {
                                    continue3dprint(client, eventloop)
                                }
                                else {
                                    println!("Pausing print until material is refilled");
                                    0
                                }
                            },
                            255 => {
                                println!("Printhead problem, aborting print");
                                client.blueprint = None;
                                0
                            },
                            _ => unreachable!("Unknown printhead status!")
                        };
                    } else {
                        match result {
                            255 => {
                                println!("Material container {} is nearly empty, pausing...",client.matid);
                                client.matempty = true;
                            },
                            1 => {
                                println!("Material container {} refilled",client.matid);
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
                if matreq > 0 {
                    let mut matcontainer : Option<&mut Printerpart> = None;
                    for (_, printpart) in self.clients.iter_mut() {
                        if printpart.parttype == Subsystem::Material
                            && printpart.matid == matid
                        {
                            matcontainer = Some(printpart);
                            break;
                        }
                    }
                    if matcontainer.is_none() {
                        unreachable!("Material not available!");
                    }
                    matcontainer.unwrap().socket.write(&[matreq]).unwrap();
                }
            }
        }
    }

    fn timeout(&mut self, eventloop: &mut EventLoop<Netserver>, timeout_token: usize) {
        match timeout_token {
            0 => { //Timeout id 0 is check for continue
                let mut matreq :u8 = 0;
                let mut matid :u8 =0;
                if self.check_mat_status() {
                    println!("All material containers refilled!");
                    for (_, printerpart) in self.clients.iter_mut() {
                        if printerpart.parttype == Subsystem::Printhead &&
                            printerpart.blueprint.is_some() {

                            println!("Continuing on printhead {}", printerpart.id );
                            matreq = continue3dprint(printerpart, eventloop);
                            matid = printerpart.matid as u8;
                        }
                    }
                }
                else {
                    println!("Still missing material...");
                }
                if matreq > 0 {
                    let mut matcontainer : Option<&mut Printerpart> = None;
                    for (_, printpart) in self.clients.iter_mut() {
                        if printpart.parttype == Subsystem::Material
                            && printpart.matid == matid as i32
                        {
                            matcontainer = Some(printpart);
                            break;
                        }
                    }
                    if matcontainer.is_none() {
                        unreachable!("Material not available!");
                    }
                    matcontainer.unwrap().socket.write(&[matreq]).unwrap();
                }
            }
            _ => {
                println!("Timeout while printing, aborting...");
                let mut connection = self.clients.get_mut(&Token(timeout_token)).unwrap();
                connection.blueprint = None; //Abort print process
                connection.benchmarkcnt = 0; //Abort benchmark process
            }
        };
    }
}

fn load_blueprint(printhead : &mut Printerpart, _: &mut EventLoop<Netserver>) {
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
}

fn continue3dprint(printhead : &mut Printerpart, eventloop: &mut EventLoop<Netserver>) -> u8{
    //ToDo: check if currently printing
    //bei UDP: aktuellen Befehl hier beim printhead speichern, um erneut senden zu können
    let mut commandid = [0];

    match printhead.blueprint.as_mut().expect("No blueprint in progess!").read_exact(&mut commandid) {
        Err(_) => {
            println!("Blueprint finished!");
            printhead.blueprint = None;
            return 0
        },
        _ => {}
    }
    let mut bp = printhead.blueprint.as_mut().expect("No blueprint in progess!");

    printhead.socket.write(&commandid).unwrap();

    let matreq = match commandid[0] {
        1 => { //Choose level & mat, 4+1=5byte params
             let mut params = [0;5];
             bp.read_exact(&mut params).unwrap();
             printhead.matid = params[4] as i32; //New material will be taken from container with id
             printhead.socket.write(&params).unwrap();
             0
        },
        2 => { //Print dot, 2*4=8 byte params
             let mut params = [0;8];
             bp.read_exact(&mut params).unwrap();
             printhead.socket.write(&   params).unwrap();
             1//A dot takes 1 material unit
        },
        3 => { //Print line, 4*4=16byte params
            let mut params = [0;16];
            bp.read_exact(&mut params).unwrap();
            printhead.socket.write(&params).unwrap();
            2//A line takes 2 material units
        }
        _ => {
            unreachable!("Unknown blueprint command");
        }
    };

    printhead.timeoutid = Some(eventloop.timeout(printhead.id, Duration::from_millis(PRINT_TIMEOUT_MS)).unwrap());
    return matreq;
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
    println!("Job's done!");
}
