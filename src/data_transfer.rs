
use std::cell::RefCell;
use std::sync::{Mutex, Arc};

use anyhow::Error;

use embassy_sync::blocking_mutex;
use embedded_svc::http::server::Method;
use embedded_svc::io::Write;

use esp_idf_svc::http::server::{fn_handler, EspHttpServer, ws::EspHttpWsProcessor};
use embedded_svc::ws::asynch::server::Acceptor;
use esp_idf_hal::task::embassy_sync::EspRawMutex;

use crate::data_provider::DataProvider;
use crate::ws::{self, *};

pub trait DataTransfer {
    fn init(&mut self) -> Result<(), Error>;
    fn send_data(&self, data: &DataProvider) -> Result<(), Error>;
}

pub struct HttpServer {

}

impl HttpServer {
    fn new() -> Self {
        Self { }
    }

   // fn post()
}

impl DataTransfer for HttpServer {
    fn init(&mut self) -> Result<(), Error> {
        todo!()
    }

    fn send_data(&self, data: &DataProvider) -> Result<(), Error> {
        todo!()
    }
}

pub struct Config {
    wifi_ssid: &'static str,
    wifi_psk: &'static str,
    ws_max_con: usize,
    ws_max_frame_size: usize,
}

const CONFIG: Config = Config {
    wifi_ssid: "ENTER_YOUR_SSID",
    wifi_psk: "ENTER_YOUR_PW",
    ws_max_con: 2,
    ws_max_frame_size: 4096,
};

pub fn httpd(
    data: Arc<Mutex<DataProvider>>,
) -> Result<(EspHttpServer, impl Acceptor), Error> {

    let (ws_processor, ws_acceptor) =
        EspHttpWsProcessor::<{ CONFIG.ws_max_con}, { CONFIG.ws_max_frame_size }>::new(());

    let ws_processor = blocking_mutex::Mutex::<EspRawMutex, _>::new(RefCell::new(ws_processor));

    let mut server =  EspHttpServer::new(&Default::default())?;

    server
        .fn_handler("/sensors", Method::Get, move|req| {
            let clone = data.clone();
            let raw_data = clone.lock().unwrap().get_http_data();
            req.into_ok_response()?
                .write_all(raw_data.as_bytes())?;

            Ok(())
        })?
        .fn_handler("/", Method::Get, move|req| {
            let response =
                r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Auto-Update Example</title>
                </head>
                <body>
                    <h1>Sensor Data</h1>
                    <div id="sensor-data"></div>
                    <script>
                        function updateSensorData() {{
                            var xhr = new XMLHttpRequest();
                            xhr.onreadystatechange = function() {{
                                if (this.readyState == 4 && this.status == 200) {{
                                    var sensorDataDiv = document.getElementById("sensor-data");
                                    sensorDataDiv.innerHTML = this.responseText;
                                }}
                            }};
                            xhr.open("GET", "/sensors", true);
                            xhr.send();
                        }}
                        setInterval(updateSensorData, 1000);
                    </script>
                </body>
                </html>
                "#.to_string();
            req.into_ok_response()?
                .write_all(response.as_bytes())?;

            println!("Client connected");

            Ok(())
        })?;

    server.ws_handler("/ws", move |connection| {
        ws_processor.lock(|ws_processor| ws_processor.borrow_mut().process(connection))
    })?;

    Ok((server, ws_acceptor))
}
