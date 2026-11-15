#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
#[path = "../watchdog_core.rs"]
mod watchdog_core;

#[cfg(windows)]
fn main() {
    use std::ffi::OsString;
    use std::path::PathBuf;

    let mut args = std::env::args_os().skip(1);
    let Some(command) = args.next() else {
        std::process::exit(2)
    };
    let Some(pid) = args
        .next()
        .and_then(|value| value.to_str().and_then(|text| text.parse::<u32>().ok()))
    else {
        std::process::exit(2)
    };
    let Some(report) = args.next().map(PathBuf::from) else {
        std::process::exit(2)
    };
    if command != OsString::from("--watchdog") || args.next().is_some() {
        std::process::exit(2);
    }
    watchdog_core::run(pid, &report);
}

#[cfg(not(windows))]
fn main() {}
