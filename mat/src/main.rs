use std::io::prelude::*;
use std::net::TcpStream;

const MATID : u8 = 0;

fn main() {
    let mut streams = Vec::<TcpStream>::new();
    let mut i = 1;

    loop {
        let mut stream = match TcpStream::connect("127.0.0.1:18000"){
            Err(e) => {
                println!("Error while opening {}. socket: {}", i, e);
                return
            },
            Ok(s) => s
        };
        i+=1;
        stream.write(&[(2+MATID)]).unwrap(); //Register as material
        streams.push(stream);
    }
}
