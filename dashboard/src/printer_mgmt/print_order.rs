use hyper;
use hyper::{Decoder, Encoder, Next};
use hyper::client::{Client, Request, Response, DefaultTransport as HttpStream};
use hyper::header::Connection;
use std::io;
use std::io::{Read, Write};
use std::ops::DerefMut;
use std::sync::{Mutex, Arc};
use std::sync::mpsc;
use std::time::Duration;
use std::collections::HashMap;
use hyper::Url;
use rustc_serialize::json;
//use printer_mgmt::printer::{Status, Printer};
use std::str::from_utf8;
use rustc_serialize::base64::{ToBase64, STANDARD};

#[derive(RustcEncodable)]
struct PrintReq {
    blueprint: String
}

#[derive(RustcDecodable)]
pub struct ReqRes {
    success: bool,
    reason: String
}

pub struct PrintOrder {
    result_pipe: mpsc::Sender<ReqRes>,
    req: PrintReq,
    buf : Vec<u8>,
    read_pos : usize
}

impl PrintOrder {
    pub fn new(result_pipe : mpsc::Sender<ReqRes>, bp : &mut Read ) -> Self {
        let mut bpdata = vec![0;0];
        bp.read_to_end(&mut bpdata);
        PrintOrder {
            result_pipe : result_pipe,
            req : PrintReq { blueprint : bpdata.to_base64(STANDARD) },
            buf : vec![0;64],
            read_pos : 0
        }
    }
}

fn read() -> Next {//Helper to generate a read-request with timeout
    Next::read().timeout(Duration::from_millis(300))
}

impl hyper::client::Handler<HttpStream> for PrintOrder {
    fn on_request(&mut self, req: &mut Request) -> Next {
        req.headers_mut().set(Connection::close());
        req.set_method(hyper::method::Method::Post);
        Next::read_and_write()
    }

    fn on_request_writable(&mut self, transport: &mut Encoder<HttpStream>) -> Next {
        let request_json = json::encode(&self.req).unwrap();
        transport.write_all( request_json.as_bytes() );
        read()
    }

    fn on_response(&mut self, _res: Response) -> Next {
        read()
    }

    fn on_response_readable(&mut self, transport: &mut Decoder<HttpStream>) -> Next {
        if self.read_pos >= self.buf.len() {
            let newsize = self.buf.len() + 256;//If buffer is full, resize by 256 byte
            self.buf.resize(newsize, 0); //(status msgs should never need this, but errors might)
        }
        match transport.read(&mut self.buf[self.read_pos .. ]) {
            Ok(0) => {
                let res_text = from_utf8(&self.buf[0 .. self.read_pos]).unwrap();
                let res : ReqRes = json::decode(res_text).unwrap();
                self.result_pipe.send(res).unwrap();
                Next::end()
            }
            Ok(n) => {
                self.read_pos += n;
                read()
            }
            Err(e) => match e.kind() {
                io::ErrorKind::WouldBlock => read(),
                _ => {
                    println!("read error {:?}", e);
                    self.result_pipe.send(
                        ReqRes{success: false, reason: "read error".to_string()}).unwrap();
                    Next::end()
                }
            }
        }
    }

    fn on_error(&mut self, err: hyper::Error) -> Next {
        println!("ERROR: {}", err);
        self.result_pipe.send(
            ReqRes{success: false, reason: format!("read error: {}", err)}).unwrap();
        Next::remove()
    }
}

pub fn printbp(printer_addr : &String, blueprint : &mut Read) -> Result<(), String> {
    let client = Client::new().unwrap();
    let (tx, rx) = mpsc::channel();

    let url = Url::parse( &*format!("http://{}/print", printer_addr) ).unwrap();

    if client.request( url, PrintOrder::new(tx, blueprint) ).is_err() {
        return Err( "Sending status request failed!".to_string() );
    }

    let response = rx.recv().unwrap();
    if response.success {
        Ok( () )
    }
    else {
        Err( response.reason )
    }
}
