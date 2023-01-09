use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use std::time::Duration;

use esp_idf_hal::peripherals;
use esp_idf_hal::i2c;
use esp_idf_hal::units::FromValueType;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();
    
    // Connect BM280 through the I2C
    let periph = peripherals::Peripherals::take().unwrap();
    
    let scl = periph.pins.gpio22.into_output().unwrap();
    let sda = periph.pins.gpio21.into_input_otput().unwrap();
    
    let _i2c = i2c::Master::new(
        periph.i2c0,
        i2c::MasterPins { sda, scl }, 
        i2c::config::MasterConfig::new().baudrate(400.kHz().into())
    ).unwrap();
    
    loop {
        println!("Toggle!");
        
        std::thread::sleep(Duration::from_millis(500));
    }
    
}
