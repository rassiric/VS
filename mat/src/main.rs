use std::net::*;
use std::str::FromStr;

const MATID : u8 = 0;

fn main() {
    let mut streams = Vec::<UdpSocket>::new();
    let addr = SocketAddrV4::from_str("127.0.0.1:18000").unwrap();

    let mut i = 1;

    loop {
        let stream = match UdpSocket::bind("0.0.0.0:0") {
            Err(e) => {
                println!("Error while opening {}. socket: {}", i, e);
                return
            },
            Ok(s) => s
        };
        i+=1;
        stream.send_to(&[(2+MATID)], &addr).unwrap(); //Register as material
        streams.push(stream);
    }
}
