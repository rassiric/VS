use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use printer_mgmt::{Printer, printbp};
use mqtt::async::{PersistenceType, Qos, MqttError, AsyncClient, AsyncConnectOptions, AsyncDisconnectOptions};

pub fn work(printers: Arc<Mutex<HashMap<usize, Printer>>>,
    job_queue : Arc<Mutex<Vec<(usize, String, String)>>>,
    broker_addr : &str) {

    let connection_options = AsyncConnectOptions::new();
    let mut client = AsyncClient::new(broker_addr, "dashboard", PersistenceType::Nothing, None).expect("Cannot create MQTT client!");
    client.connect(&connection_options).expect("Cannot connect to MQTT broker!");

    client.subscribe("queueJob", Qos::OnceAndOneOnly);
    loop {
        for message in client.messages(None) {
            println!("MQTT {:?}", message);
        }
    }
}
