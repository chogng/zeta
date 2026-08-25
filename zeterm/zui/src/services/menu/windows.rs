#![allow(unsafe_code)]

use super::SystemServiceError;

pub(super) fn attach(menu: &muda::Menu, hwnd: isize) -> Result<(), SystemServiceError> {
    unsafe { menu.init_for_hwnd(hwnd) }
        .map_err(|source| SystemServiceError::backend("native application menu", source))
}

pub(super) fn remove(menu: &muda::Menu, hwnd: isize) -> Result<(), SystemServiceError> {
    unsafe { menu.remove_for_hwnd(hwnd) }
        .map_err(|source| SystemServiceError::backend("native application menu", source))
}
