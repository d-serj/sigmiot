
use std::env;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use futures_util::{future, pin_mut, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol};

use protobuf::{EnumOrUnknown, Message};

include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));

use sensors_data::{sensor_data_response, SensorDataResponse};

#[derive(Debug, Clone)]
pub struct SensorValue {
    pub value_name: String,
    pub value: f32,
    pub unit: String,
}

#[derive(Debug, Clone)]
pub struct SensorData {
    sensor_name: String,
    sensor_type: String,
    sensor_location: String,
    sensor_values: Vec<SensorValue>,
}

pub struct DataMessage {
    pub data: Vec<SensorData>,
}

#[tokio::main]
async fn main() {
    // Encode example request
    //let url = "ws://192.168.116.62/ws";
    let connect_addr =
        env::args().nth(1).unwrap_or_else(|| panic!("this program requires at least one argument"));
    let url = url::Url::parse(&connect_addr).expect("Cannot parse URL");
    let (ws, _) = connect_async(&url).await.expect("Failed to connect");
    println!("Connected to {}", url);
    
    let (write, read) = ws.split();

    let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();
    tokio::spawn(read_stdin(stdin_tx));

    let stdin_to_ws = stdin_rx.map(Ok).forward(write);

    let ws_to_stdout = {
        read.for_each(|message| async {
            let data = message.unwrap().into_data();
            let in_msg = SensorDataResponse::parse_from_bytes(&data).unwrap();

            let sensors_data = in_msg.sensors_data;

            if in_msg.status == EnumOrUnknown::new(sensor_data_response::Status::OK) {
                for sensor in sensors_data {
                    println!("Sensor name: {}", sensor.sensor_name);
                    println!("Sensor type: {}", sensor.sensor_type);
                    println!("Sensor location: {}", sensor.sensor_location);
                    for value in sensor.sensor_values {
                        println!("Value name: {}", value.value_name);
                        println!("Value: {}", value.value_data);
                        println!("Unit: {}", value.value_unit);
                    }
                }
            }
            else {
                println!("Error: {:?}", in_msg.status);
            }

            //println!("In msg {:#?}", in_msg);
            //tokio::io::stdout().write_all(sensor_name).await.unwrap();
        })
    };

    pin_mut!(stdin_to_ws, ws_to_stdout);
    future::select(stdin_to_ws, ws_to_stdout).await;
}

// Our helper method which will read data from stdin and send it along the
// sender provided.
async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<protocol::Message>) {
    let mut stdin = tokio::io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        //tx.unbounded_send(protocol::Message::binary(String::from_utf8(buf).unwrap())).unwrap();
        tx.unbounded_send(protocol::Message::binary(buf)).unwrap();
    }
}
