//! Keyboard/mouse idle detection. Windows-only for M4; Mac/Linux fall back
//! to "always 0" (active) so the rest of the state machine keeps working.

/// Seconds since last user input event (any keyboard or mouse).
pub fn idle_seconds() -> u32 {
    #[cfg(windows)]
    {
        windows_impl()
    }
    #[cfg(not(windows))]
    {
        0
    }
}

#[cfg(windows)]
fn windows_impl() -> u32 {
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let mut info = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    let ok = unsafe { GetLastInputInfo(&mut info as *mut _) };
    if ok == 0 {
        return 0;
    }
    let now = unsafe { GetTickCount() };
    now.saturating_sub(info.dwTime) / 1000
}
