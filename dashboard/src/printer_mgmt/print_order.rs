use hyper;
use hyper::{Decoder, Encoder, Next};
use hyper::client::{Client, Request, Response, DefaultTransport as HttpStream};
use hyper::header::Connection;
use std::io;
use std::io::Read;
use std::ops::DerefMut;
use std::sync::{Mutex, Arc};
use std::sync::mpsc;
use std::time::Duration;
use std::collections::HashMap;
use hyper::Url;
use rustc_serialize::json;
use printer_mgmt::printer::{Status, Printer};
use std::str::from_utf8;
use rustc_serialize::base64::ToBase64;

#[derive(RustcEncodable)]
struct PrintReq {
    blueprint: String
}

pub struct PrintOrder {
    result_pipe: mpsc::Sender<Status>,
    req: PrintReq
}

impl PrintOrder {
    pub fn new(result_pipe : mpsc::Sender<Status>) -> Self {
        PrintOrder {
            result_pipe : result_pipe
        }
    }
}

fn read() -> Next {//Helper to generate a read-request with timeout
    Next::read().timeout(Duration::from_millis(300))
}

impl hyper::client::Handler<HttpStream> for PrintOrder {
    fn on_request(&mut self, req: &mut Request) -> Next {
        req.headers_mut().set(Connection::close());
        Next::write_and_read();
    }

    fn on_request_writable(&mut self, _encoder: &mut Encoder<HttpStream>) -> Next {
        unimplemented!();
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
                //println!("decoding '{}' / {:?}", &res_text, res_text.as_bytes() );
                let res : Status = json::decode(res_text).unwrap();
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
                    self.result_pipe.send(Status{busy: true, matempty: false}).unwrap();
                    Next::end()
                }
            }
        }
    }

    fn on_error(&mut self, err: hyper::Error) -> Next {
        println!("ERROR: {}", err);
        self.result_pipe.send(Status{busy: true, matempty: false}).unwrap();
        Next::remove()
    }
}

pub fn update_status(printers : Arc<Mutex<HashMap<usize, Printer>>>) {
    let mut printers_lock = printers.lock().unwrap();
    let mut printers = printers_lock.deref_mut();

    let mut results = HashMap::<usize, mpsc::Receiver<Status>>::new();
    let client = Client::new().unwrap();

    for (id, printer) in printers.iter() {
        let (tx, rx) = mpsc::channel();

        let url = Url::parse( &*format!("http://{}/status", printer.address) ).unwrap();

        if client.request( url, PrintOrder::new(tx) ).is_err() {
            panic!("Sending status request failed!");
        }

        results.insert( *id, rx );
    }

    for (id, result) in &results {
        printers.get_mut(id).unwrap().status = result.recv().unwrap();
    }

    client.close();
}
