use std::cell::Cell;
use std::env;
use std::io::{stdin, stdout, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use tui::backend::Backend;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, List, ListItem, Paragraph};
use tui::{Frame, Terminal};

use log::{debug, error, info};
use simplelog::{Config, LevelFilter, WriteLogger};
use std::fs::File;

use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;

use protobuf::{EnumOrUnknown, Message};

include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));

use sigmiot_data::{message_response, MessageResponse};

#[derive(Debug, Clone)]
pub struct SensorValue {
    pub value_name: String,
    pub value: f32,
    pub unit: String,
}

#[derive(Debug, Clone)]
pub struct SensorData {
    sensor_name: String,
    sensor_type: String,
    sensor_location: String,
    sensor_values: Vec<SensorValue>,
}

#[derive(Debug, Clone)]
struct Esp32LogEntry {
    pub log_message: String,
    pub log_timestamp: u64,
    pub log_level: String,
}

#[derive(Debug)]
enum ChannelMessage {
    LogsEsp32(Vec<Esp32LogEntry>),
    SensorsData(Vec<SensorData>),
    Exit,
}

struct App {
    logs: Vec<Esp32LogEntry>,
    sensors_data: Vec<SensorData>,
}

const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");

impl App {
    fn new() -> App {
        App {
            logs: vec![],
            sensors_data: vec![],
        }
    }

    fn add_log(&mut self, log: &mut Vec<Esp32LogEntry>) {
        self.logs.append(log);
    }

    fn add_sensors_data(&mut self, data: Vec<SensorData>) {
        self.sensors_data = data;
    }
}

#[tokio::main]
async fn main() {
    let log_file_name = format!("{}.log", CARGO_PKG_NAME);
    WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        File::create(log_file_name).unwrap(),
    )
    .unwrap();

    info!("Starting {}...", CARGO_PKG_NAME);

    let connect_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| panic!("This program requires at least one argument"));
    let url = url::Url::parse(&connect_addr).expect("Cannot parse URL");
    info!("Connecting to {}...", url);
    let (mut ws, _) = connect_async(&url).await.expect("Failed to connect");
    info!("Connected to {}", url);

    // Create a new channel with a capacity of at most 32.
    let (tx, rx) = mpsc::channel::<ChannelMessage>(32);

    let app = App::new();
    let ui_task_join = tokio::spawn(ui_task(rx, app));

    let count_frames = Arc::new(Mutex::new(Cell::new(0_u32)));

    loop {
        tokio::select! {
            msg = ws.next() => {
                match msg {
                    Some(msg) => {
                        let msg = msg.unwrap();
                        if msg.is_binary() {
                            let data = msg.into_data();
                            if let Err(e) = handle_binary_message(data, &tx).await {
                                error!("Error handling binary message: {:?}", e);
                                break;
                            }
                            else {
                                info!(
                                    "Frame received. Frames count: {}",
                                    count_frames.lock().unwrap().get()
                                );

                                let val = count_frames.lock().unwrap().get();
                                count_frames.lock().unwrap().set(val + 1);
                            }
                        }
                    }

                    None => break,
                }

            }

            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl-C received, exiting...");
                tx.send(ChannelMessage::Exit).await.unwrap();
                ui_task_join.await.unwrap();
                break;
            }
        }
    }

    info!("Closing connection...");
    ws.close(None).await.unwrap();
    info!("Connection closed");
}

async fn handle_binary_message(data: Vec<u8>, tx: &mpsc::Sender<ChannelMessage>) -> Result<(), ()> {
    let message_resp = match MessageResponse::parse_from_bytes(&data) {
        Ok(msg) => msg,
        Err(e) => {
            error!("Error parsing MessageResponse: {:?}", e);
            return Err(());
        }
    };

    debug!("MessageResponse: {:?}", message_resp);

    if message_resp.status == EnumOrUnknown::new(message_response::Status::OK) {
        let sensors_data = &message_resp.sensors_data_response;
        let channel_msg: Vec<SensorData> = sensors_data
            .iter()
            .map(|sensor| SensorData {
                sensor_name: sensor.sensor_name.clone(),
                sensor_type: sensor.sensor_type.clone(),
                sensor_location: sensor.sensor_location.clone(),
                sensor_values: sensor
                    .sensor_values
                    .iter()
                    .map(|value| SensorValue {
                        value_name: value.value_name.clone(),
                        value: value.value_data,
                        unit: value.value_unit.clone(),
                    })
                    .collect(),
            })
            .collect();

        tx.send(ChannelMessage::SensorsData(channel_msg))
            .await
            .unwrap();
    } else {
        error!("Error: {:?}", message_resp.status);
    }

    let logs = message_resp.log_data_response;

    let channel_msg: Vec<Esp32LogEntry> = logs
        .iter()
        .map(|log| Esp32LogEntry {
            log_message: log.log_message.clone(),
            log_timestamp: log.log_timestamp,
            log_level: log.log_level.clone(),
        })
        .collect();

    tx.send(ChannelMessage::LogsEsp32(channel_msg))
        .await
        .unwrap();

    Ok(())
}

async fn ui_task(mut rx: mpsc::Receiver<ChannelMessage>, mut app: App) {
    // termion raw mode
    //let stdout = stdout().into_raw_mode().unwrap();
    let stdout = stdout();
    let backend = TermionBackend::new(stdout);

    let mut terminal = Terminal::new(backend).unwrap();

    terminal.hide_cursor().unwrap();
    terminal.clear().unwrap();

    'ui_loop: loop {
        let msg = rx.recv().await.unwrap();

        match msg {
            ChannelMessage::LogsEsp32(log) => {
                // Rotate logs
                // 28 is the number of lines in the log window
                while app.logs.len() > 28 {
                    app.logs.remove(0);
                }

                debug!("Logs len {}", app.logs.len());

                app.add_log(log.clone().as_mut());
            }
            ChannelMessage::SensorsData(data) => {
                app.add_sensors_data(data);
            }
            ChannelMessage::Exit => {
                info!("Exit received, exiting...");
                break 'ui_loop;
            }
        }

        terminal.draw(|f| ui(f, &mut app)).unwrap();

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    terminal.clear().unwrap();
    terminal.show_cursor().unwrap();
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let size = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(60),
            ]
            .as_ref(),
        )
        .split(size);

    let mut chunk = 0;

    for sensor in &app.sensors_data {
        let mut sensor_data_str = String::new();

        for values in &sensor.sensor_values {
            sensor_data_str.push_str(
                format!("{}: {} {}\n", values.value_name, values.value, values.unit).as_str(),
            );
        }

        let title = format!(" Sensor {} ", sensor.sensor_name);
        let paragraph = Paragraph::new(sensor_data_str)
            .block(Block::default().title(title).borders(Borders::ALL));

        f.render_widget(paragraph, chunks[chunk]);

        chunk += 1;
    }

    let logs_with_date = logs_to_tui_list_item(app);

    let logs = List::new(logs_with_date)
        .block(Block::default().title(" ESP32 Logs ").borders(Borders::ALL))
        .highlight_style(Style::default());

    f.render_widget(logs, chunks[chunk]);
}

fn logs_to_tui_list_item(app: &App) -> Vec<ListItem> {
    app.logs
        .iter()
        .map(|log| {
            let sty = match log.log_level.as_str() {
                "ERROR" => Style::default().fg(Color::Red),
                "WARN" => Style::default().fg(Color::Yellow),
                "INFO" => Style::default().fg(Color::Blue),
                _ => Style::default(),
            };

            let timestamp_str = format!("{:<9}", log.log_timestamp);

            let log = Spans::from(vec![
                Span::styled(
                    timestamp_str,
                    Style::default().add_modifier(Modifier::ITALIC),
                ),
                Span::raw(" "),
                Span::styled(format!("{:<9}", log.log_level), sty),
                Span::raw(log.log_message.clone()),
            ]);

            ListItem::new(vec![log])
        })
        .collect()
}
