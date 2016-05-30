extern crate hyper;

use hyper::client::{Client, Request, Response, DefaultTransport as HttpStream};
use hyper::header::Connection;
use hyper::{Decoder, Encoder, Next};
use std::sync::mpsc;
use std::io;
use std::time::Duration;
use hyper::Url;


struct Dump(mpsc::Sender<()>);

impl Drop for Dump {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

fn read() -> Next {
    Next::read().timeout(Duration::from_secs(10))
}

impl hyper::client::Handler<HttpStream> for Dump {
    fn on_request(&mut self, req: &mut Request) -> Next {
        req.headers_mut().set(Connection::close());
        read()
    }

    fn on_request_writable(&mut self, _encoder: &mut Encoder<HttpStream>) -> Next {
        read()
    }

    fn on_response(&mut self, res: Response) -> Next {
        println!("Response: {}", res.status());
        println!("Headers:\n{}", res.headers());
        read()
    }

    fn on_response_readable(&mut self, decoder: &mut Decoder<HttpStream>) -> Next {
        match io::copy(decoder, &mut io::stdout()) {
            Ok(0) => Next::end(),
            Ok(_) => read(),
            Err(e) => match e.kind() {
                io::ErrorKind::WouldBlock => Next::read(),
                _ => {
                    println!("ERROR: {}", e);
                    Next::end()
                }
            }
        }
    }

    fn on_error(&mut self, err: hyper::Error) -> Next {
        println!("ERROR: {}", err);
        Next::remove()
    }
}


fn main() {
    let (tx, rx) = mpsc::channel();
    let url = Url::parse("https://0wl.eu/vs.json").unwrap();
    let client = Client::new().unwrap();

    client.request(url, Dump(tx));

    // wait till done
    let _  = rx.recv();
    client.close();
}
