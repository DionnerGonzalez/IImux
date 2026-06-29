// tui/spinner.rs — a minimal spinner that doesn't fight with stdout

use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct Spinner {
    running: Arc<Mutex<bool>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Spinner {
    pub fn start(label: String) -> Self {
        let running = Arc::new(Mutex::new(true));
        let running_clone = running.clone();

        let handle = thread::spawn(move || {
            let mut i = 0;
            while *running_clone.lock().unwrap() {
                print!("\r  {} {}", FRAMES[i % FRAMES.len()], label);
                let _ = io::stdout().flush();
                thread::sleep(Duration::from_millis(80));
                i += 1;
            }
            // Clear the line
            print!("\r{}\r", " ".repeat(label.len() + 8));
            let _ = io::stdout().flush();
        });

        Self {
            running,
            handle: Some(handle),
        }
    }

    pub fn stop(&mut self) {
        *self.running.lock().unwrap() = false;
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop();
    }
}
