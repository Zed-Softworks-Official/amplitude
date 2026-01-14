#[cfg(target_os = "linux")]
pub mod pipewire;

#[cfg(target_os = "macos")]
pub mod coreaudio;
