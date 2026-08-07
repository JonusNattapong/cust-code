use cust_core::{DebouncedWatchReceiver, ThrottledWatchReceiver};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_debounced_watch_receiver() {
    let (tx, rx) = mpsc::unbounded_channel();
    let mut debounced = DebouncedWatchReceiver::new(rx, Duration::from_millis(50));

    tx.send(PathBuf::from("file1.rs")).unwrap();
    tx.send(PathBuf::from("file2.rs")).unwrap();
    tx.send(PathBuf::from("file1.rs")).unwrap(); // Duplicate

    let event = debounced.recv().await.unwrap();
    assert_eq!(event.paths.len(), 2);
    assert_eq!(event.paths[0], PathBuf::from("file1.rs"));
    assert_eq!(event.paths[1], PathBuf::from("file2.rs"));
}

#[tokio::test]
async fn test_throttled_watch_receiver() {
    let (tx, rx) = mpsc::unbounded_channel();
    let mut throttled = ThrottledWatchReceiver::new(rx, Duration::from_millis(50));

    tx.send(PathBuf::from("file1.rs")).unwrap();
    let event = throttled.recv().await.unwrap();
    assert_eq!(event.paths, vec![PathBuf::from("file1.rs")]);
}
