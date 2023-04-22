use std::sync::{Arc, Mutex};

use esp_idf_hal::task::embassy_sync::EspRawMutex;
use lazy_static::lazy_static;
use log::{debug, info};
use protobuf::{EnumOrUnknown, Message, MessageField};

include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));

use sigmiot_data::{sensor_data_response, SensorData, SensorValue, SensorDataResponse, MessageResponse, LogDataResponse};

use crate::{sensors, sigmiot_log::remote_logger_get_entries};

pub struct DataMessage {
    pub data: Vec<sensors::SensorData>,
}

const DATA_CHANNEL_SIZE: usize = 2;
static CHANNEL: embassy_sync::channel::Channel<EspRawMutex, DataMessage, DATA_CHANNEL_SIZE> =
    embassy_sync::channel::Channel::new();

fn get_data() -> Vec<sensors::SensorData> {
    lazy_static!{ static ref PREV_DATA: Arc<Mutex<Vec<sensors::SensorData>>> = Arc::new(Mutex::new(Vec::new())); }

    if let Ok(msg) = CHANNEL.receiver().try_recv() {
        *PREV_DATA.lock().unwrap() = msg.data.clone();
        PREV_DATA.lock().unwrap().clone()
    }
    else {
        PREV_DATA.lock().unwrap().clone()
    }
}

pub async fn publish_async(data: Vec<sensors::SensorData>) {
    let msg = DataMessage { data };
    CHANNEL.sender().send(msg).await;
}

async fn get_data_async() -> Vec<sensors::SensorData> {
    let msg = CHANNEL.receiver().recv().await;
    msg.data
}

pub fn get_http_data() -> String {
    let sensors_data = get_data();
    let mut buf = String::new();

    for sensor in sensors_data.iter() {
        buf.push_str(&format!("<h2>{}</h2>\n", sensor.get_name()));
        buf.push_str("<ul>\n");

        let sensor_values = sensor.get_values();
        for val_ref in sensor_values {
            let value_name = val_ref.value_name.as_str();
            let value = val_ref.value;
            let unit = val_ref.unit.as_str();
            buf.push_str(&format!("<li>{}: {} {}</li>\n", value_name, value as i32, unit));
        }

        buf.push_str("</ul>\n");
    }

    buf
}

pub async fn get_protobuf_data_async() -> Vec<u8> {
    let sensors_data = get_data_async().await;

    let mut msg_response = MessageResponse::new();

    let mut sensor_data_response = SensorDataResponse::new();
    sensor_data_response.status = EnumOrUnknown::new(sensor_data_response::Status::OK);

    for sensor in sensors_data.iter() {
        let mut sensor_data = SensorData::new();

        sensor_data.sensor_name = sensor.get_name().to_owned();
        // TODO: add sensor type and location
        sensor_data.sensor_type = "thp".to_string();
        sensor_data.sensor_location = "inside".to_string();

        let sensor_values = sensor.get_values();
        for val_ref in sensor_values {
            let mut sensor_value = SensorValue::new();
            sensor_value.value_name = val_ref.value_name.clone();
            sensor_value.value_data = val_ref.value;
            sensor_value.value_unit = val_ref.unit.clone();

            sensor_data.sensor_values.push(sensor_value);
        }

        sensor_data_response.sensors_data.push(sensor_data);
    }

    msg_response.sensor_data_response = MessageField::<SensorDataResponse>::some(sensor_data_response);

    let log_entries = remote_logger_get_entries();
    
    for entry in log_entries {
        let mut log_entry = LogDataResponse::new();
        log_entry.log_level = entry.level;
        log_entry.log_message = entry.message;
        log_entry.log_timestamp = entry.timestamp;

        msg_response.log_data_response.push(log_entry);
    }

    info!("Log data response size: {}", msg_response.log_data_response.len());

    msg_response.write_to_bytes().unwrap()
}
