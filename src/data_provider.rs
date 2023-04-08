use std::{sync::{Arc, Mutex}, time::Duration};

use esp_idf_hal::task::embassy_sync::EspRawMutex;
use embassy_sync::channel::Receiver;

use crate::{sensors::SensorData, DATA_CHANNEL_SIZE};

pub struct DataMessage {
    pub data: Vec<SensorData>,
}

pub struct DataProvider {
    data_receiver: Receiver<'static, EspRawMutex, DataMessage, DATA_CHANNEL_SIZE>,
    prev_data: Vec<SensorData>,
}

impl DataProvider {
    pub fn new(
        data_receiver: Receiver<'static, EspRawMutex, DataMessage, DATA_CHANNEL_SIZE>,
    ) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self { data_receiver, prev_data: Vec::new(), }))
    }

    fn get_data(&mut self) -> &Vec<SensorData> {
        if let Ok(msg) = self.data_receiver.try_recv() {
            self.prev_data = msg.data.clone();
            &self.prev_data
        }
        else {
            &self.prev_data
        }
    }

    pub fn get_http_data(&mut self) -> String {
        let sensors_data = self.get_data();
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
}
