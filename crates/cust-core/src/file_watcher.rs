use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileWatcherEvent {
    pub paths: Vec<PathBuf>,
}

pub struct DebouncedWatchReceiver {
    rx: mpsc::UnboundedReceiver<PathBuf>,
    debounce_duration: Duration,
}

impl DebouncedWatchReceiver {
    pub fn new(rx: mpsc::UnboundedReceiver<PathBuf>, debounce_duration: Duration) -> Self {
        Self {
            rx,
            debounce_duration,
        }
    }

    pub async fn recv(&mut self) -> Option<FileWatcherEvent> {
        let first_path = self.rx.recv().await?;
        let mut paths = BTreeSet::new();
        paths.insert(first_path);

        let deadline = Instant::now() + self.debounce_duration;

        loop {
            tokio::select! {
                maybe_path = self.rx.recv() => {
                    match maybe_path {
                        Some(p) => { paths.insert(p); },
                        None => break,
                    }
                }
                _ = tokio::time::sleep_until(deadline) => {
                    break;
                }
            }
        }

        Some(FileWatcherEvent {
            paths: paths.into_iter().collect(),
        })
    }
}

pub struct ThrottledWatchReceiver {
    rx: mpsc::UnboundedReceiver<PathBuf>,
    throttle_duration: Duration,
    last_emitted: Option<Instant>,
}

impl ThrottledWatchReceiver {
    pub fn new(rx: mpsc::UnboundedReceiver<PathBuf>, throttle_duration: Duration) -> Self {
        Self {
            rx,
            throttle_duration,
            last_emitted: None,
        }
    }

    pub async fn recv(&mut self) -> Option<FileWatcherEvent> {
        if let Some(last) = self.last_emitted {
            let next_allowed = last + self.throttle_duration;
            if Instant::now() < next_allowed {
                tokio::time::sleep_until(next_allowed).await;
            }
        }

        let path = self.rx.recv().await?;
        self.last_emitted = Some(Instant::now());

        Some(FileWatcherEvent { paths: vec![path] })
    }
}
