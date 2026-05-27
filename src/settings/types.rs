use windows::Win32::Graphics::Gdi::HBRUSH;

// Thread-safe wrapper for HBRUSH raw pointer
#[derive(Clone, Copy)]
pub struct SafeHbrush(pub HBRUSH);
unsafe impl Send for SafeHbrush {}
unsafe impl Sync for SafeHbrush {}

#[derive(Clone, Copy, PartialEq, Debug)]
#[allow(dead_code)]
pub enum HoveredControl {
    None,
    SidebarTab(usize),
    // Tab 0
    DeadzoneDec,
    DeadzoneInc,
    ThresholdDec,
    ThresholdInc,
    StartupDec,
    StartupInc,
    GesturesDec,
    GesturesInc,
    // Tab 1
    GapDec,
    GapInc,
    RadiusDec,
    RadiusInc,
    OpacityDec,
    OpacityInc,
    LayoutsDec,
    LayoutsInc,
    // Action Buttons
    SaveButton,
}

// Live state management for dynamic dashboard
pub struct DashboardState {
    pub active_tab: usize,
    pub hovered: HoveredControl,
    // Cached config values for dynamic adjustment
    pub deadzone: i32,
    pub threshold: i32,
    pub gap: i32,
    pub radius: i32,
    pub opacity: u8,
    pub fill_color: u32,
    pub border_color: u32,
    pub blacklist_text: String,
    pub startup_enabled: bool,
    pub layouts_enabled: bool,
    pub gestures_enabled: bool,
}
