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
             let value_name = &sensor_value.value_name;
             let value = sensor_value.value;
             let unit = &sensor_value.unit;
             sensor_values.push_value(value_name, value, unit.as_str());
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
