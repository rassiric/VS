extern crate hyper;
extern crate mio;
extern crate rustc_serialize;

mod printer_mgmt;
mod ui;

use printer_mgmt::Printer;
use std::fs::File;
use std::path::Path;
use std::io::{BufReader, BufRead};
use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

static PRINTER_ID_COUNTER : AtomicUsize = ATOMIC_USIZE_INIT;

pub fn get_new_printer_id() -> usize {
    PRINTER_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn load_configured_printers(printers : Arc<Mutex<HashMap<usize, Printer>>>) {
    if ! Path::new("printers.conf").exists() {
        return;
    }

    let mut printers_lock = printers.lock().unwrap();
    let mut printers = printers_lock.deref_mut();

    let conf_file = File::open("printers.conf").unwrap();
    let mut conf = BufReader::new(conf_file);
    let mut conf_line = String::new();
    loop {
        match conf.read_line(&mut conf_line) {
            Ok(0) => return,
            Ok(_) => {
                let (fab, addr) = conf_line.split_at( conf_line.find("\t").expect("Invalid config file: Line without TAB!") );
                let printerid = get_new_printer_id();
                printers.insert( printerid,
                    Printer::new( fab.parse().expect("Invalid config file: Non-numeric fab id!"),
                        printerid, addr.trim().to_string() ) );
            }
            Err(e) => panic!("Error while reading configured printers: {}", e)
        }
    }
}

fn main() {
    let printers : Arc<Mutex<HashMap<usize, Printer>>> = Arc::new( Mutex::new( HashMap::new() ) );
    load_configured_printers( printers.clone() );

    printer_mgmt::update_status( printers.clone() );

    {
        let p = printers.lock().unwrap();
        println!( "{:#?}", p.deref() );
    }
}
