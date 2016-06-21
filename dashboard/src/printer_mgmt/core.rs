use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::ops::Deref;
use std::time::Duration;
use mio::{Handler, EventLoop};
use printer_mgmt::printbp;
use super::Printer;
use super::super::BenchWatchStopTime;
use super::super::time;

pub struct Core {
    printers : Arc<Mutex<HashMap<usize, Printer>>>,
    job_queue : Arc<Mutex<Vec<(usize, String, String)>>>
}

pub enum TimeoutType {
    PollStatus
}

impl Core {
    pub fn new(printers : Arc<Mutex<HashMap<usize, Printer>>>,
            job_queue : Arc<Mutex<Vec<(usize, String, String)>>>) -> Self {
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
                let queue_copy;
                {
                    let mut jobs = self.job_queue.lock().unwrap();
                    unsafe {
                        if jobs.is_empty() && BenchWatchStopTime > 0 {

                            println!("Benchmark finished, time: {}ms",
                                ( (time::precise_time_ns() - BenchWatchStopTime) as f32) / 1_000_000.0);
                                BenchWatchStopTime = 0;
                        }
                    }
                    queue_copy = jobs.clone();
                    jobs.clear();
                }
                for (fab, job, title) in queue_copy {
                    printbp(self.printers.clone(), self.job_queue.clone(), fab, job, &title);
                }
            }
        }
    }
}
