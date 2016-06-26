use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::str::from_utf8;
use printer_mgmt::{Printer, printbp};
use mqtt::async::{PersistenceType, Qos, AsyncClient, AsyncConnectOptions};

fn queue_job(printers: Arc<Mutex<HashMap<usize, Printer>>>,
    job_queue : Arc<Mutex<Vec<(usize, String, String)>>>,
    msg_payload : &str) -> Result<String, String> {
    let v: Vec<&str> = msg_payload.split(|c| c == ';').collect();

    if v.len() != 3 {
        return Err("Invalid payload, expected <FAB>;<BP>;<Title>".to_string());
    }

    let fab = try!(v[0].parse().or(Err("Cannot parse fabid".to_string())));
    return printbp( printers, job_queue, fab, v[1].to_string(), &v[2].to_string() );
}

fn publish_error(msg : &str, client : &mut AsyncClient) {
    println!("MQTT task failed: {}", msg);
    let _ = client.send(format!("Failure: {}", msg).as_bytes(), "queueFeedback", Qos::OnceAndOneOnly, false);
}

pub fn work(printers: Arc<Mutex<HashMap<usize, Printer>>>,
    job_queue : Arc<Mutex<Vec<(usize, String, String)>>>,
    broker_addr : &str) {

    let connection_options = AsyncConnectOptions::new();
    let mut client = AsyncClient::new(broker_addr, "dashboard", PersistenceType::Nothing, None).expect("Cannot create MQTT client!");
    client.connect(&connection_options).expect("Cannot connect to MQTT broker!");

    client.subscribe("queueJob", Qos::OnceAndOneOnly).expect("Cannot subscribe to queueJob topic!");
    loop {
        for message in client.messages(None) {
            //println!("{:?}", message);
            match &*message.topic {
                "queueJob" => {
                    match message.payload {
                        None => publish_error("queueJob without payload", &mut client),
                        Some(payload) => match from_utf8(&*payload) {
                            Err(_) => publish_error("queueJob with non-UTF8 payload!", &mut client),
                            Ok(payload_msg) => match queue_job(printers.clone(), job_queue.clone(), payload_msg) {
                                Ok(msg) => {
                                    let _ = client.send(msg.as_bytes(), "queueFeedback", Qos::OnceAndOneOnly, false);
                                },
                                Err(e) => publish_error(&*e, &mut client)
                            }
                        }
                    };
                }
                _ => {}
            }
        }
    }
}
