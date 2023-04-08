

use esp_idf_hal::task::embassy_sync::EspRawMutex;
use esp_idf_hal::task::executor::{Executor, EspExecutor};
use embedded_svc::ws::asynch::server::Acceptor;
use esp_idf_hal::task::executor::{Task, Local, Monitor, SpawnError, Wait};
use embassy_sync::channel::{Sender, Receiver};

use crate::{sensors, DATA_CHANNEL_SIZE};
use crate::ws;

pub fn hight_prio<'a, const C: usize, M>(
    executor: &mut Executor<'a, C, M, Local>,
    tasks: &mut heapless::Vec<Task<()>, C>,
    ws_acceptor: impl Acceptor + 'a,
    sensor_manager: sensors::SensorManager,
    data_sender: Sender<'static, EspRawMutex, u32, DATA_CHANNEL_SIZE>,
    data_receiver: Receiver<'static, EspRawMutex, u32, DATA_CHANNEL_SIZE>,
) -> Result<(), SpawnError>
where
    M: Monitor + Default,
{
    executor.spawn_local_collect(ws::ws_conn_handler(ws_acceptor), tasks)?;
    executor.spawn_local_collect(sensors::run_sensor_manager(sensor_manager, data_sender), tasks)?;
    executor.spawn_collect(test_run(data_receiver), tasks)?;

    Ok(())
}

pub async fn test_run(
    data_receiver: Receiver<'static, EspRawMutex, u32, DATA_CHANNEL_SIZE>,
) {
    loop {
        let data = data_receiver.recv().await;
        println!("Data: {}", data);
    }
}

#[allow(dead_code)]
pub fn data_collect<'a, const C: usize, M>(
    executor: &mut Executor<'a, C, M, Local>,
    tasks: &mut heapless::Vec<Task<()>, C>,

) -> Result<(), SpawnError>
where
    M: Monitor + Default,
{


    Ok(())
}

pub fn run<'a, const C: usize, M>(
    executor: &mut Executor<'a, C, M, Local>,
    tasks: heapless::Vec<Task<()>, C>,
)
where
    M: Monitor + Wait + Default,
{
    executor.run_tasks(move || true, tasks);
}
