#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(source_state_id: u32, event_type: u32) -> f64;
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
