use std::io::prelude::*;
use std::net::TcpStream;
use std::io::stdin;

fn main() {

    let mut stream = TcpStream::connect("127.0.0.1:18000").unwrap();
    let _ = stream.write(&[2]); //Register as printhead

    let mut input = String::new();
    loop{
        stdin().read_line(&mut input).unwrap();
        let _ = stream.write(&[255]); //notify nearly empty
        stdin().read_line(&mut input).unwrap();
        let _ = stream.write(&[1]); //notify nearly empty
    }

}
