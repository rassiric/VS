use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::time::Duration;
use mio::{Handler, Timeout, EventLoop};
use super::Printer;

pub struct Core {
    printers : Arc<Mutex<HashMap<usize, Printer>>>,
    job_queue : Arc<Vec<String>>
}

pub enum TimeoutType {
    PollStatus
}

impl Core {
    pub fn new(printers : Arc<Mutex<HashMap<usize, Printer>>>,
            job_queue : Arc<Vec<String>>) -> Self {
        Core {
            printers: printers,
            job_queue: job_queue
        }
    }
}

impl Handler for Core {
    type Timeout = TimeoutType;
    type Message = usize;

    fn timeout(&mut self, eventloop: &mut EventLoop<Core>, timeout_token: TimeoutType) {
        match timeout_token {
            TimeoutType::PollStatus => {
                super::update_status( self.printers.clone() );
                eventloop.timeout( TimeoutType::PollStatus, Duration::from_millis(super::super::POLL_TIME_MS) ).unwrap();
                {
                    let p = self.printers.lock().unwrap();
                    println!( "Poll result:\n{:#?}", p.deref() );
                }
            }
        }
    }
}