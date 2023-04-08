mod data_provider;
mod data_transfer;
mod sensors;
mod wifi;
mod ws;
mod spawn;

use data_transfer::httpd;
use esp_idf_hal::task::embassy_sync::EspRawMutex;
use esp_idf_hal::task::executor::EspExecutor;
use esp_idf_hal::task::thread::ThreadSpawnConfiguration;
use esp_idf_svc::log::EspLogger;
use esp_idf_sys::{self as _, EspError};
use embassy_sync::mutex::Mutex;
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::time::Duration;

use esp_idf_hal::delay;
use esp_idf_hal::i2c::{self};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::units::FromValueType;

use sensors::{BME280Sensor, GY30Sensor, Sensor, SensorData};
use wifi::Wifi;
pub use crate::data_provider::DataProvider;

static LOGGER: EspLogger = EspLogger;

const DATA_CHANNEL_SIZE: usize = 8;
static CHANNEL: embassy_sync::channel::Channel::<EspRawMutex, u32, DATA_CHANNEL_SIZE> = embassy_sync::channel::Channel::new();

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    // esp_idf_svc::log::EspLogger::initialize_default();
    log::set_logger(&LOGGER)
        .map(|()| {
            LOGGER.initialize();
            log::set_max_level(log::LevelFilter::Debug)
        })
        .expect("Configure and set logger with log level");

    let peripherals = Peripherals::take().unwrap();

    let scl = peripherals.pins.gpio22;
    let sda = peripherals.pins.gpio21;

    let i2c0 = peripherals.i2c0;

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

    //let i2c_inst = get_i2c0_inst(i2c0, &mut sda, &mut scl);

    let config = i2c::config::Config::new().baudrate(400.kHz().into());
    let i2c_inst = i2c::I2cDriver::new(i2c0, sda, scl, &config).unwrap();
    //let bus = shared_bus::BusManagerSimple::new(i2c_inst);
    let bus: &'static _ = shared_bus::new_std!(i2c::I2cDriver = i2c_inst).unwrap();

    let mut bme280 =
        Box::new(BME280Sensor::new("BME280", bus.acquire_i2c(), delay::Ets));
    bme280.init().unwrap();

    let gy30 =
        Box::new(GY30Sensor::new("GY30", bus.acquire_i2c(), delay::Ets));

    let data_provider = DataProvider::new();
    let (_http, ws_acceptor) = httpd(data_provider.clone()).unwrap();

    let mut tasks_high_prio = heapless::Vec::<_, 16>::new();
    let mut executor_high_prio = EspExecutor::<16, _>::new();

    let mut sensor_manager = sensors::SensorManager::new();
    sensor_manager.add_sensor(bme280);
    sensor_manager.add_sensor(gy30);

    spawn::hight_prio(&mut executor_high_prio, &mut tasks_high_prio, ws_acceptor, sensor_manager, CHANNEL.sender(), CHANNEL.receiver()).unwrap();

    spawn::run(&mut executor_high_prio, tasks_high_prio);

    loop {
        std::thread::sleep(Duration::from_secs(1));
    }

    unreachable!("This should never be reached");

    // ThreadSpawnConfiguration {
    //     name: Some(b"async-exec-mid\0"),
    //     ..Default::default()
    // }
    // .set()
    // .unwrap();

    // let mut tasks_low_prio = heapless::Vec::<_, 2>::new();
    // let mut executor_low_prio = EspExecutor::<2, _>::new();



    //spawn::data_collect(&mut executor_low_prio, &mut tasks_low_prio, sensor_manager).unwrap();


    // loop {
    //     bme280.measure_cmd();
    //     bme280.read();
    //     let bme280_data = bme280.get_data();
    //     print_sensor_data(bme280_data);

    //     gy30.measure_cmd();
    //     gy30.read();
    //     let gy30_data = gy30.get_data();
    //     print_sensor_data(gy30_data);

    //     data_provider.lock().unwrap().push_data(bme280.get_name(), bme280_data);
    //     data_provider.lock().unwrap().push_data(gy30.get_name(), gy30_data);

    //     std::thread::sleep(Duration::from_secs(1));
    // }
}
