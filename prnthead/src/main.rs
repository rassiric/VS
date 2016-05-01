extern crate rand;

use std::io::prelude::*;
use std::net::TcpStream;
use std::io::stdin;
use rand::distributions::*;
use rand::Rng;

fn main() {

    let mut stream = TcpStream::connect("127.0.0.1:18000").unwrap();

    let mut rng = rand::thread_rng();
    let rndrange = Range::new(1, 100);

    let _ = stream.write(&[1]); //Register as printhead
    loop {
        let mut recbuf = [0;1];
        match stream.read(&mut recbuf) {
            Err(_) => unreachable!("Error while receiving next command"),
            Ok(_) => {
                println!("Received command");
            }
        };
        if(rndrange.ind_sample(&mut rng) <= 5) { //Fail with 5% probability
            stream.write(&[255]); //Report failure
            println!("Simulating failure");
            continue;
        }
        match(recbuf[0]) {
            1 => { //Matlevel
                let mut parambuf = [0;8]; // 2 * 4 byte Parameter
                match stream.read_exact(&mut parambuf) {
                    Err(_) => unreachable!("Error while receiving params"),
                    Ok(_) => {}
                }
                let zcoor : i32 = (parambuf[0] as i32) | ((parambuf[1] as i32) << 8) |
                    ((parambuf[2] as i32) << 16) | ((parambuf[3] as i32) << 24);
                println!("{} {}", parambuf[0], zcoor);
            }
            _ => {
                println!("Unknown blueprint command received!");
            }
        }
    }

}
