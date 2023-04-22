
use std::cell::Cell;

use log::{info, debug};

use embedded_svc::ws::asynch::server::Acceptor;
use embedded_svc::ws::FrameType;

use embassy_sync::blocking_mutex::raw::{NoopRawMutex, RawMutex};
use embassy_sync::mutex::Mutex as AsyncMutex;
use embassy_time::{Timer, Duration};
use embassy_futures::select::{select, Either};

use crate::data_channel::get_protobuf_data_async;
use crate::sigmiot_log::remote_logger_set_enable;

pub async fn ws_conn_handler<A: Acceptor>(acceptor: A) {
    loop {
        debug!("[WS HANDLER] Wait for connection...");
        let (sender, mut receiver) = acceptor.accept().await.unwrap();
        debug!("[WS HANDLER] ..got connection");
        let sender = AsyncMutex::<NoopRawMutex, _>::new(sender);

        let count_frames = AsyncMutex::<NoopRawMutex, _>::new(Cell::new(0_u32));

        remote_logger_set_enable(true);

        let mut open = true;
        loop {
            if !open {
                break;
            }

            open = process_connection(&mut receiver, &sender, &count_frames).await.unwrap();
        }

        remote_logger_set_enable(false);
        debug!("[WS HANDLER] Connection closed");
    }
}

async fn process_connection(
    receiver: impl embedded_svc::ws::asynch::Receiver,
    sender: &AsyncMutex<impl RawMutex, impl embedded_svc::ws::asynch::Sender>,
    counter: &AsyncMutex<impl RawMutex, Cell<u32>>,
) -> Result<bool, ()> {
    match select(
        receive(receiver, sender, counter),
        async {
            let out_bytes = get_protobuf_data_async().await;
            send(&sender, &counter, &out_bytes).await;
        },
    ).await {
        Either::First(open) => {
            return open;
        }
        Either::Second(_) => {
            return Ok(true);
        }
    };
}

async fn send(
    sender: &AsyncMutex<impl RawMutex, impl embedded_svc::ws::asynch::Sender>,
    count_frames: &AsyncMutex<impl RawMutex, Cell<u32>>,
    msg: &Vec<u8>,
) {
    let count = count_frames.lock().await;
    count.set(count.get() + 1);
    debug!("[WS SEND] Frame number: {:?}", count.get());

    let mut sender_lock = sender.lock().await;
    let sender_int = &mut sender_lock;

    sender_int
        .send(FrameType::Binary(false), msg)
        .await
        .unwrap();
}

#[allow(unused)]
pub async fn receive(
    mut receiver: impl embedded_svc::ws::asynch::Receiver,
    sender: &AsyncMutex<impl RawMutex, impl embedded_svc::ws::asynch::Sender>,
    counter: &AsyncMutex<impl RawMutex, Cell<u32>>,
) -> Result<bool, ()> {
    let mut recv_buffer: [u8; 4096] = [0; 4096];
    let (frame_type, mut size) = receiver.recv(&mut recv_buffer).await.unwrap();
    size = if size > 0 { size } else { 1 };
    let debug = core::str::from_utf8(&recv_buffer[..size - 1]).unwrap();

    debug!("[WS RECEIVE] msg: {}", debug);

    let count = counter.lock().await;
    count.set(count.get() + 1);
    debug!("[WS RECEIVE] Frame number: {:?}", count.get());
    debug!(
        "[WS RECEIVE] Frame {:?} Type: {:?}",
        count.get(),
        frame_type);

    let hold_open = match frame_type {
        FrameType::Text(_) => false, // We don't support text frames
        FrameType::Binary(_) => true, // TODO - handle binary frames
        FrameType::Continue(_) => true,
        FrameType::Ping => true,
        FrameType::Pong => true,
        FrameType::Close => false,
        FrameType::SocketClose => false,
    };

    debug!("[WS HANDLER] ..finished receiving frame");

    Ok(hold_open)
}
