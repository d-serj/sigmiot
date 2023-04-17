
use std::cell::RefCell;
use std::sync::{Mutex, Arc};

use anyhow::Error;

use embassy_sync::blocking_mutex;
use embedded_svc::http::server::Method;
use embedded_svc::io::Write;

use esp_idf_svc::http::server::{fn_handler, EspHttpServer, ws::EspHttpWsProcessor};
use embedded_svc::ws::asynch::server::Acceptor;
use esp_idf_hal::task::embassy_sync::EspRawMutex;

use crate::data_channel::get_http_data;
use crate::ws::{self, *};

pub struct Config {
    ws_max_con: usize,
    ws_max_frame_size: usize,
}

const CONFIG: Config = Config {
    ws_max_con: 2,
    ws_max_frame_size: 4096,
};

pub fn httpd() -> Result<(EspHttpServer, impl Acceptor), Error> {

    let (ws_processor, ws_acceptor) =
        EspHttpWsProcessor::<{ CONFIG.ws_max_con}, { CONFIG.ws_max_frame_size }>::new(());

    let ws_processor = blocking_mutex::Mutex::<EspRawMutex, _>::new(RefCell::new(ws_processor));

    let mut server =  EspHttpServer::new(&Default::default())?;

    server
        .fn_handler("/sensors", Method::Get, move|req| {
            let raw_data = get_http_data();
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
