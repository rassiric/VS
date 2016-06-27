extern crate mqtt;
extern crate time;

use std::thread;
use std::str::from_utf8;
use std::io;
use std::io::BufRead;
use mqtt::async::{PersistenceType, Qos, AsyncClient, AsyncConnectOptions};

static mut BenchCounter : i64 = 0;
static mut BenchWatchStopTime : u64 = 0;

fn outputloop() {
    let connection_options = AsyncConnectOptions::new();
    let mut client = AsyncClient::new("127.0.0.1", "client_display", PersistenceType::Nothing, None).expect("Cannot create MQTT client!");
    client.connect(&connection_options).expect("Cannot connect to MQTT broker!");

    client.subscribe("queueFeedback", Qos::OnceAndOneOnly).expect("Cannot subscribe to queueFeedback topic!");
    client.subscribe("printInfo", Qos::OnceAndOneOnly).expect("Cannot subscribe to printInfo topic!");

    loop {
        for message in client.messages(None) {
            match message.payload {
                None => println!("{}: No payload", message.topic),
                Some(payload) => match from_utf8(&*payload) {
                    Err(_) => println!("{}: Non-UTF8 payload", message.topic),
                    Ok(payload_msg) => {
                        println!("{}: {}", message.topic, payload_msg);
                        unsafe {
                            if BenchCounter > 0 &&
                                    &*message.topic == "printInfo" &&
                                    payload_msg.contains("Done") {
                                BenchCounter -= 1;
                                if BenchCounter == 0 {
                                    println!("Benchmark: {}ms", (time::precise_time_ns() - BenchWatchStopTime) / 1_000_000);
                                }
                            }
                        } 
                    }
                }
            };
        }
    }
}

fn main() {
    println!("VS FAB Msgclient");
    println!("Input:");
    println!("\tb - Benchmark");
    println!("\t<FabId>;<BP-Name>;<Job title>");

    let _mqttthread = thread::spawn( outputloop );

    let connection_options = AsyncConnectOptions::new();
    let mut client = AsyncClient::new("127.0.0.1", "client_input", PersistenceType::Nothing, None).expect("Cannot create MQTT client!");
    client.connect(&connection_options).expect("Cannot connect to MQTT broker!"); 

    loop {
        let mut inp = String::new();
        let cons = io::stdin();
        if cons.lock().read_line(&mut inp).unwrap() <= 0 {
            break;
        }

        match inp.trim() {
            "b" => {
                unsafe {
                    BenchCounter = 50;
                    BenchWatchStopTime = time::precise_time_ns();
                }
                for _ in 0..50 {
                    let _ = client.send("0;bm;bm".to_string().as_bytes(), "queueJob", Qos::OnceAndOneOnly, false);
                }
            },
            _ => { let _ = client.send(inp.trim().as_bytes(), "queueJob", Qos::OnceAndOneOnly, false); }
        }
    }
}
