use hyper::server::{Server, Handler};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use mio::Token;
use super::internals;
use std::cell::RefCell;
use internals::Printerpart;

mod printer_rest;

use self::printer_rest::PrinterRest;

pub fn serve(internals : Arc<RwLock<HashMap<Token, Arc<RwLock<Printerpart>>>>>) {
    let server = Server::http(&"0.0.0.0:8080".parse().unwrap()).unwrap();
    let (_, server) = server.handle(|_| PrinterRest::new( internals.clone() ) ).unwrap();

    server.run();
}

