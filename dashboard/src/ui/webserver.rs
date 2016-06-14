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
use std::ops::{Deref, Add};
use printer_mgmt::Printer;
use regex::Regex;

struct Templates {
    page_begin :  String,
    page_end :    String,
    error :       String,
    status :      String,
    status_fab_begin : String,
    status_fab_end : String,
    status_printer : String,

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
    action:        Action,
    buf:           Vec<u8>,
    read_pos:      usize,
    templates:     Arc<Templates>
}

enum Action {
    InvalidRequest,
    GetStatus
}

impl WebUi {
    fn new(printers : Arc<Mutex<HashMap<usize, Printer>>>, templates: Arc<Templates>) -> Self{
        WebUi {
            printers:  printers,
            action:    Action::InvalidRequest,
            buf:       vec![0;0], //Start with empty read buffer, will be increased when used
            read_pos:  0,
            templates: templates
        }
    }

    fn get_status(&mut self, outp:&mut Write) {
        let mut printers_lock = self.printers.lock().unwrap();
        let mut printers = printers_lock.deref();

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
            outp.write_all( self.templates.reg_fab.replace_all(&*self.templates.status_fab_begin, &*fab.to_string()).as_bytes() );
            println!("{}", fab);
            for printer in printers.values() {
                println!("#{}", printer.id);
                if printer.fabid != *fab { continue; }
                outp.write_all( self.templates.reg_printer.replace_all(
                                &*self.templates.reg_status.replace_all(
                                    &*self.templates.status_printer,
                                    &*format!("{:#?}", printer.status)
                                ), &*printer.id.to_string() ).as_bytes() );
            }
            println!("/{}", fab);
            outp.write_all( self.templates.status_fab_end.as_bytes() );
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
                _ => Next::write(), //InvalidRequest
            },
            _ => Next::write(), //InvalidRequest
        }
    }

    fn on_request_readable(&mut self, transport: &mut Decoder<HttpStream>) -> Next {
        unimplemented!()
    }

    fn on_response(&mut self, res: &mut Response) -> Next {
	    res.headers_mut().set( ContentType(
            mime::Mime( mime::TopLevel::Text, mime::SubLevel::Html,
                vec![(mime::Attr::Charset, mime::Value::Utf8)] ) ) );
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
            }
            //_ => unimplemented!()
        };
        let _ = transport.write_all( self.templates.page_end.as_bytes() );
        Next::end()
    }
}

pub fn serve(printers: Arc<Mutex<HashMap<usize, Printer>>>) {
    let mut temps = Templates {
        page_begin: String::new(),
        page_end:   String::new(),
        error:      String::new(),
        status :    String::new(),
        status_fab_begin : String::new(),
        status_fab_end : String::new(),
        status_printer : String::new(),

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
        .read_to_string( &mut temps.page_begin );
    File::open("uitemplates/page_end.html").expect("Cannot open template page_end.html!")
        .read_to_string( &mut temps.page_end );
    File::open("uitemplates/error.html").expect("Cannot open template error.html!")
        .read_to_string( &mut temps.error );
    File::open("uitemplates/status.html").expect("Cannot open template status.html!")
        .read_to_string( &mut temps.status );
    File::open("uitemplates/status_fab_begin.html").expect("Cannot open template status_fab_begin.html!")
        .read_to_string( &mut temps.status_fab_begin );
    File::open("uitemplates/status_fab_end.html").expect("Cannot open template status_fab_end.html!")
        .read_to_string( &mut temps.status_fab_end );
    File::open("uitemplates/status_printer.html").expect("Cannot open template status_printer.html!")
        .read_to_string( &mut temps.status_printer );

    
    let temps = Arc::new(temps);

    let server = Server::http(&"0.0.0.0:8080".parse().unwrap()).unwrap();
    let (_, serverloop) = server.handle(|_| WebUi::new( printers.clone(), temps.clone() ) ).unwrap();

    serverloop.run();
}