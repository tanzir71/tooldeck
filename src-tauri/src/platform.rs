use serde::Serialize;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(source_state_id: u32, event_type: u32) -> f64;
    fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> *const std::ffi::c_void;
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

#[cfg(target_os = "macos")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFArrayGetCount(array: *const std::ffi::c_void) -> isize;
    fn CFArrayGetValueAtIndex(
        array: *const std::ffi::c_void,
        index: isize,
    ) -> *const std::ffi::c_void;
    fn CFDictionaryGetValueIfPresent(
        dictionary: *const std::ffi::c_void,
        key: *const std::ffi::c_void,
        value: *mut *const std::ffi::c_void,
    ) -> u8;
    fn CFStringCreateWithCString(
        allocator: *const std::ffi::c_void,
        value: *const std::ffi::c_char,
        encoding: u32,
    ) -> *const std::ffi::c_void;
    fn CFStringGetCString(
        string: *const std::ffi::c_void,
        buffer: *mut std::ffi::c_char,
        buffer_size: isize,
        encoding: u32,
    ) -> u8;
    fn CFNumberGetValue(
        number: *const std::ffi::c_void,
        number_type: isize,
        value: *mut std::ffi::c_void,
    ) -> u8;
    fn CFRelease(value: *const std::ffi::c_void);
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityPermission {
    pub app_names_available: bool,
    pub window_titles_available: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveWindow {
    pub app_name: String,
    pub window_title: Option<String>,
    pub permission: ActivityPermission,
}

pub fn idle_seconds() -> f64 {
    #[cfg(target_os = "macos")]
    {
        const COMBINED_SESSION_STATE: u32 = 0;
        const ANY_INPUT_EVENT: u32 = u32::MAX;
        // SAFETY: This CoreGraphics query has no pointer arguments and no side
        // effects. The constants are the documented combined-session source
        // state and kCGAnyInputEventType.
        unsafe { CGEventSourceSecondsSinceLastEventType(COMBINED_SESSION_STATE, ANY_INPUT_EVENT) }
    }
    #[cfg(not(target_os = "macos"))]
    {
        0.0
    }
}

pub fn activity_permission() -> ActivityPermission {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: This read-only CoreGraphics permission query has no pointer
        // arguments and does not display a prompt.
        let window_titles_available = unsafe { CGPreflightScreenCaptureAccess() };
        ActivityPermission {
            app_names_available: true,
            window_titles_available,
            detail: if window_titles_available {
                "App names and window titles are available.".into()
            } else {
                "App names are available. Window titles stay off until Screen Recording access is enabled.".into()
            },
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        ActivityPermission {
            app_names_available: false,
            window_titles_available: false,
            detail: "App-linked tracking is available only on macOS.".into(),
        }
    }
}

pub fn request_activity_permission() -> ActivityPermission {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: This is the documented user-initiated CoreGraphics request.
        // macOS owns the consent UI and the returned Boolean.
        unsafe {
            let _ = CGRequestScreenCaptureAccess();
        }
        activity_permission()
    }
    #[cfg(not(target_os = "macos"))]
    {
        activity_permission()
    }
}

pub fn active_window() -> Result<ActiveWindow, String> {
    #[cfg(target_os = "macos")]
    {
        active_window_macos()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("App-linked tracking is available only on macOS.".into())
    }
}

#[cfg(target_os = "macos")]
fn active_window_macos() -> Result<ActiveWindow, String> {
    use std::ffi::{c_void, CStr, CString};

    const ON_SCREEN_ONLY: u32 = 1;
    const EXCLUDE_DESKTOP: u32 = 16;
    const NULL_WINDOW: u32 = 0;
    const UTF8: u32 = 0x0800_0100;
    const SINT32: isize = 3;

    unsafe fn key(name: &str) -> Result<*const c_void, String> {
        let name = CString::new(name).map_err(|error| error.to_string())?;
        // SAFETY: The C string is NUL-terminated for the duration of the call;
        // CoreFoundation copies it into an owned CFString.
        let value = unsafe { CFStringCreateWithCString(std::ptr::null(), name.as_ptr(), UTF8) };
        if value.is_null() {
            Err(format!("could not create CoreFoundation key {name:?}"))
        } else {
            Ok(value)
        }
    }

    unsafe fn dictionary_value(
        dictionary: *const c_void,
        key: *const c_void,
    ) -> Option<*const c_void> {
        let mut value = std::ptr::null();
        // SAFETY: The dictionary and key come from CoreGraphics/CoreFoundation
        // and remain alive while the owning window array is retained.
        (unsafe { CFDictionaryGetValueIfPresent(dictionary, key, &mut value) } != 0)
            .then_some(value)
    }

    unsafe fn string_value(value: *const c_void) -> Option<String> {
        let mut buffer = [0_i8; 2048];
        // SAFETY: The caller supplies a CFString borrowed from the retained
        // window dictionary and a correctly sized writable buffer.
        if unsafe { CFStringGetCString(value, buffer.as_mut_ptr(), buffer.len() as isize, UTF8) }
            == 0
        {
            return None;
        }
        // SAFETY: CFStringGetCString wrote a NUL-terminated UTF-8 string.
        unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_str()
            .ok()
            .map(ToString::to_string)
    }

    let permission = activity_permission();
    // SAFETY: The documented list call returns a retained CFArray. We release
    // it exactly once after borrowing its dictionaries.
    let windows =
        unsafe { CGWindowListCopyWindowInfo(ON_SCREEN_ONLY | EXCLUDE_DESKTOP, NULL_WINDOW) };
    if windows.is_null() {
        return Err("macOS did not return the foreground window list.".into());
    }
    let owner_key = unsafe { key("kCGWindowOwnerName")? };
    let name_key = unsafe { key("kCGWindowName")? };
    let layer_key = unsafe { key("kCGWindowLayer")? };
    let count = unsafe { CFArrayGetCount(windows) };
    let mut result = None;
    for index in 0..count {
        let dictionary = unsafe { CFArrayGetValueAtIndex(windows, index) };
        let mut layer = -1_i32;
        let layer_ok = unsafe {
            dictionary_value(dictionary, layer_key).is_some_and(|value| {
                CFNumberGetValue(value, SINT32, (&mut layer as *mut i32).cast()) != 0
            })
        };
        if !layer_ok || layer != 0 {
            continue;
        }
        let app_name = unsafe {
            dictionary_value(dictionary, owner_key).and_then(|value| string_value(value))
        };
        let Some(app_name) = app_name.filter(|name| !name.trim().is_empty()) else {
            continue;
        };
        let window_title = if permission.window_titles_available {
            unsafe {
                dictionary_value(dictionary, name_key)
                    .and_then(|value| string_value(value))
                    .filter(|title| !title.trim().is_empty())
            }
        } else {
            None
        };
        result = Some(ActiveWindow {
            app_name,
            window_title,
            permission: permission.clone(),
        });
        break;
    }
    unsafe {
        CFRelease(owner_key);
        CFRelease(name_key);
        CFRelease(layer_key);
        CFRelease(windows);
    }
    result.ok_or_else(|| "No foreground app is visible right now.".into())
}
