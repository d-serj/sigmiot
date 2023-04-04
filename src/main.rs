mod data_provider;
mod data_transfer;
mod sensors;
mod wifi;

use data_transfer::httpd;
use esp_idf_sys::{self as _, EspError};
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::time::Duration;

use esp_idf_hal::delay;
use esp_idf_hal::i2c::{self};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::units::FromValueType;

use sensors::{BME280Sensor, GY30Sensor, Sensor, SensorData};
use wifi::Wifi;
pub use crate::data_provider::DataProvider;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    let mut peripherals = Peripherals::take().unwrap();

    let mut scl = peripherals.pins.gpio22;
    let mut sda = peripherals.pins.gpio21;

    let i2c0 = &mut peripherals.i2c0;

    let mut wifi = Wifi::new(peripherals.modem);
    //wifi.scan().unwrap();
    wifi.connect("sakhmil", "qlsh7760").unwrap();

    // for i in 0..127 {
    //     let res = i2c_inst.read(i, &mut buffer, 10);
    //     match res {
    //         Ok(_) => println!("Device found at address {i}"),
    //         Err(e) => println!("not found at address {i}, i2c error {e:?}"),
    //     }
    // }

    let i2c_inst = get_i2c0_inst(i2c0, &mut sda, &mut scl);
    let bus = shared_bus::BusManagerSimple::new(i2c_inst);

    let mut bme280 = BME280Sensor::new("BME280", bus.acquire_i2c(), delay::Ets);
    bme280.init().unwrap();

    let mut gy30 = GY30Sensor::new("GY30", bus.acquire_i2c(), delay::Ets);

    let data_provider = DataProvider::new();
    let _http_server = httpd(data_provider.clone());

    loop {
        bme280.measure_cmd();
        bme280.read();
        let bme280_data = bme280.get_data();
        print_sensor_data(bme280_data);

        gy30.measure_cmd();
        gy30.read();
        let gy30_data = gy30.get_data();
        print_sensor_data(gy30_data);

        data_provider.lock().unwrap().push_data(bme280.get_name(), bme280_data);
        data_provider.lock().unwrap().push_data(gy30.get_name(), gy30_data);

        std::thread::sleep(Duration::from_secs(1));
    }
}

fn print_sensor_data(sensor_data: &SensorData) {
    println!("{} values:", sensor_data.get_name());
    let gy30_values = sensor_data.get_values();
    for val_ref in gy30_values {
        let value_name = &val_ref.value_name;
        let value = val_ref.value;
        let unit = &val_ref.unit;
        println!("  {value_name}: {value} {unit}");
    }
}

fn get_i2c0_inst<'a>(
    i2c0: &'a mut i2c::I2C0,
    sda: &'a mut esp_idf_hal::gpio::Gpio21,
    scl: &'a mut esp_idf_hal::gpio::Gpio22,
) -> i2c::I2cDriver<'a> {
    let config = i2c::config::Config::new().baudrate(400.kHz().into());
    let i2c_res = i2c::I2cDriver::new(i2c0, sda, scl, &config);
    let i2c_inst = match i2c_res {
        Ok(i2c) => i2c,
        Err(e) => panic!("Error in i2c initialization {:?}", e),
    };

    i2c_inst
}
