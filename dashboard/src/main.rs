extern crate hyper;
extern crate mio;
extern crate rustc_serialize;
extern crate regex;

mod printer_mgmt;
mod ui;

use printer_mgmt::core;
use printer_mgmt::{Printer, Core};
use std::fs::File;
use std::path::Path;
use std::io::{BufReader, BufRead};
use std::time::Duration;
use std::thread;
use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};
use mio::{EventLoop};

static PRINTER_ID_COUNTER : AtomicUsize = ATOMIC_USIZE_INIT;

pub const POLL_TIME_MS : u64 = 2500;

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
    let jobqueue : Arc<Vec<String>> = Arc::new( Vec::new() ); //Access is protected by printers Mutex per convention
    let printers : Arc<Mutex<HashMap<usize, Printer>>> = Arc::new( Mutex::new( HashMap::new() ) );
    load_configured_printers( printers.clone() );

    let ui_printers = printers.clone();
    let _uithread = thread::spawn( move || ui::serve( ui_printers ) );

    printer_mgmt::update_status( printers.clone() );

    {
        let p = printers.lock().unwrap();
        println!( "Printers loaded from config, first query done:\n{:#?}", p.deref() );
    }


    let mut eventloop = EventLoop::new().unwrap();

    let mut c = Core::new( printers.clone(), jobqueue.clone() );

    eventloop.timeout( core::TimeoutType::PollStatus, Duration::from_millis(POLL_TIME_MS) ).unwrap();

    eventloop.run(&mut c);
}
