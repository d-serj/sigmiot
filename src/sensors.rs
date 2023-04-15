use std::collections::HashMap;

use bme280::i2c::BME280;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c::{Read, Write, WriteRead};
use embassy_time::{Timer, Duration};
use esp_idf_hal::task::embassy_sync::EspRawMutex;
use embassy_sync::channel::Sender;

use crate::DATA_CHANNEL_SIZE;
use crate::data_provider::DataMessage;

#[derive(Debug, Clone)]
pub struct SensorValue {
    pub value_name: String,
    pub value: f32,
    pub unit: String,
}

#[derive(Debug, Clone)]
pub struct SensorData {
    name: String,
    values: HashMap<String, SensorValue>,
}

impl SensorData {
    pub fn new(sensor_name: &str) -> Self {
        Self {
            name: sensor_name.into(),
            values: HashMap::new(),
        }
    }

    pub fn push_value(&mut self, name: &str, value: f32, unit: &str) {
        let sensor_value = self.values.entry(name.to_string()).or_insert(SensorValue {
            value_name: name.into(),
            value,
            unit: unit.into(),
        });

        sensor_value.value = value;
    }

    pub fn get_values(&self) -> Vec<&SensorValue> {
        self.values.values().collect()
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }
}

pub trait Sensor {
    fn init(&mut self) -> Result<(), ()>;
    fn measure_cmd(&mut self);
    fn read(&mut self);
    fn get_data(&self) -> &SensorData;
    fn get_name(&self) -> &String;
}

pub struct GY30Sensor<I2C, D> {
    i2c: I2C,
    addr: u8,
    delay: D,
    data: SensorData,
}

impl<I2C, D, E> GY30Sensor<I2C, D>
where
    I2C: Read<Error = E> + Write<Error = E> + WriteRead<Error = E>,
    D: DelayMs<u8>,
{
    const GY30_I2C_ADDR: u8 = 0x23;
    const BH1750_CONTINUOUS_HIGH_RES_MODE: u8 = 0x10;

    pub fn new(sensor_name: &str, i2c: I2C, delay: D) -> Self {
        Self {
            i2c,
            addr: GY30Sensor::<I2C, D>::GY30_I2C_ADDR,
            delay,
            data: SensorData::new(sensor_name),
        }
    }
}

impl<I2C, D, E> Sensor for GY30Sensor<I2C, D>
where
    I2C: Read<Error = E> + Write<Error = E> + WriteRead<Error = E>,
    D: DelayMs<u8>,
{
    fn init(&mut self) -> Result<(), ()> {
        // Configure the BH1750 sensor
        self.i2c
            .write(
                self.addr,
                &[GY30Sensor::<I2C, D>::BH1750_CONTINUOUS_HIGH_RES_MODE],
            )
            .map_err(|_| ())?;
        self.delay.delay_ms(180);
        Ok(())
    }

    fn read(&mut self) {
        let mut buffer: [u8; 2] = [0, 0];

        // Read the illumination level
        self.i2c.write(self.addr, &[0x00]).map_err(|_| ()).expect("Cannot write I2c!");
        // Should wait more than 180ms
        self.delay.delay_ms(200);
        self.i2c.read(self.addr, &mut buffer).map_err(|_| ()).expect("Cannot read I2c!");

        let illumination_level = ((buffer[0] as u16) << 8) | (buffer[1] as u16);

        self.data
            .push_value("illuminance", illumination_level as f32, "lx");
    }

    fn get_data(&self) -> &SensorData {
        &self.data
    }

    fn get_name(&self) -> &String {
        &self.data.name
    }

    fn measure_cmd(&mut self) {
        self.init().unwrap()
    }
}

pub struct BME280Sensor<I2C, D> {
    bme280: BME280<I2C, D>,
    data: SensorData,
}

impl<I2C, D, E> BME280Sensor<I2C, D>
where
    I2C: Read<Error = E> + Write<Error = E> + WriteRead<Error = E>,
    D: DelayMs<u8>,
{
    pub fn new(sensor_name: &str, i2c: I2C, delay: D) -> Self {
        Self {
            bme280: BME280::new_primary(i2c, delay),
            data: SensorData::new(sensor_name),
        }
    }
}

impl<I2C, D, E> Sensor for BME280Sensor<I2C, D>
where
    I2C: Read<Error = E> + Write<Error = E> + WriteRead<Error = E>,
    D: DelayMs<u8>,
    E: std::fmt::Debug
{
    fn init(&mut self) -> Result<(), ()> {
        //self.bme280.init().map_err(|_| ())
        let res = self.bme280.init();

        match res {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("Error: {:?}", e);
                Err(())
            }
        }
    }

    fn read(&mut self) {
        let temperature = self.bme280.measure().map_err(|_| ()).unwrap().temperature;
        let humidity = self.bme280.measure().map_err(|_| ()).unwrap().humidity;
        let pressure = self.bme280.measure().map_err(|_| ()).unwrap().pressure;

        self.data.push_value("temperature", temperature, "°C");
        self.data.push_value("humidity", humidity, "%");
        self.data.push_value("pressure", pressure, "hPa");
    }

    fn get_data(&self) -> &SensorData {
        &self.data
    }

    fn get_name(&self) -> &String {
        &self.data.name
    }

    fn measure_cmd(&mut self) {
        // nothing to do for BME280
    }
}

pub struct SensorManager {
    sensors: Vec<Box<dyn Sensor>>,
}

impl SensorManager {
    pub fn new() -> Self {
        Self { sensors: vec![], }
    }

    pub fn add_sensor(&mut self, sensor: Box<dyn Sensor>) {
        self.sensors.push(sensor);
    }

    fn get_sensors(&self) -> &Vec<Box<dyn Sensor>> {
        &self.sensors
    }

    fn get_sensor(&self, sensor_name: &str) -> Option<&Box<dyn Sensor>> {
        self.get_sensors().iter().find(|s| s.get_name() == sensor_name)
    }

    pub fn get_sensor_data(&self, sensor_name: &str) -> Option<&SensorData> {
        self.get_sensor(sensor_name).map(|s| s.get_data())
    }

    pub fn get_sensor_values(&self, sensor_name: &str) -> Option<Vec<&SensorValue>> {
        self.get_sensor_data(sensor_name).map(|s| s.get_values())
    }

    pub fn measure(&mut self) {
        for sensor in self.sensors.iter_mut() {
            sensor.measure_cmd();
        }
    }

    pub fn read(&mut self) {
        for sensor in self.sensors.iter_mut() {
            sensor.read();
        }
    }

    #[allow(dead_code)]
    pub fn print_sensors_data(&self) {
        for sensor in self.sensors.iter() {
            println!("{}:", sensor.get_name());
            for value in sensor.get_data().get_values() {
                println!("{}: {} {}", value.value_name, value.value, value.unit);
            }
        }
    }
}

pub async fn run_sensor_manager(
    mut sensor_manager: SensorManager,
    sender: Sender<'static, EspRawMutex, DataMessage, DATA_CHANNEL_SIZE>
) {
    loop {
        sensor_manager.measure();
        sensor_manager.read();
        //sensor_manager.print_sensors_data().await;

        let mut msg: DataMessage = DataMessage {
            data: Vec::with_capacity(sensor_manager.get_sensors().len())
        };

        for sensor in sensor_manager.get_sensors().iter() {
            msg.data.push(sensor.get_data().clone());
        }

        sender.send(msg).await;

        Timer::after(Duration::from_secs(1)).await;
    }
}
