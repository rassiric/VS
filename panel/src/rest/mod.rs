use hyper::{Get, Post, StatusCode, RequestUri, Decoder, Encoder, Next};
use hyper::header::ContentLength;
use hyper::net::HttpStream;
use hyper::server::{Server, Handler, Request, Response};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use mio::Token;
use super::internals::Printerpart;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{Write, Read};

struct PrinterRest { 
    pub internals: Arc<RwLock<HashMap<Token, Rc<RefCell<Printerpart>>>>>,
    action:        Action
}

enum Action {
    InvalidRequest,
    GetStatus
}

impl PrinterRest {
    fn new(internals: Arc<RwLock<HashMap<Token, Rc<RefCell<Printerpart>>>>>) -> Self{
        PrinterRest {
            internals: internals,
            action:    Action::InvalidRequest
        }
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
        match self.action {
            Action::InvalidRequest => {
                res.set_status(StatusCode::BadRequest); //Generic 400 failure
                res.headers_mut().set( ContentLength(29) );
                Next::write()
            },
            Action::GetStatus => {
                res.headers_mut().set( ContentLength(0) );
                Next::end()
            }
        }
    }

    fn on_response_writable(&mut self, transport: &mut Encoder<HttpStream>) -> Next {
        match self.action {
            Action::InvalidRequest => {
                transport.write(b"{ \"error\": \"invalidrequest\" }").unwrap();
                Next::end()
            }
            _ => unimplemented!()
        }
    }
}

pub fn serve(internals : Arc<RwLock<HashMap<Token, Rc<RefCell<Printerpart>>>>>) {
    let server = Server::http(&"0.0.0.0:8080".parse().unwrap()).unwrap();
    let (_, server) = server.handle(|_| PrinterRest::new( internals.clone() ) ).unwrap();

    server.run();
}

