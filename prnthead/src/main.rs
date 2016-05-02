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
        let mut cmd = [0;1];
        match stream.read(&mut cmd) {
            Err(_) => unreachable!("Error while receiving next command"),
            Ok(_) => {
                println!("Received command");
            }
        };
        match(cmd[0]) {
            1 => { //Matlevel
                let mut parambuf = [0;8]; // 2 * 4 byte Parameter
                match stream.read_exact(&mut parambuf) {
                    Err(_) => unreachable!("Error while receiving params"),
                    Ok(_) => {}
                }
                let zcoor : i32 = (parambuf[0] as i32) | ((parambuf[1] as i32) << 8) |
                    ((parambuf[2] as i32) << 16) | ((parambuf[3] as i32) << 24);
                let matid : i32 = (parambuf[4] as i32) | ((parambuf[5] as i32) << 8) |
                    ((parambuf[6] as i32) << 16) | ((parambuf[7] as i32) << 24);
                println!("Going to level:{}; using material:{}",zcoor,matid);
            }
            2 => { //Single dot
                let mut parambuf = [0;8]; // 2 * 4 byte Parameter
                match stream.read_exact(&mut parambuf) {
                    Err(_) => unreachable!("Error while receiving params"),
                    Ok(_) => {}
                }
                let xcoor : i32 = (parambuf[0] as i32) | ((parambuf[1] as i32) << 8) |
                    ((parambuf[2] as i32) << 16) | ((parambuf[3] as i32) << 24);
                let ycoor : i32 = (parambuf[4] as i32) | ((parambuf[5] as i32) << 8) |
                    ((parambuf[6] as i32) << 16) | ((parambuf[7] as i32) << 24);
                println!("Print dot ({}, {})",xcoor,ycoor);
            }
            3 => { //line
                let mut parambuf = [0;16]; // 4 * 4 byte Parameter
                match stream.read_exact(&mut parambuf) {
                    Err(_) => unreachable!("Error while receiving params"),
                    Ok(_) => {}
                }
                let startx : i32 = (parambuf[0] as i32) | ((parambuf[1] as i32) << 8) |
                    ((parambuf[2] as i32) << 16) | ((parambuf[3] as i32) << 24);
                let starty : i32 = (parambuf[4] as i32) | ((parambuf[5] as i32) << 8) |
                    ((parambuf[6] as i32) << 16) | ((parambuf[7] as i32) << 24);
                let endx : i32 = (parambuf[8] as i32) | ((parambuf[9] as i32) << 8) |
                    ((parambuf[10] as i32) << 16) | ((parambuf[11] as i32) << 24);
                let endy : i32 = (parambuf[12] as i32) | ((parambuf[13] as i32) << 8) |
                    ((parambuf[14] as i32) << 16) | ((parambuf[15] as i32) << 24);
                println!("Print line from ({}, {}) to ({}, {})",startx,starty,endx,endy);
            }
            _ => {
                println!("Unknown blueprint command received!");
            }
        }
        if(rndrange.ind_sample(&mut rng) <= 5) { //Fail with 5% probability
            stream.write(&[255]); //Report failure
            println!("Simulating failure");
            continue;
        } else { // OK - Meldung
            stream.write(&[1]);
        }
    }

}
