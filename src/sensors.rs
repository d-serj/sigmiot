use std::collections::HashMap;

use bme280::i2c::BME280;
use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::i2c::{Read, Write, WriteRead};

#[derive(Debug)]
pub struct SensorValue {
    value_name: String,
    value: f32,
}

impl SensorValue {
    pub fn get_name_n_value(&self) -> (&String, f32) {
        ( &self.value_name, self.value )
    }
}

#[derive(Debug)]
pub struct SensorData {
    name: String,
    values: HashMap<String, SensorValue>,
}

impl SensorData {
    fn new(sensor_name: &str) -> Self {
        Self {
            name: sensor_name.into(),
            values: HashMap::new(),
        }
    }

    fn push_value(&mut self, name: &str, value: f32) {
        let sensor_value = self.values.entry(name.to_string()).or_insert(SensorValue {
            value_name: name.into(),
            value,
        });

        sensor_value.value = value;
    }
}

pub trait Sensor {
    fn init(&mut self) -> Result<(), ()>;
    fn read(&mut self);
    fn get_values(&self) -> Vec<&SensorValue>;
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
        self.i2c.write(self.addr, &[0x00]).map_err(|_| ());
        // Should wait more than 180ms
        self.delay.delay_ms(200);
        self.i2c.read(self.addr, &mut buffer).map_err(|_| ());

        let illumination_level = ((buffer[0] as u16) << 8) | (buffer[1] as u16);

        self.data
            .push_value("illuminance", illumination_level as f32);
    }

    fn get_values(&self) -> Vec<&SensorValue> {
        self.data.values.values().collect()
    }

    fn get_name(&self) -> &String {
        &self.data.name
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
{
    fn init(&mut self) -> Result<(), ()> {
        self.bme280.init().map_err(|_| ())
    }

    fn read(&mut self) {
        let temperature = self.bme280.measure().map_err(|_| ()).unwrap().temperature;
        let humidity = self.bme280.measure().map_err(|_| ()).unwrap().humidity;
        let pressure = self.bme280.measure().map_err(|_| ()).unwrap().pressure;

        self.data.push_value("temperature", temperature as f32);
        self.data.push_value("humidity", humidity as f32);
        self.data.push_value("pressure", pressure as f32);
    }

    fn get_values(&self) -> Vec<&SensorValue> {
        self.data.values.values().collect()
    }

    fn get_name(&self) -> &String {
        &self.data.name
    }
}
