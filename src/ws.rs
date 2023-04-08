
use std::cell::Cell;

use log::info;
use serde::{Deserialize, Serialize};

use embedded_svc::ws::asynch::server::Acceptor;
use embedded_svc::ws::FrameType;

use embassy_sync::blocking_mutex::raw::{NoopRawMutex, RawMutex};
use embassy_sync::mutex::Mutex as AsyncMutex;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebRequest {
    Request,
    RequestWithPayload(u32),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebEvent {
    Event,
    EventWithPayload(u32),
    MalformedRequest,
}

pub async fn ws_conn_handler<A: Acceptor>(acceptor: A) {
    loop {
        info!("[WS HANDLER] Wait for connection...");
        let (sender, mut receiver) = acceptor.accept().await.unwrap();
        info!("[WS HANDLER] ..got connection");
        let sender = AsyncMutex::<NoopRawMutex, _>::new(sender);

        let count_frames = AsyncMutex::<NoopRawMutex, _>::new(Cell::new(0_u32));

        let mut open = true;
        loop {
            if !open {
                break;
            }

            info!("[WS HANDLER] Wait for Frame..");

            open = receive(&mut receiver, &sender, &count_frames)
                .await
                .unwrap();

            info!("[WS HANDLER] ..finished receiving frame");
        }
    }
}

pub async fn receive(
    mut receiver: impl embedded_svc::ws::asynch::Receiver,
    sender: &AsyncMutex<impl RawMutex, impl embedded_svc::ws::asynch::Sender>,
    counter: &AsyncMutex<impl RawMutex, Cell<u32>>,
) -> Result<bool, ()> {
    let mut recv_buffer: [u8; 4096] = [0; 4096];
    let (frame_type, mut size) = receiver.recv(&mut recv_buffer).await.unwrap();
    size = if size > 0 { size } else { 1 };
    let debug = core::str::from_utf8(&recv_buffer[..size - 1]).unwrap();

    info!("[WS RECEIVE] msg: {}", debug);

    let count = counter.lock().await;
    count.set(count.get() + 1);
    info!("[WS RECEIVE] Frame number: {:?}", count.get());
    info!(
        "[WS RECEIVE] Frame {:?} Type: {:?}",
        count.get(),
        frame_type);

    let hold_open = match frame_type {
        FrameType::Text(_) => {
            let mut response = String::from_utf8(recv_buffer[..size - 1].to_vec())
                .unwrap();
            response.push_str(" - echo");
            let msg_slice = response.as_bytes();
            info!("[WS RECEIVE] Frame {:?} Try Sending echo..", count.get());
            let mut sender_lock = sender.lock().await;
            sender_lock
                .send(FrameType::Text(false), msg_slice)
                .await
                .unwrap();
            info!("[WS RECEIVE] Frame {:?} ..Send", count.get());

            true
        }
        FrameType::Binary(_) => true,
        FrameType::Continue(_) => true,
        FrameType::Ping => true,
        FrameType::Pong => true,
        FrameType::Close => false,
        FrameType::SocketClose => false,
    };

    Ok(hold_open)
}
