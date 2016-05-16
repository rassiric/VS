use std::io::prelude::*;
use std::net::*;
use std::io::stdin;
use std::str::FromStr;

const MATID : u8 = 0;

fn main() {
    let mut level = 10;
    let addr = SocketAddrV4::from_str("127.0.0.1:18000").unwrap();
    let stream = UdpSocket::bind("0.0.0.0:0").unwrap(); //Bind to any IP, let OS choose port
    stream.send_to(&[(2+MATID)], &addr).unwrap(); //Register as material

    let mut input = String::new();
    loop{
        let mut usebuf = [0];
        match stream.recv_from(&mut usebuf) {
            Ok((_, source)) => {
                match source {
                    SocketAddr::V4(sourceaddr) => {
                        if sourceaddr != addr {
                            println!("Something is running interference!");
                            continue;
                        }
                    },
                    SocketAddr::V6(_) => {
                        println!("Something is running interference!");
                        continue;
                    }
                }

            },
            Err(err) => {
                unreachable!("Error while reading used material: {}", err);
            }
        }
        level -= usebuf[0]; //material abziehen
        println!("Matlevel: {}", level);
        if level > 2 {
            continue;
        }
        println!("Nearly empty, halting!", );

        let _ = stream.send_to(&[255], addr); //notify nearly empty
        stdin().read_line(&mut input).unwrap(); //wait till enter to reset
        level = 20;
        let _ = stream.send_to(&[1], addr); //notify nearly empty
    }

}
