mod ffi;
mod volume_step;

use std::{thread, time::Duration};

fn main() {
    if !ffi::accessibility_trusted(true) {
        eprintln!(
            "fine-volume: waiting for Accessibility access \
             (System Settings → Privacy & Security → Accessibility)"
        );
        while !ffi::accessibility_trusted(false) {
            thread::sleep(Duration::from_secs(2));
        }
    }

    volume_step::install();
    eprintln!("fine-volume: running");
    unsafe { ffi::CFRunLoopRun() };
}
