
use std::net::Ipv4Addr;
use std::time::Duration;

use embedded_svc::wifi::*;

use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::netif::{EspNetifWait, EspNetif};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::{wifi::*};
use esp_idf_sys::EspError;
use esp_idf_hal::modem::Modem;

pub struct Wifi<'a> {
    wifi_inst: Box<EspWifi<'a>>,
    sys_loop: EspSystemEventLoop,
}

impl<'a> Wifi<'a> {
    pub fn new(modem: Modem) -> Self {
        //let periph = peripherals::Peripherals::take().unwrap();

        let sys_loop = EspSystemEventLoop::take().unwrap();

        let default_nvs = EspDefaultNvsPartition::take().unwrap();

        let wifi = Box::new(EspWifi::new(modem, sys_loop.clone(), Some(default_nvs)).unwrap());

        Wifi { wifi_inst: wifi, sys_loop }
    }

    pub fn connect(&mut self, ssid: &'static str, psk: &'static str) -> Result<(), EspError> {
        let ap_infos = self.wifi_inst.scan()?;
        let ours = ap_infos.into_iter().find(|a| a.ssid == ssid);
        let channel = if let Some(ours) = ours {
            println!("found configured access point {} on channel {}", ssid, ours.channel);
            Some(ours.channel)
        } else {
            panic!("Configured access point was not found!");
        };

        self.wifi_inst.set_configuration(&Configuration::Client(ClientConfiguration {
            ssid: ssid.into(),
            bssid: None,
            auth_method: Default::default(),
            password: psk.into(),
            channel
        }))?;

        let config = self.wifi_inst.get_configuration().unwrap();

        println!("WiFi config {:?}", config);

        self.wifi_inst.start()?;

        self.wifi_inst.connect()?;

        println!("WiFi connected!");


        if !EspNetifWait::new::<EspNetif>(self.wifi_inst.sta_netif(), &self.sys_loop)?.wait_with_timeout(
            Duration::from_secs(30),
            || {
                self.wifi_inst.is_up().unwrap() && self.wifi_inst.sta_netif().get_ip_info().unwrap().ip != Ipv4Addr::new(0, 0, 0, 0)
            }) {
                panic!("Wifi did not connect or did not receive a DHCP lease");
            };

        let ip_info = self.wifi_inst.sta_netif().get_ip_info()?;
        println!("Wifi DHCP info: {:?}", ip_info);

        Ok(())
    }

    #[allow(dead_code)]
    pub fn scan(&mut self) -> Result<Vec<String>, EspError> {
        let access_points = self.wifi_inst.driver_mut().scan()?;

        let mut ssids = Vec::new();

        for points in access_points.iter() {
            println!("SSID: {}, channel {}, signal strength {}",
                points.ssid, points.channel, points.signal_strength);
            ssids.push(points.ssid.to_string().clone());
        }

        Ok(ssids)
    }
}
