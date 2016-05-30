use std::io::prelude::*;
use std::net::TcpStream;
use std::io::stdin;

const MATID : u8 = 0;

fn main() {
    let mut level = 10;
    let mut stream = TcpStream::connect("127.0.0.1:18000").unwrap();
    stream.write(&[(2+MATID)]).unwrap(); //Register as material

    let mut input = String::new();
    loop{
        let mut usebuf = [0];
        match stream.read_exact(&mut usebuf) {
            Ok(_) => {},
            Err(_) => {
                unreachable!("Error while reading used material");
            }
        }
        level -= usebuf[0]; //material abziehen
        println!("Matlevel: {}", level);
        if level > 2 {
            continue;
        }
        println!("Nearly empty, halting!", );

        let _ = stream.write(&[255]); //notify nearly empty
        stdin().read_line(&mut input).unwrap(); //wait till enter to reset
        level = 20;
        let _ = stream.write(&[1]); //notify nearly empty
    }

}
