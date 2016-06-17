use super::super::time;

use std::io::{Read, Write};
use std::fs::File;
use std::time::Duration;
use mio::tcp::TcpStream;
use mio::{Timeout, TryRead, EventLoop};

use super::Server;
use super::super::PRINT_TIMEOUT_MS;
use super::super::CONTINUE_DELAY_MS;
use super::BenchWatchStopTime;

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum PrinterPartType {
    Printhead,
    Material
}

pub struct Printerpart {
    pub id: usize,
    pub socket: TcpStream,
    pub parttype: PrinterPartType,
    pub blueprint: Option<Box<Read>>,
    pub job_title: Option<String>,
    pub timeoutid: Option<Timeout>,
    pub matempty: bool,
    pub matid: i32,
    pub benchmarkcnt: i32
}

unsafe impl Send for Printerpart {}
unsafe impl Sync for Printerpart {}

impl Printerpart {
    pub fn new(mut socket: TcpStream, id : usize) -> Printerpart{
        let mut buf = [0];
        let ptype;
        loop {
            ptype = match socket.try_read(&mut buf) {
                Err(_) => panic!("Error while handshaking with new client"),
                Ok(None) => continue,
                Ok(Some(_)) => {
                    match buf[0] {
                        0 => panic!("Something tried to register as an invalid printerpart!"),
                        1 => PrinterPartType::Printhead,
                        _ => PrinterPartType::Material
                    }
                }
            };
            break;
        };
        println!("{:?}", ptype);
        Printerpart {
            id: id,
            socket: socket,
            parttype: ptype,
            blueprint: None,
            job_title: None,
            timeoutid: None,
            matempty: false,
            matid: (buf[0] as i32) - 2,
            benchmarkcnt: 0
        }
    }

    pub fn set_blueprint(self : &mut Self, blueprint : Option<Box<Read>>) {
        self.blueprint = blueprint;
    }

    pub fn abort_benchmark(self : &mut Self) {
        self.benchmarkcnt = 0;
    }

    pub fn load_blueprint(self : &mut Self) {
        self.blueprint = Some( Box::new( File::open("modell.3dbp").unwrap() ) );
        self.job_title = Some( "local job".to_string() );

        //Read & check Magic number
        let mut magic = [0;4];
        self.blueprint.as_mut().unwrap().read_exact(&mut magic).unwrap();
        for i in 0..4 {
            if magic[i] != b"RBAM"[i] {
                panic!("Invalid blueprint magic");
            }
        }
        //b vor String-Literal erzeugt ein ASCII-byte array literal
    }

    pub fn exec_instr(self : &mut Self, eventloop: &mut EventLoop<Server>, matsrc: Option<&mut Printerpart>) {
        let mut commandid = [0];

        let job_title = match self.job_title.as_ref() {
                Some(title)=>title.clone(),
                None => "--".to_string()
        };

        match self.blueprint.as_mut().expect("No blueprint in progess!").read_exact(&mut commandid) {
            Err(_) => {
                println!("Blueprint finished! Job: {}", job_title);
                self.blueprint = None;
                self.job_title = None;
                return
            },
            _ => {}
        }
        let mut bp = self.blueprint.as_mut().unwrap();
        self.socket.write(&commandid).unwrap();

        let matreq = match commandid[0] {
            1 => { //Choose level & mat, 4+1=5byte params
                 let mut params = [0;5];
                 bp.read_exact(&mut params).unwrap();
                 self.matid = params[4] as i32; //New material will be taken from container with id
                 self.socket.write(&params).unwrap();
                 0
            },
            2 => { //Print dot, 2*4=8 byte params
                 let mut params = [0;8];
                 bp.read_exact(&mut params).unwrap();
                 self.socket.write(&params).unwrap();
                 1//A dot takes 1 material unit
            },
            3 => { //Print line, 4*4=16byte params
                let mut params = [0;16];
                bp.read_exact(&mut params).unwrap();
                self.socket.write(&params).unwrap();
                2//A line takes 2 material units
            }
            c => {
                panic!("Unknown blueprint command {:#x}", c);
            }
        };

        if matreq > 0 {
            matsrc.expect("No matching material source available!").sim_mat_usage(matreq);
        }

        self.timeoutid = Some( eventloop.timeout(self.id, Duration::from_millis(PRINT_TIMEOUT_MS)).unwrap() );
    }

    fn sim_mat_usage(self : &mut Self, amount : u8) {
        if amount == 0 {
            return;
        }
        assert!(self.parttype == PrinterPartType::Material, "sim_mat_usage on non-Material!");
        self.socket.write(&[amount]).unwrap();
    }

    fn read_result(self : &mut Self) -> u8 {
        let mut buf = [0];
        loop {
            return match self.socket.try_read(&mut buf) {
                Err(_) => panic!("Error while receiving client data"),
                Ok(None) => continue,
                Ok(Some(_)) => buf[0]
            };
        };
    }

    pub fn notify_printhead(self : &mut Self, eventloop : &mut EventLoop<Server>, matcontainer : Option<&mut Printerpart>) {
        eventloop.clear_timeout(& self.timeoutid.as_mut().expect("Unexpected printhead message!"));
        self.timeoutid = None;

        if self.benchmarkcnt > 0 {
            self.continue_benchmark(eventloop);
            return;
        }
        match self.read_result() {
            1 => {
                if matcontainer.is_some() {
                    self.exec_instr(eventloop, matcontainer)
                }
                else {
                    println!("Printhead({}): Pausing print until material is refilled", self.id);
                }
            },
            255 => {
                println!("Printhead problem, aborting print");
                self.blueprint = None;
            },
            _ => panic!("Unknown printhead status!")
        };
    }

    pub fn notify_material(self : &mut Self, eventloop : &mut EventLoop<Server>, continuedelay : &mut Option<Timeout>) {
        match self.read_result() {
            255 => {
                println!("Material container {} is nearly empty, pausing...", self.matid);
                self.matempty = true;
            },
            1 => {
                println!("Material container {} refilled", self.matid);
                self.matempty = false;
                if continuedelay.is_some() {
                    eventloop.clear_timeout(continuedelay.as_mut().expect(""));
                }
                *continuedelay = Some(eventloop.timeout( 0, Duration::from_millis(CONTINUE_DELAY_MS)).unwrap() );
            },
            _ => panic!("Unknown material status!")
        }
    }

    fn continue_benchmark(self : &mut Self, eventloop : &mut EventLoop<Server>) {
        self.benchmarkcnt -= 1;
        if self.benchmarkcnt == 0 {
            unsafe {
                println!("Benchmark finished, time: {}ms",
                    ( (time::precise_time_ns() - BenchWatchStopTime) as f32) / 1_000_000.0);
            }
            return;
        }
        self.socket.write(&[1, 57, 5, 0, 0, 0]).unwrap();//Arbitrary change level command
        self.timeoutid = Some( eventloop.timeout( self.id, Duration::from_millis(PRINT_TIMEOUT_MS) ).unwrap() );
        return;
    }
}
