mod data_channel;
mod httpd;
mod sensors;
mod sigmiot_log;
mod spawn;
mod wifi;
mod ws;

use esp_idf_hal::task::executor::EspExecutor;
use esp_idf_sys::{self as _};
use httpd::httpd;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::time::Duration;

use esp_idf_hal::delay;
use esp_idf_hal::i2c::{self};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::units::FromValueType;

use sensors::{BME280Sensor, GY30Sensor, Sensor};
use wifi::Wifi;

use crate::sigmiot_log::sigmiot_log_init;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    sigmiot_log_init();

    // Bind the log crate to the ESP Logging facilities
    // log::set_logger(&LOGGER)
    //     .map(|()| {
    //         LOGGER.initialize();
    //         log::set_max_level(log::LevelFilter::Debug)
    //     })
    //     .expect("Configure and set logger with log level");

    let peripherals = Peripherals::take().unwrap();

    let scl = peripherals.pins.gpio22;
    let sda = peripherals.pins.gpio21;

    let i2c0 = peripherals.i2c0;

    let mut wifi = Wifi::new(peripherals.modem);

    wifi.connect("sakhmil", "qlsh7760").unwrap();

    // for i in 0..127 {
    //     let res = i2c_inst.read(i, &mut buffer, 10);
    //     match res {
    //         Ok(_) => println!("Device found at address {i}"),
    //         Err(e) => println!("not found at address {i}, i2c error {e:?}"),
    //     }
    // }

    let config = i2c::config::Config::new().baudrate(400.kHz().into());
    let i2c_inst = i2c::I2cDriver::new(i2c0, sda, scl, &config).unwrap();

    let bus: &'static _ = shared_bus::new_std!(i2c::I2cDriver = i2c_inst).unwrap();

    let mut bme280 = Box::new(BME280Sensor::new(
        "BME280",
        "room1",
        bus.acquire_i2c(),
        delay::Ets,
    ));
    bme280.init().unwrap();

    let gy30 = Box::new(GY30Sensor::new(
        "GY30",
        "room1",
        bus.acquire_i2c(),
        delay::Ets,
    ));

    let (_http, ws_acceptor) = httpd().unwrap();

    let mut tasks_high_prio = heapless::Vec::<_, 16>::new();
    let mut executor_high_prio = EspExecutor::<16, _>::new();

    let mut sensor_manager = sensors::SensorManager::new(1000);
    sensor_manager.add_sensor(bme280);
    sensor_manager.add_sensor(gy30);

    spawn::collect_high_prio(
        &mut executor_high_prio,
        &mut tasks_high_prio,
        ws_acceptor,
        sensor_manager,
    )
    .unwrap();

    spawn::run(&mut executor_high_prio, tasks_high_prio);

    unreachable!("This should never be reached");
}
