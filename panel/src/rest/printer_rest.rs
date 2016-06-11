use hyper::{Get, Post, StatusCode, RequestUri, Decoder, Encoder, Next};
use hyper::header::ContentType;
use hyper::net::HttpStream;
use hyper::server::{Handler, Request, Response};
use hyper::mime;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use mio::Token;
use internals::Printerpart;
use internals::PrinterPartType;
use std::io::{Write, Read};
use rustc_serialize::json;

#[derive(RustcEncodable)]
struct Status {
    busy: bool,
    matempty: bool
}

pub struct PrinterRest { 
    pub internals: Arc<RwLock<HashMap<Token, Arc<RwLock<Printerpart>>>>>,
    action:        Action

}

enum Action {
    InvalidRequest,
    GetStatus
}

impl PrinterRest {
    pub fn new(internals: Arc<RwLock<HashMap<Token, Arc<RwLock<Printerpart>>>>>) -> Self{
        PrinterRest {
            internals: internals,
            action:    Action::InvalidRequest
        }
    }

    fn get_status(&mut self) -> String {
        let status = Status {
            busy: self.get_free_printhead().is_none(), //Printer is busy if no printhead is available (so it also works if there is no Printhead connected yet)
            matempty: !self.check_mat_status()
        };
        json::encode(&status).unwrap()
    }


    fn get_free_printhead(self : &Self) -> Option<Arc<RwLock<Printerpart>>> {
        let clients = self.internalss.read().unwrap();
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
            RequestUri::AbsolutePath(ref path) => match (req.method(), &path[..]) {
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
        unimplemented!();
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
            Action::GetStatus => {
                Next::write()
            } 
        }
    }

    fn on_response_writable(&mut self, transport: &mut Encoder<HttpStream>) -> Next {
        match self.action {
            Action::InvalidRequest => {
                transport.write(b"{ \"error\": \"invalidrequest\" }").unwrap();
                Next::end()
            },
            Action::GetStatus => {
                transport.write( self.get_status( ).as_bytes() ).unwrap();
                Next::end()
            }
            //_ => unimplemented!()
        }
    }
}
