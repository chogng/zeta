#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod portable;

#[cfg(target_os = "macos")]
pub(crate) use macos::system_family_names;
#[cfg(not(target_os = "macos"))]
pub(crate) use portable::system_family_names;
