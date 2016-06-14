use hyper::{Get, Post, StatusCode, RequestUri, Decoder, Encoder, Next};
use hyper::header::ContentType;
use hyper::net::HttpStream;
use hyper::server::{Handler, Request, Response};
use hyper::mime;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::io;
use std::io::{Write, Read, Cursor};
use mio;
use mio::Token;
use internals::{Printerpart, PrinterPartType};
use rustc_serialize::json;
use rustc_serialize::base64::FromBase64;

#[derive(RustcEncodable)]
struct Status {
    busy: bool,
    matempty: bool
}

#[derive(RustcDecodable)]
struct PrintReq {
    blueprint: String
}

pub struct PrinterRest {
    pub internals: Arc<RwLock<HashMap<Token, Arc<RwLock<Printerpart>>>>>,
    evloop_send:   Arc<mio::Sender<Token>>,
    action:        Action,
    buf:           Vec<u8>,
    read_pos:      usize
}

enum Action {
    InvalidRequest,
    GetStatus,
    Print
}

impl PrinterRest {
    pub fn new(internals: Arc<RwLock<HashMap<Token, Arc<RwLock<Printerpart>>>>>,
            evloop_send: Arc<mio::Sender<Token>>) -> Self{
        PrinterRest {
            internals: internals,
            evloop_send: evloop_send,
            action:    Action::InvalidRequest,
            buf:       vec![0;0], //Start with empty read buffer, will be increased when used
            read_pos:  0
        }
    }

    fn get_status(&mut self) -> String {
        let status = Status {
            busy: self.get_free_printhead().is_none(), //Printer is busy if no printhead is available (so it also works if there is no Printhead connected yet)
            matempty: !self.check_mat_status()
        };
        json::encode(&status).unwrap()
    }

    fn start_print(&mut self) -> String {
        let reqtext = String::from_utf8(self.buf.clone()).unwrap();
        let req : PrintReq = json::decode(&reqtext).unwrap();
        let bp = req.blueprint.from_base64().unwrap();
        let printhead = self.get_free_printhead();
        if printhead.is_none() {
            return "{ \"success\": false, \"reason\": \"no printhead\" }".to_string();
        }

        let printhead = printhead.unwrap();
        printhead.write().unwrap().blueprint = Some( Box::new( Cursor::new(bp) ) );

        let printheadid = printhead.read().unwrap().id;
        match self.evloop_send.send( Token( printheadid ) ) { //Continue 3d print in internal eventloop
            Ok(_) => "{ \"success\": true, \"reason\": \"\"}".to_string(),
            Err(msg) => format!("{{\"success\": false, \"reason\": \"notify failed: {:?}\" }}", msg)
        }
    }

    fn get_free_printhead(self : &Self) -> Option<Arc<RwLock<Printerpart>>> {
        let clients = self.internals.read().unwrap();
        for cell in clients.values() {
            let part = cell.read().unwrap();
            if part.parttype == PrinterPartType::Printhead
                    && part.blueprint.is_none() {
                return Some(cell.clone());
            }
        }
        None
    }

    fn check_mat_status(&self) -> bool {
        let clients = self.internals.read().unwrap();
        for cell in clients.values() {
            let part = cell.read().unwrap();
            if part.parttype != PrinterPartType::Material {
                continue;
            }
            if part.matempty {
                return false;
            }
        }
        true
    }
}

impl Handler<HttpStream> for PrinterRest {
    fn on_request(&mut self, req: Request) -> Next{
        match *req.uri() {
            RequestUri::AbsolutePath(ref path) =>
            match (req.method(), &path[..]) {
                (&Get, "/") | (&Get, "/status") => {
                    self.action = Action::GetStatus;
                    Next::write()
                },
                (&Post, "/print") => {
                    self.action = Action::Print;
                    Next::read_and_write()
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
        match self.action {
            Action::Print => {
                match transport.read(&mut self.buf[self.read_pos .. ]) {
                    Ok(0) => Next::write(),
                    Ok(n) => {
                        self.read_pos += n;
                        Next::read_and_write()
                    }
                    Err(e) => match e.kind() {
                        io::ErrorKind::WouldBlock => Next::read_and_write(),
                        _ => {
                            println!("read error {:?}", e);
                            Next::end()
                        }
                    }
                }
            },
            _ => unimplemented!()
        }
    }

    fn on_response(&mut self, res: &mut Response) -> Next {
	    res.headers_mut().set( ContentType(
            mime::Mime( mime::TopLevel::Application, mime::SubLevel::Json,
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
        match self.action {
            Action::InvalidRequest => {
                transport.write_all(b"{ \"error\": \"invalidrequest\" }").unwrap();
                Next::end()
            },
            Action::GetStatus => {
                transport.write_all( self.get_status( ).as_bytes() ).unwrap();
                Next::end()
            }
            Action::Print => {
                transport.write_all( self.start_print( ).as_bytes() ).unwrap();
                Next::end()
            }
            //_ => unimplemented!()
        }
    }
}
