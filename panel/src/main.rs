extern crate mio;
extern crate time;
extern crate hyper;
extern crate rustc_serialize;

mod internals;
mod rest;

use std::sync::{Arc, RwLock};
use mio::{EventLoop, Token, EventSet, PollOpt};
use std::net::SocketAddr;
use mio::tcp::TcpListener;
use std::collections::HashMap;
use std::thread;

const SERVER_TOKEN: Token = Token(0);
const CLI_TOKEN: Token = Token(1);
const PRINT_TIMEOUT_MS : u64 = 10000;
const CONTINUE_DELAY_MS : u64 = 1000;

fn main() {
    println!("VS-Fab 3D Printer Panel - Ramiz Bahrami(736861), Adrian Müller(734922)");
    println!("Welcome! Your options are:");
    println!(" p - Print blueprint once");
    println!(" b - Run throughput benchmark");
    println!(" q - Quit");

    let mut eventloop = EventLoop::new().unwrap();

    let internal_parts = Arc::new(RwLock::new(HashMap::new()));

    let rparts = internal_parts.clone();
    let eventloop_channel = eventloop.channel();
    let _restthread = thread::spawn( move || rest::serve( rparts, eventloop_channel ) );
    
    let address = "0.0.0.0:18000".parse::<SocketAddr>().unwrap();
    let mut server = internals::Server {
            socket: TcpListener::bind(&address).unwrap(),
            tokencounter : 2,
            clients: internal_parts.clone(),
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
