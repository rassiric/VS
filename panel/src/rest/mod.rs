use hyper::server::Server;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use mio;
use mio::Token;
use internals::Printerpart;

mod printer_rest;

use self::printer_rest::PrinterRest;

pub fn serve(internals : Arc<RwLock<HashMap<Token, Arc<RwLock<Printerpart>>>>>,
        evloop_send : mio::Sender<Token>) {
    let server = Server::http(&"0.0.0.0:8080".parse().unwrap()).unwrap();
    let evloop_send = Arc::new( evloop_send );
    let (_, serverloop) = server.handle(|_| PrinterRest::new( internals.clone(), evloop_send.clone() ) ).unwrap();

    serverloop.run();
}

