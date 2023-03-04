

use crate::sensors::Sensor;

pub struct DataProvider {
     sensors: Vec<Box<dyn Sensor>>,
}

impl DataProvider {
     // fn new() -> Self {
     //      Self { sensors: vec![] }
     // }

     // fn add_sensor(&mut self, sensor: dyn Sensor) {
     //      self.sensors.
     // }

     // fn get_http_data(&self) -> String {
     //      writeln!(buf, "<h2>{}</h2>", self.name).unwrap();
     //      writeln!(buf, "<ul>").unwrap();
     //      for (name, value) in &self.values {
     //          writeln!(buf, "<li>{}: {}</li>", name, value).unwrap();
     //      }
     //      writeln!(buf, "</ul>").unwrap();
     //      buf
     //  }
     // }
}


