extern crate rand;

use std::io::prelude::*;
use std::net::TcpStream;
use rand::distributions::*;

fn execute_cmd(stream : &mut TcpStream, cmd : u8) -> Result<(), &'static str> {
    match cmd  {
        1 => { //Matlevel
            let mut parambuf = [0;5]; // 4, 1 byte Parameter
            match stream.read_exact(&mut parambuf) {
                Err(_) => return Err("Cannot receive matlevel parameters!"),
                Ok(_) => {}
            }
            let zcoor : i32 = (parambuf[0] as i32) | ((parambuf[1] as i32) << 8) |
                ((parambuf[2] as i32) << 16) | ((parambuf[3] as i32) << 24);
            let matid : i32 = parambuf[4] as i32;
            print!("Going to level:{}; using material:{}",zcoor,matid);
        }
        2 => { //Single dot
            let mut parambuf = [0;8]; // 2 * 4 byte Parameter
            match stream.read_exact(&mut parambuf) {
                Err(_) => return Err("Cannot receive dot parameters!"),
                Ok(_) => {}
            }
            let xcoor : i32 = (parambuf[0] as i32) | ((parambuf[1] as i32) << 8) |
                ((parambuf[2] as i32) << 16) | ((parambuf[3] as i32) << 24);
            let ycoor : i32 = (parambuf[4] as i32) | ((parambuf[5] as i32) << 8) |
                ((parambuf[6] as i32) << 16) | ((parambuf[7] as i32) << 24);
            print!("Print dot ({}, {})",xcoor,ycoor);
        }
        3 => { //line
            let mut parambuf = [0;16]; // 4 * 4 byte Parameter
            match stream.read_exact(&mut parambuf) {
                Err(_) => return Err("Cannot receive line parameters!"),
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
            print!("Print line from ({}, {}) to ({}, {})",startx,starty,endx,endy);
            std::thread::sleep(std::time::Duration::from_millis(3000));
        }
        _ => {
            return Err("Unknown blueprint command received!");
        }
    }
    return Ok(());
}

fn main() {

    let mut stream = TcpStream::connect("127.0.0.1:18000").unwrap();

    let mut rng = rand::thread_rng();
    let rndrange = Range::new(1, 100);

    let _ = stream.write(&[1]); //Register as printhead
    loop {
        let mut cmd = [0];
        match stream.read_exact(&mut cmd) {
            Err(_) => {
                println!("Connection closed, exiting");
                return;
            },
            Ok(_) => {
                print!("R: ");
            }
        };
        match execute_cmd(&mut stream, cmd[0]) {
            Err(msg) => {
                println!(" - Err: {}", msg);
                stream.write(&[255]).unwrap(); //Report failure
                continue;
            },
            Ok(_) => {},
        };

        stream.write(&[1]).unwrap();
        println!(" - Done");
    }

}
