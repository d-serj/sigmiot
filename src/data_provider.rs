use std::{collections::HashMap, sync::{Arc, Mutex}};

use crate::sensors::SensorData;

pub struct DataProvider {
    sensors: HashMap<String, SensorData>,
}

impl DataProvider {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self { sensors: HashMap::new() }))
    }

    pub fn push_data(&mut self, name: &str, sensor_data: &SensorData) {

        let sensor_values = self
            .sensors
            .entry(name.to_string())
            .or_insert(SensorData::new(name));

        let input_sensor_values = sensor_data.get_values();

        for sensor_value in input_sensor_values.iter() {
             let (name, value) = sensor_value.get_name_n_value();
             sensor_values.push_value(name, value);
        }
    }

    pub fn get_http_data(&self) -> String {
        let mut buf = String::new();

        let sensors = &self.sensors;

        for sensor in sensors.iter() {
            buf.push_str(&format!("<h2>{}</h2>\n", sensor.0));
            buf.push_str("<ul>\n");

            let sensor_data = sensor.1;
            let sensor_values = sensor_data.get_values();
            for val_ref in sensor_values {
                let (name, value) = val_ref.get_name_n_value();
                buf.push_str(&format!("<li>{}: {}</li>\n", name, value));
            }

            buf.push_str("</ul>\n");
        }

        buf
    }
}
