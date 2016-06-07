use super::super::mio;
use super::super::time;

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::thread;
use std::cell::RefCell;
use std::io::stdin;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::time::Duration;
use mio::tcp::TcpListener;
use mio::{Token, Timeout, EventLoop, EventSet, PollOpt, Handler};

use super::Printerpart;
use super::PrinterPartType;
use super::super::SERVER_TOKEN;
use super::super::CLI_TOKEN;
use super::super::PRINT_TIMEOUT_MS;

use super::BenchWatchStopTime;

pub struct Server {
    pub socket: TcpListener,
    pub clients: Arc<Mutex<HashMap<Token, Rc<RefCell<Printerpart>>>>>,
    pub tokencounter: usize,
    pub continuedelay: Option<Timeout>
}

impl Server {
    fn check_mat_status(&self) -> bool {
        let client = self.clients.lock().unwrap();
        for cell in client.values() {
            let part = cell.borrow();
            if part.parttype != PrinterPartType::Material {
                continue;
            }
            if part.matempty {
                return false;
            }
        }
        true
    }

    fn accept_new_client(&mut self, eventloop : &mut EventLoop<Server>) {
        let clientsocket = match self.socket.accept() {
            Err(e) => {
                println!("Accept error: {}", e);
                return;
            },
            Ok(None) => panic!("Accept has returned 'None'"),
            Ok(Some((sock,_))) => sock
       };

       self.tokencounter += 1;
       let token = Token(self.tokencounter);

       let mut clients = self.clients.lock().unwrap();
       clients.insert( token, Rc::new( RefCell::new( Printerpart::new(clientsocket, self.tokencounter) ) ) );
       eventloop.register( & clients[&token].borrow().socket, token,
                           EventSet::readable(), PollOpt::edge() ).unwrap();
    }

    fn get_free_printhead(self : &Self) -> Option<Rc<RefCell<Printerpart>>> {
        let clients = self.clients.lock().unwrap();
        for cell in clients.values() {
            let part = cell.borrow();
            if part.parttype == PrinterPartType::Printhead
                    && part.blueprint.is_none() {
                return Some(cell.clone());
            }
        }
        None
    }

    fn start_print(self : &mut Self, eventloop : &mut EventLoop<Server>) {
        if !self.check_mat_status() {
            println!("Job discarded: Please refill material containers first!");
            return;
        }
         
        match self.get_free_printhead(){
            None => {
                println!("Printhead[s] busy");
            },
            Some(printhead) => {
                let mut printhead = printhead.borrow_mut();
                println!("Sending job to printhead({})", printhead.id);
                printhead.load_blueprint();

                match self.get_mat_src(printhead.matid) {
                    Some(mat_src) => {
                        printhead.exec_instr( eventloop, Some(mat_src.borrow_mut().deref_mut()) );
                    },
                    None => {
                        printhead.exec_instr( eventloop, None );
                    }
                }
            }
        }
    }

    fn benchmark(self : &mut Self, eventloop : &mut EventLoop<Server>) {
        match self.get_free_printhead(){
            None => {
                println!("Printhead[s] busy");
                return;
            },
            Some(mut printhead) => {
                let mut printhead = printhead.borrow_mut();
                println!("Benchmarking printhead({})", printhead.id);
                printhead.benchmarkcnt = 10000;
                unsafe{BenchWatchStopTime = time::precise_time_ns();}
                printhead.socket.write(&[1,57,5,0,0,0]).unwrap();
                printhead.timeoutid = Some(eventloop.timeout(printhead.id, Duration::from_millis(PRINT_TIMEOUT_MS)).unwrap());
            }
        }
    }

    fn get_mat_src(self : &Self, required_mat_id : i32) -> Option<Rc<RefCell<Printerpart>>> {
        if self.check_mat_status() {
            let clients = self.clients.lock().unwrap();
            for cell in clients.values() {
                let part = cell.borrow();
                if part.parttype == PrinterPartType::Material
                        && part.matid == required_mat_id {
                    return Some(cell.clone());
                }
            }
            None
        } else {
            None //If production is paused until refill, don't supply MatContainer references
        }
    }
}

impl Handler for Server {
    type Timeout = usize;
    type Message = ();

    fn ready(&mut self, eventloop: &mut EventLoop<Server>, token: Token, _: EventSet)
    {
        match token {
            SERVER_TOKEN => {
                self.accept_new_client(eventloop);
            },
            CLI_TOKEN => {
                let mut input = String::new();
                stdin().read_line(&mut input).unwrap();
                match input.trim() {
                    "p" => {
                        self.start_print(eventloop);                      
                    },
                    "b" => {
                        self.benchmark(eventloop);
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

                let mut clients = self.clients.lock().unwrap();
                let mut client = clients.get(&token).unwrap().borrow_mut();
                match self.get_mat_src(client.matid) {
                    Some(mat_src) => {
                        client.notify( eventloop, Some(mat_src.borrow_mut().deref_mut()), &mut self.continuedelay );
                    },
                    None => {
                        client.notify( eventloop, None, &mut self.continuedelay );
                    }
                }
            }
        }
    }
    fn timeout(&mut self, eventloop: &mut EventLoop<Server>, timeout_token: usize) {
        match timeout_token {
            0 => { //Timeout id 0 is check for continue
                if self.check_mat_status() {
                    println!("All material containers refilled!");
                    for mut printerpart in self.clients.lock().unwrap().values().map( |cell| cell.borrow_mut() ) {
                        if printerpart.parttype == PrinterPartType::Printhead &&
                                printerpart.blueprint.is_some() {
                            println!("Continuing on printhead {}", printerpart.id );
                            
                            match self.get_mat_src(printerpart.matid) {
                                Some(mat_src) => {
                                    printerpart.exec_instr( eventloop, Some(mat_src.borrow_mut().deref_mut()) );
                                },
                                None => {
                                    printerpart.exec_instr( eventloop, None );
                                }
                            }
                        }
                    }
                }
                else {
                    println!("Still missing material...");
                }
            }
            _ => {
                println!("Timeout while printing, aborting...");
                let mut clients = self.clients.lock().unwrap();
                let mut connection = clients.get(&Token(timeout_token)).unwrap().borrow_mut();
                connection.set_blueprint(None); //Abort print process
                connection.abort_benchmark(); //Abort benchmark process
            }
        };
    }
}

