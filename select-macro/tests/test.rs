use select_macro::{count, select, select_variant};
use tokio::sync::mpsc::channel;
use tokio::time::{interval, sleep};

use async_std::{
    channel as async_channel,
    stream::{self, StreamExt},
    task,
};
use std::time::Duration;

#[tokio::test]
async fn test_tokio_select() {
    let mut inter = interval(Duration::from_secs(2));
    inter.tick().await;
    let (rx, mut tx) = channel(1);
    tokio::spawn(async move {
        sleep(Duration::from_secs(1)).await;
        rx.send(1).await.unwrap();
    });
    select! {
        _ = inter.tick() => {
            panic!("unreachable!");
        }
        data = tx.recv() => {
            assert_eq!(data, Some(1));
        }
    };
}

#[async_std::test]
async fn test_asyncstd_select() {
    let mut inter = stream::interval(Duration::from_secs(2));
    let (rx, tx) = async_channel::bounded(1);
    task::spawn(async move {
        task::sleep(Duration::from_secs(1)).await;
        rx.send(1).await.unwrap();
    });
    select! {
        _ = inter.next() => {
            panic!("unreachable!");
        }
        data = tx.recv() => {
            assert_eq!(data, Ok(1));
        }
    };
}
