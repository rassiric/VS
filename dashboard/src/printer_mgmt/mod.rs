mod printer;
mod status_req;
mod print_order;
pub mod core;

pub use self::core::Core;
pub use self::printer::Printer;
pub use self::status_req::update_status;

use self::printer::Status;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::ops::DerefMut;
use std::path::Path;

pub fn printbp(printers : Arc<Mutex<HashMap<usize, Printer>>>,
    job_queue : Arc<Mutex<Vec<(usize, String, String)>>>,
    fab : usize, bpname : String, job_title: &String) -> Result<(), String> {
    let filename = format!("blueprints/{}.3dbp", bpname);

    if ! Path::new(&filename).exists() {
        return Err("blueprint not found".to_string());
    }

    let mut printers_lock = printers.lock().unwrap();
    let mut printers = printers_lock.deref_mut();

    for (_id, printer) in printers.iter_mut() {
        if printer.fabid != fab || printer.status.busy || printer.status.matempty {
            continue;
        }
        let mut bpfile = File::open(filename).unwrap();
        printer.status = Status { busy: true, matempty: false, current_job: job_title.clone() };

        return print_order::printbp(&printer.address, &mut bpfile, job_title);
    }
    job_queue.lock().unwrap().deref_mut().push(( fab,bpname.to_string(),job_title.clone() ));
    Ok(())
}
