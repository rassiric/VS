use hyper::{Get, Post, StatusCode, RequestUri, Decoder, Encoder, Next};
use hyper::header::ContentType;
use hyper::net::HttpStream;
use hyper::server::{Server, Handler, Request, Response};
use hyper::mime;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::io;
use std::io::{Write, Read};
use std::fs::File;
use std::ops::{Deref, DerefMut};
use printer_mgmt::{Printer, printbp};
use regex::Regex;
use super::super::get_new_printer_id;
use url::form_urlencoded;
use super::super::BenchWatchStopTime;
use super::super::time;

struct Templates {
    page_begin :  String,
    page_end :    String,
    error :       String,
    status :      String,
    status_fab_begin : String,
    status_fab_end : String,
    status_printer : String,
    print :       String,
    mgmt_begin :  String,
    mgmt_printer: String,
    mgmt_end :    String,

    reg_fabs:  Regex,
    reg_printers: Regex,
    reg_available: Regex,
    reg_matempty: Regex,
    reg_queue:  Regex,
    reg_fab:  Regex,
    reg_printer: Regex,
    reg_status: Regex
}

pub struct WebUi {
    printers:      Arc<Mutex<HashMap<usize, Printer>>>,
    job_queue :    Arc<Mutex<Vec<(usize, String, String)>>>,
    action:        Action,
    buf:           Vec<u8>,
    read_pos:      usize,
    templates:     Arc<Templates>
}

enum Action {
    InvalidRequest,
    GetStatus,
    GetPrint,
    GetMgmt,
    Print,
    AddPrinter,
    DelPrinter,
    Benchmark
}

impl WebUi {
    fn new(printers : Arc<Mutex<HashMap<usize, Printer>>>,
        job_queue : Arc<Mutex<Vec<(usize, String, String)>>>,
        templates: Arc<Templates>) -> Self{
        WebUi {
            printers:  printers,
            job_queue: job_queue,
            action:    Action::InvalidRequest,
            buf:       vec![0;0], //Start with empty read buffer, will be increased when used
            read_pos:  0,
            templates: templates
        }
    }

    fn get_status(&mut self, outp:&mut Write) {
        let printers_lock = self.printers.lock().unwrap();
        let printers = printers_lock.deref();

        let mut fabs : Vec<usize> = Vec::new();
        let mut count_avail = 0;
        let mut count_matempty = 0;
        for printer in printers.values() {
            fabs.push(printer.fabid);
            if !printer.status.busy && !printer.status.matempty {
                count_avail += 1;
            }
            else if printer.status.matempty {
                count_matempty += 1;
            }
        }
        fabs.sort();
        fabs.dedup();

        let result = self.templates.reg_fabs.replace_all(&*self.templates.status, &*fabs.len().to_string());
        let result = self.templates.reg_printers.replace_all(&*result, &*printers.len().to_string());
        let result = self.templates.reg_available.replace_all(&*result, &*count_avail.to_string());
        let result = self.templates.reg_matempty.replace_all(&*result, &*count_matempty.to_string());
        let result = self.templates.reg_queue.replace_all(&*result, "0");
        let _ = outp.write_all( result.as_bytes() );

        for fab in fabs.iter() {
            let _ = outp.write_all( self.templates.reg_fab.replace_all(&*self.templates.status_fab_begin, &*fab.to_string()).as_bytes() );
            for printer in printers.values() {
                if printer.fabid != *fab { continue; }
                let _ = outp.write_all( self.templates.reg_printer.replace_all(
                                &*self.templates.reg_status.replace_all(
                                    &*self.templates.status_printer,
                                    &*format!("{:#?}", printer.status)
                                ), &*printer.id.to_string() ).as_bytes() );
            }
            let _ = outp.write_all( self.templates.status_fab_end.as_bytes() );
        }
    }

    fn get_print(&mut self, outp:&mut Write) {
        let _ = outp.write_all( self.templates.print.as_bytes() );
    }

    fn get_mgmt(&mut self, outp:&mut Write) {
        let printers_lock = self.printers.lock().unwrap();
        let printers = printers_lock.deref();

        let _ = outp.write_all( self.templates.mgmt_begin.as_bytes() );

        for printer in printers.values() {
            let _ = outp.write_all( &* self.templates.reg_status.replace_all(
                &* self.templates.reg_printer.replace_all(
                    &*self.templates.mgmt_printer,
                    &*printer.id.to_string()
                ), &*format!("{:#?}", printer) ).as_bytes() );
        }

        let _ = outp.write_all( self.templates.mgmt_end.as_bytes() );
    }

    fn add_printer(&mut self, outp:&mut Write){
        let mut reqtext = form_urlencoded::parse(&self.buf[0 .. self.read_pos]);

        let fab :usize = match reqtext.find(|&(ref key,_)| key=="fab") {
            Some((_, fabstr)) => match fabstr.parse() {
                    Ok(fabint) => fabint,
                    Err(_) => {
                        let _ = outp.write_all(
                            b"<div class=\"alert alert-danger\">Adding printer failed: fab not numeric!</div>" );
                        return;
                    }
                },
            None => {
                let _ = outp.write_all(
                    b"<div class=\"alert alert-danger\">Adding printer failed: no fab specified!</div>" );
                return;
            }
        };

        let ip = match reqtext.find(|&(ref key,_)| key=="ip") {
            Some((_, ip)) => ip.into_owned(),
            None => {
                let _ = outp.write_all(
                    b"<div class=\"alert alert-danger\">Adding printer failed: no ip specified!</div>" );
                return;
            }
        };

        let mut printers_lock = self.printers.lock().unwrap();
        let mut printers = printers_lock.deref_mut();
        let printerid = get_new_printer_id();

        printers.insert( printerid, Printer::new( fab, printerid, ip ) );

        let _ = outp.write_all( format!("<div class=\"alert alert-success\">Printer added - ID:{}</div>", printerid).as_bytes() );
    }

    fn del_printer(&mut self, outp:&mut Write){

        let mut reqtext = form_urlencoded::parse(&self.buf[0 .. self.read_pos]);

        let printer_id :usize = match reqtext.find(|&(ref key,_)| key=="id") {
            Some((_, fabstr)) => match fabstr.parse() {
                    Ok(fabint) => fabint,
                    Err(_) => {
                        let _ = outp.write_all(
                            b"<div class=\"alert alert-danger\">Delete failed: id not numeric!</div>" );
                        return;
                    }
                },
            None => {
                let _ = outp.write_all(
                    b"<div class=\"alert alert-danger\">Delete failed: no id!</div>" );
                return;
            }
        };

        let mut printers_lock = self.printers.lock().unwrap();
        let mut printers = printers_lock.deref_mut();

        match printers.remove(&printer_id){
            Some(_) => {
                let _ = outp.write_all(
                    b"<div class=\"alert alert-success\">Printer deleted!</div>" );
                },
            None => {
                let _ = outp.write_all(
                    b"<div class=\"alert alert-warn\">Delete failed: printer not found!</div>" );
                return;
            }
        }
    }

    fn print(&mut self, outp:&mut Write){
        let mut params = form_urlencoded::parse(&self.buf[0 .. self.read_pos]);

        let fab :usize = match params.find(|&(ref key,_)| key=="fab") {
            Some((_, fabstr)) => match fabstr.parse() {
                    Ok(fabint) => fabint,
                    Err(_) => {
                        let _ = outp.write_all(
                            b"<div class=\"alert alert-danger\">Printing failed: fab not numeric!</div>" );
                        return;
                    }
                },
            None => {
                let _ = outp.write_all(
                    b"<div class=\"alert alert-danger\">Printing failed: no fab specified!</div>" );
                return;
            }
        };

        let model = match params.find(|&(ref key,_)| key=="bp") {
            Some((_, model)) => model.into_owned(),
            None => {
                let _ = outp.write_all(
                    b"<div class=\"alert alert-danger\">Printing failed: no model specified!</div>" );
                return;
            }
        };

        let title = match params.find(|&(ref key,_)| key=="jt") {
            Some((_, title)) => title.into_owned(),
            None => "".to_string()
        };

        match printbp(self.printers.clone(), self.job_queue.clone(),
            fab, model, &title) {
                Ok(_) => {
                    let _ = outp.write_all(b"<div class=\"alert alert-success\">Printing job</div>");
                }
                Err(err) => {
                    let _ = outp.write_all( format!("<div class=\"alert alert-danger\">Printing failed: {}</div>", err).as_bytes() );
                }
            }
    }

    fn benchmark(&mut self, outp:&mut Write){

        unsafe{BenchWatchStopTime = time::precise_time_ns();}

        for _ in 0..50{
            match printbp(self.printers.clone(), self.job_queue.clone(),
                0, "bm".to_string(), &"benchmark".to_string()) {
                    Ok(_) => {
                        let _ = outp.write_all(b"<div class=\"alert alert-success\">Printing job</div>");
                    }
                    Err(err) => {
                        let _ = outp.write_all( format!("<div class=\"alert alert-danger\">Printing failed: {}</div>", err).as_bytes() );
                    }
                }
        }
    }
}

impl Handler<HttpStream> for WebUi {
    fn on_request(&mut self, req: Request) -> Next{
        match *req.uri() {
            RequestUri::AbsolutePath(ref path) =>
            match (req.method(), &path[..]) {
                (&Get, "/") | (&Get, "/status") => {
                    self.action = Action::GetStatus;
                    Next::write()
                },
                (&Get, "/mgmt") => {
                    self.action = Action::GetMgmt;
                    Next::write()
                },
                (&Get, "/bm") => {
                    self.action = Action::Benchmark;
                    Next::write()
                },
                (&Post, "/print") => {
                    self.action = Action::Print;
                    Next::read()
                },
                (&Get, "/print") => {
                    self.action = Action::GetPrint;
                    Next::write()
                },
                (&Post, "/mgmt/add") => {
                    self.action = Action::AddPrinter;
                    Next::read()
                },
                (&Post, "/mgmt/delete") => {
                    self.action = Action::DelPrinter;
                    Next::read()
                },
                _ => Next::write(), //InvalidRequest
            },
            _ => Next::write(), //InvalidRequest
        }
    }

    fn on_request_readable(&mut self, transport: &mut Decoder<HttpStream>) -> Next {
        if self.read_pos >= self.buf.len() {
            let newsize = self.buf.len() + 2048;
            self.buf.resize(newsize, 0); //If buffer is full, resize by 2KB
        }
        match transport.read(&mut self.buf[self.read_pos .. ]) {
            Ok(0) => Next::write(),//Finished reading, continue with response generation
            Ok(n) => {//Read sth
                self.read_pos += n;
                Next::read_and_write() //re-invoke this method to continue reading, e.g. if buffer was too small
            }
            Err(e) => match e.kind() {
                io::ErrorKind::WouldBlock => Next::read_and_write(), //wait for more data
                _ => {
                    println!("read error {:?}", e);
                    Next::end()
                }
            }
        }
    }

    fn on_response(&mut self, res: &mut Response) -> Next {
	    res.headers_mut().set( ContentType(
            mime::Mime( mime::TopLevel::Text, mime::SubLevel::Html,
                vec![(mime::Attr::Charset, mime::Value::Utf8)] ) ) ); //We ALWAYS respond in HTML
        match self.action {
            Action::InvalidRequest => {
                res.set_status(StatusCode::BadRequest); //Generic 400 failure
                Next::write()
            },
            _ => {
                Next::write()
            }
        }
    }

    fn on_response_writable(&mut self, transport: &mut Encoder<HttpStream>) -> Next {
        let _ = transport.write_all( self.templates.page_begin.as_bytes() );
        match self.action {
            Action::InvalidRequest => {
                let _ = transport.write_all( self.templates.error.as_bytes() );
            },
            Action::GetStatus => {
                self.get_status( transport );
            },
            Action::GetPrint => {
                self.get_print( transport );
            },
            Action::GetMgmt => {
                self.get_mgmt( transport );
            },
            Action::Benchmark => {
                self.benchmark( transport );
            },
            Action::Print => {;
                self.print( transport );
            },
            Action::AddPrinter => {
                self.add_printer( transport );
                self.get_mgmt( transport );
            },
            Action::DelPrinter => {
                self.del_printer( transport );
                self.get_mgmt( transport );
            }
            //_ => unimplemented!()
        };
        let _ = transport.write_all( self.templates.page_end.as_bytes() );
        Next::end()
    }
}

pub fn serve(printers: Arc<Mutex<HashMap<usize, Printer>>>,
    job_queue : Arc<Mutex<Vec<(usize, String, String)>>>) {
    let mut temps = Templates {
        page_begin: String::new(),
        page_end:   String::new(),
        error:      String::new(),
        status :    String::new(),
        status_fab_begin : String::new(),
        status_fab_end : String::new(),
        status_printer : String::new(),
        print :     String::new(),
        mgmt_begin : String::new(),
        mgmt_printer : String::new(),
        mgmt_end :  String::new(),

        reg_fabs :      Regex::new(r"\{fabs\}").unwrap(),
        reg_printers :  Regex::new(r"\{printers\}").unwrap(),
        reg_available : Regex::new(r"\{available\}").unwrap(),
        reg_matempty :  Regex::new(r"\{matempty\}").unwrap(),
        reg_queue :     Regex::new(r"\{queue\}").unwrap(),
        reg_fab :       Regex::new(r"\{fab\}").unwrap(),
        reg_printer :   Regex::new(r"\{printer\}").unwrap(),
        reg_status :    Regex::new(r"\{status\}").unwrap()
    };
    File::open("uitemplates/page_begin.html").expect("Cannot open template page_begin.html!")
        .read_to_string( &mut temps.page_begin ).unwrap();
    File::open("uitemplates/page_end.html").expect("Cannot open template page_end.html!")
        .read_to_string( &mut temps.page_end ).unwrap();
    File::open("uitemplates/error.html").expect("Cannot open template error.html!")
        .read_to_string( &mut temps.error ).unwrap();
    File::open("uitemplates/status.html").expect("Cannot open template status.html!")
        .read_to_string( &mut temps.status ).unwrap();
    File::open("uitemplates/status_fab_begin.html").expect("Cannot open template status_fab_begin.html!")
        .read_to_string( &mut temps.status_fab_begin ).unwrap();
    File::open("uitemplates/status_fab_end.html").expect("Cannot open template status_fab_end.html!")
        .read_to_string( &mut temps.status_fab_end ).unwrap();
    File::open("uitemplates/status_printer.html").expect("Cannot open template status_printer.html!")
        .read_to_string( &mut temps.status_printer ).unwrap();
    File::open("uitemplates/print.html").expect("Cannot open template print.html!")
        .read_to_string( &mut temps.print ).unwrap();
    File::open("uitemplates/mgmt_begin.html").expect("Cannot open template mgmt_begin.html!")
        .read_to_string( &mut temps.mgmt_begin ).unwrap();
    File::open("uitemplates/mgmt_end.html").expect("Cannot open template mgmt_end.html!")
        .read_to_string( &mut temps.mgmt_end ).unwrap();
    File::open("uitemplates/mgmt_printer.html").expect("Cannot open template mgmt_printer.html!")
        .read_to_string( &mut temps.mgmt_printer ).unwrap();


    let temps = Arc::new(temps);

    let server = Server::http(&"0.0.0.0:8080".parse().unwrap()).unwrap();
    let (_, serverloop) = server.handle(|_| WebUi::new( printers.clone(),
        job_queue.clone(), temps.clone() ) ).unwrap();

    serverloop.run();
}
