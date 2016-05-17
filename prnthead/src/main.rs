extern crate rand;

use std::net::*;
use rand::distributions::*;
use std::str::FromStr;

fn execute_cmd(cmd : [u8;17]) -> Result<(), &'static str> {
    let parambuf = &cmd[1..17];
    match cmd[0]  {
        1 => { //Matlevel
            let zcoor : i32 = (parambuf[0] as i32) | ((parambuf[1] as i32) << 8) |
                ((parambuf[2] as i32) << 16) | ((parambuf[3] as i32) << 24);
            let matid : i32 = parambuf[4] as i32;
            print!("Going to level:{}; using material:{}",zcoor,matid);
        }
        2 => { //Single dot
            let xcoor : i32 = (parambuf[0] as i32) | ((parambuf[1] as i32) << 8) |
                ((parambuf[2] as i32) << 16) | ((parambuf[3] as i32) << 24);
            let ycoor : i32 = (parambuf[4] as i32) | ((parambuf[5] as i32) << 8) |
                ((parambuf[6] as i32) << 16) | ((parambuf[7] as i32) << 24);
            print!("Print dot ({}, {})",xcoor,ycoor);
        }
        3 => { //line
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
            println!("Unknown blueprint command {} received!", cmd[0]);
            return Err("Unknown blueprint command received!");
        }
    }
    return Ok(());
}

fn main() {
    let addr = SocketAddrV4::from_str("127.0.0.1:18000").unwrap();
    let stream = UdpSocket::bind("0.0.0.0:0").unwrap();

    let mut rng = rand::thread_rng();
    let rndrange = Range::new(1, 100);

    stream.send_to(&[1], addr).unwrap(); //Register as printhead
    loop {
        let mut cmd = [0;17]; //Maximum package size 17 = 1 [cmdid] + 4 * 4

        match stream.recv_from(&mut cmd) {
            Err(_) => unreachable!("Error while receiving next command"),
            Ok(_) => {
                print!("R: ");
            }
        };
        match execute_cmd(cmd) {
            Err(msg) => {
                println!(" - Err: {}", msg);
                stream.send_to(&[255], addr).unwrap(); //Report failure
            },
            Ok(_) => {},
        };

        if rndrange.ind_sample(&mut rng) <= 5 { //Fail with 5% probability
            stream.send_to(&[255], addr).unwrap(); //Report failure
            println!(" - SimErr!");
            continue;
        } else { // Report sucess
            stream.send_to(&[1], addr).unwrap();
            println!(" - Done");
        }
    }

}
