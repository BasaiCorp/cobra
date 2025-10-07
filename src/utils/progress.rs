use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;
use tokio::sync::Mutex;

/// High-performance progress tracker for parallel operations
pub struct ProgressTracker {
    multi: Arc<MultiProgress>,
    bars: Arc<Mutex<Vec<ProgressBar>>>,
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            multi: Arc::new(MultiProgress::new()),
            bars: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn add_download(&self, name: &str, size: u64) -> ProgressBar {
        let pb = self.multi.add(ProgressBar::new(size));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} {msg}")
                .unwrap()
                .progress_chars("█▓▒░"),
        );
        pb.set_message(name.to_string());
        self.bars.lock().await.push(pb.clone());
        pb
    }

    pub async fn add_spinner(&self, msg: &str) -> ProgressBar {
        let pb = self.multi.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        self.bars.lock().await.push(pb.clone());
        pb
    }

    pub async fn finish_all(&self) {
        let bars = self.bars.lock().await;
        for bar in bars.iter() {
            bar.finish_and_clear();
        }
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}
