use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";

const LABEL_WIDTH: usize = 22;
const SPINNER_FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

fn fmt_dur(d: Duration) -> String {
    let ms = d.as_secs_f64() * 1000.0;
    if ms < 1000.0 {
        format!("{ms:.0}ms")
    } else {
        format!("{:.2}s", ms / 1000.0)
    }
}

pub fn header(action: &str, details: &[&str]) {
    let details = details
        .iter()
        .filter(|d| !d.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(&format!(" {DIM}·{RESET} "));

    println!();
    if details.is_empty() {
        println!("  {BOLD}{GREEN}GOYDA{RESET} {action}");
    } else {
        println!("  {BOLD}{GREEN}GOYDA{RESET} {action} {DIM}·{RESET} {details}");
    }
    println!();
}

pub struct Step {
    label: &'static str,
    start: Instant,
}

pub fn step(label: &'static str) -> Step {
    Step {
        label,
        start: Instant::now(),
    }
}

impl Step {
    pub fn ok(self) {
        let elapsed = fmt_dur(self.start.elapsed());
        println!(
            "  {GREEN}✓{RESET} {label:<width$} {DIM}{elapsed}{RESET}",
            label = self.label,
            width = LABEL_WIDTH,
        );
    }
}

pub struct SpinnerStep {
    label: &'static str,
    start: Instant,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

pub fn spinner_step(label: &'static str) -> SpinnerStep {
    let start = Instant::now();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();

    let handle = thread::spawn(move || {
        let mut frame = 0usize;
        while !stop_clone.load(Ordering::Relaxed) {
            let elapsed = fmt_dur(start.elapsed());
            print!(
                "\r  {DIM}{}{RESET} {label:<width$} {DIM}{elapsed}{RESET}",
                SPINNER_FRAMES[frame % SPINNER_FRAMES.len()],
                label = label,
                width = LABEL_WIDTH,
            );
            let _ = std::io::stdout().flush();
            frame += 1;
            thread::sleep(Duration::from_millis(80));
        }
    });

    SpinnerStep {
        label,
        start,
        stop,
        handle: Some(handle),
    }
}

impl SpinnerStep {
    fn clear_line(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        print!("\r{}\r", " ".repeat(LABEL_WIDTH + 20));
        let _ = std::io::stdout().flush();
    }

    pub fn ok(mut self) {
        self.clear_line();
        let elapsed = fmt_dur(self.start.elapsed());
        println!(
            "  {GREEN}✓{RESET} {label:<width$} {DIM}{elapsed}{RESET}",
            label = self.label,
            width = LABEL_WIDTH,
        );
    }

    pub fn fail(mut self, reason: &str) {
        self.clear_line();
        println!("  {RED}✗{RESET} {}", self.label);
        println!("    {DIM}{reason}{RESET}");
    }
}

impl Drop for SpinnerStep {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

pub fn info(msg: &str) {
    println!("  {DIM}→{RESET} {msg}");
}

pub fn ready(msg: &str, elapsed: Duration) {
    println!();
    println!(
        "  {GREEN}ready{RESET} in {BOLD}{}{RESET} {DIM}{RESET} {msg}",
        fmt_dur(elapsed)
    );
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_dur_uses_whole_milliseconds_under_one_second() {
        assert_eq!(fmt_dur(Duration::from_millis(0)), "0ms");
        assert_eq!(fmt_dur(Duration::from_millis(1)), "1ms");
        assert_eq!(fmt_dur(Duration::from_millis(999)), "999ms");
    }

    #[test]
    fn fmt_dur_switches_to_seconds_with_two_decimals_at_one_second() {
        assert_eq!(fmt_dur(Duration::from_millis(1000)), "1.00s");
        assert_eq!(fmt_dur(Duration::from_millis(1500)), "1.50s");
        assert_eq!(fmt_dur(Duration::from_secs(73)), "73.00s");
    }
}
