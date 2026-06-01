use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::slice;

use terminal::{mouse, mouse_encode, point};

mod input;
mod terminal;

// ABI ownership model:
// - Config/app/surface handles returned by Roastty are heap-owned by Roastty and
//   released only by their matching free function.
// - Runtime callback userdata, surface config pointers, strings, env arrays,
//   platform pointers, and app pointers stored on surfaces are borrowed from the
//   caller; this skeleton records scalar values but never frees borrowed data.
// - RoasttyString values are freed only by roastty_string_free and only when
//   they were returned by Roastty string-returning functions.
pub type RoasttyApp = *mut c_void;
pub type RoasttyConfig = *mut c_void;
pub type RoasttyMouseEncoder = *mut c_void;
pub type RoasttyMouseEvent = *mut c_void;
pub type RoasttySurface = *mut c_void;

const ROASTTY_SUCCESS: c_int = 0;
#[allow(dead_code)]
const ROASTTY_OUT_OF_MEMORY: c_int = 1;
const ROASTTY_INVALID_VALUE: c_int = 2;
const ROASTTY_OUT_OF_SPACE: c_int = 3;
const ROASTTY_BUILD_MODE_DEBUG: c_int = 0;

#[repr(C)]
pub struct RoasttyInfo {
    build_mode: c_int,
    version: *const c_char,
    version_len: usize,
}

#[repr(C)]
pub struct RoasttyDiagnostic {
    message: *const c_char,
}

#[repr(C)]
pub struct RoasttyConfigPath {
    path: *const c_char,
    optional: bool,
}

#[repr(C)]
pub struct RoasttyString {
    ptr: *const c_char,
    len: usize,
    sentinel: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoasttyEnvVar {
    key: *const c_char,
    value: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoasttyPlatformMacos {
    nsview: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoasttyPlatformIos {
    uiview: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union RoasttyPlatform {
    macos: RoasttyPlatformMacos,
    ios: RoasttyPlatformIos,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoasttySurfaceConfig {
    platform_tag: c_int,
    platform: RoasttyPlatform,
    userdata: *mut c_void,
    scale_factor: f64,
    font_size: f32,
    working_directory: *const c_char,
    command: *const c_char,
    env_vars: *mut RoasttyEnvVar,
    env_var_count: usize,
    initial_input: *const c_char,
    wait_after_command: bool,
    context: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoasttySurfaceSize {
    columns: u16,
    rows: u16,
    width_px: u32,
    height_px: u32,
    cell_width_px: u32,
    cell_height_px: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoasttyClipboardContent {
    mime: *const c_char,
    data: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoasttyTarget {
    tag: c_int,
    surface: RoasttySurface,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoasttyAction {
    tag: c_int,
    storage: [usize; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoasttyMouseMods {
    shift: bool,
    alt: bool,
    ctrl: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoasttyMousePosition {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoasttyMouseEncoderSize {
    size: usize,
    screen_width: u32,
    screen_height: u32,
    cell_width: u32,
    cell_height: u32,
    padding_top: u32,
    padding_bottom: u32,
    padding_right: u32,
    padding_left: u32,
}

type WakeupCallback = Option<unsafe extern "C" fn(*mut c_void)>;
type ActionCallback =
    Option<unsafe extern "C" fn(RoasttyApp, RoasttyTarget, RoasttyAction) -> bool>;
type ReadClipboardCallback = Option<unsafe extern "C" fn(*mut c_void, c_int, *mut c_void) -> bool>;
type ConfirmReadClipboardCallback =
    Option<unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_void, c_int)>;
type WriteClipboardCallback =
    Option<unsafe extern "C" fn(*mut c_void, c_int, *const RoasttyClipboardContent, usize, bool)>;
type CloseSurfaceCallback = Option<unsafe extern "C" fn(*mut c_void, bool)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoasttyRuntimeConfig {
    userdata: *mut c_void,
    supports_selection_clipboard: bool,
    wakeup_cb: WakeupCallback,
    action_cb: ActionCallback,
    read_clipboard_cb: ReadClipboardCallback,
    confirm_read_clipboard_cb: ConfirmReadClipboardCallback,
    write_clipboard_cb: WriteClipboardCallback,
    close_surface_cb: CloseSurfaceCallback,
}

struct Config {
    finalized: bool,
}

struct App {
    runtime: RoasttyRuntimeConfig,
    focused: bool,
    color_scheme: c_int,
}

struct Surface {
    app: RoasttyApp,
    userdata: *mut c_void,
    scale_factor_x: f64,
    scale_factor_y: f64,
    focused: bool,
    occluded: bool,
    size: RoasttySurfaceSize,
    color_scheme: c_int,
}

struct MouseEvent {
    event: mouse_encode::Event,
}

struct MouseEncoder {
    event: mouse::MouseEventMode,
    format: mouse::MouseFormat,
    geometry: mouse_encode::Geometry,
    any_button_pressed: bool,
    track_last_cell: bool,
    last_cell: Option<point::Coordinate>,
}

static VERSION: &[u8] = b"0.1.0-roastty\0";
static EMPTY_DIAGNOSTIC: &[u8] = b"\0";
static WINDOW_SAVE_STATE_DEFAULT: &[u8] = b"default\0";
static WINDOW_DECORATION_AUTO: &[u8] = b"auto\0";
static WINDOW_THEME_AUTO: &[u8] = b"auto\0";

fn config_from_handle<'a>(handle: RoasttyConfig) -> Option<&'a mut Config> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &mut *(handle.cast::<Config>()) })
    }
}

fn app_from_handle<'a>(handle: RoasttyApp) -> Option<&'a mut App> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &mut *(handle.cast::<App>()) })
    }
}

fn surface_from_handle<'a>(handle: RoasttySurface) -> Option<&'a mut Surface> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &mut *(handle.cast::<Surface>()) })
    }
}

fn mouse_event_from_handle<'a>(handle: RoasttyMouseEvent) -> Option<&'a mut MouseEvent> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &mut *(handle.cast::<MouseEvent>()) })
    }
}

fn mouse_encoder_from_handle<'a>(handle: RoasttyMouseEncoder) -> Option<&'a mut MouseEncoder> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &mut *(handle.cast::<MouseEncoder>()) })
    }
}

fn mouse_action_from_int(value: c_int) -> Option<mouse::MouseAction> {
    match value {
        0 => Some(mouse::MouseAction::Press),
        1 => Some(mouse::MouseAction::Release),
        2 => Some(mouse::MouseAction::Motion),
        _ => None,
    }
}

fn mouse_action_to_int(value: mouse::MouseAction) -> c_int {
    match value {
        mouse::MouseAction::Press => 0,
        mouse::MouseAction::Release => 1,
        mouse::MouseAction::Motion => 2,
    }
}

fn mouse_button_from_int(value: c_int) -> Option<mouse::MouseButton> {
    match value {
        0 => Some(mouse::MouseButton::Unknown),
        1 => Some(mouse::MouseButton::Left),
        2 => Some(mouse::MouseButton::Right),
        3 => Some(mouse::MouseButton::Middle),
        4 => Some(mouse::MouseButton::Four),
        5 => Some(mouse::MouseButton::Five),
        6 => Some(mouse::MouseButton::Six),
        7 => Some(mouse::MouseButton::Seven),
        8 => Some(mouse::MouseButton::Eight),
        9 => Some(mouse::MouseButton::Nine),
        10 => Some(mouse::MouseButton::Ten),
        11 => Some(mouse::MouseButton::Eleven),
        _ => None,
    }
}

fn mouse_button_to_int(value: mouse::MouseButton) -> c_int {
    match value {
        mouse::MouseButton::Unknown => 0,
        mouse::MouseButton::Left => 1,
        mouse::MouseButton::Right => 2,
        mouse::MouseButton::Middle => 3,
        mouse::MouseButton::Four => 4,
        mouse::MouseButton::Five => 5,
        mouse::MouseButton::Six => 6,
        mouse::MouseButton::Seven => 7,
        mouse::MouseButton::Eight => 8,
        mouse::MouseButton::Nine => 9,
        mouse::MouseButton::Ten => 10,
        mouse::MouseButton::Eleven => 11,
    }
}

fn mouse_event_mode_from_int(value: c_int) -> Option<mouse::MouseEventMode> {
    match value {
        0 => Some(mouse::MouseEventMode::None),
        1 => Some(mouse::MouseEventMode::X10),
        2 => Some(mouse::MouseEventMode::Normal),
        3 => Some(mouse::MouseEventMode::Button),
        4 => Some(mouse::MouseEventMode::Any),
        _ => None,
    }
}

fn mouse_format_from_int(value: c_int) -> Option<mouse::MouseFormat> {
    match value {
        0 => Some(mouse::MouseFormat::X10),
        1 => Some(mouse::MouseFormat::Utf8),
        2 => Some(mouse::MouseFormat::Sgr),
        3 => Some(mouse::MouseFormat::Urxvt),
        4 => Some(mouse::MouseFormat::SgrPixels),
        _ => None,
    }
}

fn default_mouse_geometry() -> mouse_encode::Geometry {
    mouse_encode::Geometry {
        screen: mouse_encode::PixelSize {
            width: 1,
            height: 1,
        },
        cell: mouse_encode::PixelSize {
            width: 1,
            height: 1,
        },
        padding: mouse_encode::Padding::default(),
    }
}

fn mouse_geometry_from_abi(size: &RoasttyMouseEncoderSize) -> Option<mouse_encode::Geometry> {
    if size.cell_width == 0 || size.cell_height == 0 {
        return None;
    }

    Some(mouse_encode::Geometry {
        screen: mouse_encode::PixelSize {
            width: size.screen_width,
            height: size.screen_height,
        },
        cell: mouse_encode::PixelSize {
            width: size.cell_width,
            height: size.cell_height,
        },
        padding: mouse_encode::Padding {
            top: size.padding_top,
            bottom: size.padding_bottom,
            right: size.padding_right,
            left: size.padding_left,
        },
    })
}

fn mouse_geometry_from_abi_ptr(value: *const c_void) -> Option<mouse_encode::Geometry> {
    let provided_size = unsafe { value.cast::<usize>().read() };
    if provided_size < std::mem::size_of::<RoasttyMouseEncoderSize>() {
        return None;
    }

    let size = unsafe { value.cast::<RoasttyMouseEncoderSize>().read() };
    mouse_geometry_from_abi(&size)
}

fn empty_string() -> RoasttyString {
    RoasttyString {
        ptr: ptr::null(),
        len: 0,
        sentinel: false,
    }
}

fn allocated_string(bytes: &[u8]) -> RoasttyString {
    let owned = bytes.to_vec().into_boxed_slice();
    let len = owned.len();
    let ptr = Box::into_raw(owned).cast::<u8>();
    RoasttyString {
        ptr: ptr.cast::<c_char>(),
        len,
        sentinel: false,
    }
}

fn allocated_c_string(value: &str) -> RoasttyString {
    let c_string = CString::new(value).expect("static strings must not contain interior nuls");
    let len = c_string.as_bytes().len();
    let ptr = c_string.into_raw();
    RoasttyString {
        ptr,
        len,
        sentinel: true,
    }
}

#[no_mangle]
pub extern "C" fn roastty_init(_argc: usize, _argv: *mut *mut c_char) -> c_int {
    ROASTTY_SUCCESS
}

#[no_mangle]
pub extern "C" fn roastty_info() -> RoasttyInfo {
    RoasttyInfo {
        build_mode: ROASTTY_BUILD_MODE_DEBUG,
        version: VERSION.as_ptr().cast::<c_char>(),
        version_len: VERSION.len() - 1,
    }
}

#[no_mangle]
pub extern "C" fn roastty_string_free(value: RoasttyString) {
    if value.ptr.is_null() || value.len == 0 {
        return;
    }

    unsafe {
        if value.sentinel {
            drop(CString::from_raw(value.ptr.cast_mut()));
        } else {
            let slice = ptr::slice_from_raw_parts_mut(value.ptr.cast::<u8>().cast_mut(), value.len);
            drop(Box::from_raw(slice));
        }
    }
}

#[no_mangle]
pub extern "C" fn roastty_config_new() -> RoasttyConfig {
    Box::into_raw(Box::new(Config { finalized: false })).cast()
}

#[no_mangle]
pub extern "C" fn roastty_config_free(config: RoasttyConfig) {
    if !config.is_null() {
        unsafe {
            drop(Box::from_raw(config.cast::<Config>()));
        }
    }
}

#[no_mangle]
pub extern "C" fn roastty_config_clone(config: RoasttyConfig) -> RoasttyConfig {
    let finalized = config_from_handle(config)
        .map(|config| config.finalized)
        .unwrap_or(false);
    Box::into_raw(Box::new(Config { finalized })).cast()
}

#[no_mangle]
pub extern "C" fn roastty_config_load_cli_args(_config: RoasttyConfig) {}

#[no_mangle]
pub extern "C" fn roastty_config_load_file(_config: RoasttyConfig, _path: *const c_char) {}

#[no_mangle]
pub extern "C" fn roastty_config_load_default_files(_config: RoasttyConfig) {}

#[no_mangle]
pub extern "C" fn roastty_config_load_recursive_files(_config: RoasttyConfig) {}

#[no_mangle]
pub extern "C" fn roastty_config_finalize(config: RoasttyConfig) {
    if let Some(config) = config_from_handle(config) {
        config.finalized = true;
    }
}

#[no_mangle]
pub extern "C" fn roastty_config_get(
    config: RoasttyConfig,
    output: *mut c_void,
    key: *const c_char,
    key_len: usize,
) -> bool {
    if config.is_null() || output.is_null() || key.is_null() {
        return false;
    }

    let key = unsafe { slice::from_raw_parts(key.cast::<u8>(), key_len) };
    unsafe {
        match key {
            b"initial-window" => {
                output.cast::<bool>().write(true);
                true
            }
            b"quit-after-last-window-closed" => {
                output.cast::<bool>().write(false);
                true
            }
            b"window-save-state" => {
                output
                    .cast::<*const c_char>()
                    .write(WINDOW_SAVE_STATE_DEFAULT.as_ptr().cast());
                true
            }
            b"window-decoration" => {
                output
                    .cast::<*const c_char>()
                    .write(WINDOW_DECORATION_AUTO.as_ptr().cast());
                true
            }
            b"window-theme" => {
                output
                    .cast::<*const c_char>()
                    .write(WINDOW_THEME_AUTO.as_ptr().cast());
                true
            }
            b"background-opacity" => {
                output.cast::<f64>().write(1.0);
                true
            }
            b"bell-audio-volume" => {
                output.cast::<f64>().write(0.5);
                true
            }
            b"notify-on-command-finish-after" => {
                output.cast::<usize>().write(5000);
                true
            }
            b"title" => {
                output.cast::<*const c_char>().write(ptr::null());
                true
            }
            b"window-position-x" | b"window-position-y" | b"bell-audio-path" => false,
            _ => false,
        }
    }
}

#[no_mangle]
pub extern "C" fn roastty_config_diagnostics_count(_config: RoasttyConfig) -> u32 {
    0
}

#[no_mangle]
pub extern "C" fn roastty_config_get_diagnostic(
    _config: RoasttyConfig,
    _index: u32,
) -> RoasttyDiagnostic {
    RoasttyDiagnostic {
        message: EMPTY_DIAGNOSTIC.as_ptr().cast::<c_char>(),
    }
}

#[no_mangle]
pub extern "C" fn roastty_config_open_path() -> RoasttyString {
    allocated_string(b"roastty-config")
}

#[no_mangle]
pub extern "C" fn roastty_app_new(
    runtime: *const RoasttyRuntimeConfig,
    _config: RoasttyConfig,
) -> RoasttyApp {
    let runtime = if runtime.is_null() {
        RoasttyRuntimeConfig {
            userdata: ptr::null_mut(),
            supports_selection_clipboard: false,
            wakeup_cb: None,
            action_cb: None,
            read_clipboard_cb: None,
            confirm_read_clipboard_cb: None,
            write_clipboard_cb: None,
            close_surface_cb: None,
        }
    } else {
        unsafe { *runtime }
    };

    Box::into_raw(Box::new(App {
        runtime,
        focused: false,
        color_scheme: 0,
    }))
    .cast()
}

#[no_mangle]
pub extern "C" fn roastty_app_free(app: RoasttyApp) {
    if !app.is_null() {
        unsafe {
            drop(Box::from_raw(app.cast::<App>()));
        }
    }
}

#[no_mangle]
pub extern "C" fn roastty_app_tick(_app: RoasttyApp) {}

#[no_mangle]
pub extern "C" fn roastty_app_userdata(app: RoasttyApp) -> *mut c_void {
    app_from_handle(app)
        .map(|app| app.runtime.userdata)
        .unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn roastty_app_set_focus(app: RoasttyApp, focused: bool) {
    if let Some(app) = app_from_handle(app) {
        app.focused = focused;
    }
}

#[no_mangle]
pub extern "C" fn roastty_app_update_config(_app: RoasttyApp, _config: RoasttyConfig) {}

#[no_mangle]
pub extern "C" fn roastty_app_needs_confirm_quit(_app: RoasttyApp) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn roastty_app_has_global_keybinds(_app: RoasttyApp) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn roastty_app_set_color_scheme(app: RoasttyApp, color_scheme: c_int) {
    if let Some(app) = app_from_handle(app) {
        app.color_scheme = color_scheme;
    }
}

#[no_mangle]
pub extern "C" fn roastty_mouse_event_new(out: *mut RoasttyMouseEvent) -> c_int {
    if out.is_null() {
        return ROASTTY_INVALID_VALUE;
    }

    let event = Box::new(MouseEvent {
        event: mouse_encode::Event::default(),
    });
    unsafe {
        out.write(Box::into_raw(event).cast());
    }
    ROASTTY_SUCCESS
}

#[no_mangle]
pub extern "C" fn roastty_mouse_event_free(event: RoasttyMouseEvent) {
    if !event.is_null() {
        unsafe {
            drop(Box::from_raw(event.cast::<MouseEvent>()));
        }
    }
}

#[no_mangle]
pub extern "C" fn roastty_mouse_event_set_action(event: RoasttyMouseEvent, action: c_int) -> c_int {
    let Some(event) = mouse_event_from_handle(event) else {
        return ROASTTY_INVALID_VALUE;
    };
    let Some(action) = mouse_action_from_int(action) else {
        return ROASTTY_INVALID_VALUE;
    };

    event.event.action = action;
    ROASTTY_SUCCESS
}

#[no_mangle]
pub extern "C" fn roastty_mouse_event_get_action(event: RoasttyMouseEvent) -> c_int {
    mouse_event_from_handle(event)
        .map(|event| mouse_action_to_int(event.event.action))
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn roastty_mouse_event_set_button(event: RoasttyMouseEvent, button: c_int) -> c_int {
    let Some(event) = mouse_event_from_handle(event) else {
        return ROASTTY_INVALID_VALUE;
    };
    let Some(button) = mouse_button_from_int(button) else {
        return ROASTTY_INVALID_VALUE;
    };

    event.event.button = Some(button);
    ROASTTY_SUCCESS
}

#[no_mangle]
pub extern "C" fn roastty_mouse_event_clear_button(event: RoasttyMouseEvent) {
    if let Some(event) = mouse_event_from_handle(event) {
        event.event.button = None;
    }
}

#[no_mangle]
pub extern "C" fn roastty_mouse_event_get_button(
    event: RoasttyMouseEvent,
    out: *mut c_int,
) -> bool {
    let Some(event) = mouse_event_from_handle(event) else {
        return false;
    };
    let Some(button) = event.event.button else {
        return false;
    };

    if !out.is_null() {
        unsafe {
            out.write(mouse_button_to_int(button));
        }
    }
    true
}

#[no_mangle]
pub extern "C" fn roastty_mouse_event_set_mods(event: RoasttyMouseEvent, mods: RoasttyMouseMods) {
    if let Some(event) = mouse_event_from_handle(event) {
        event.event.mods = mouse::MouseMods {
            shift: mods.shift,
            alt: mods.alt,
            ctrl: mods.ctrl,
        };
    }
}

#[no_mangle]
pub extern "C" fn roastty_mouse_event_get_mods(event: RoasttyMouseEvent) -> RoasttyMouseMods {
    mouse_event_from_handle(event)
        .map(|event| RoasttyMouseMods {
            shift: event.event.mods.shift,
            alt: event.event.mods.alt,
            ctrl: event.event.mods.ctrl,
        })
        .unwrap_or(RoasttyMouseMods {
            shift: false,
            alt: false,
            ctrl: false,
        })
}

#[no_mangle]
pub extern "C" fn roastty_mouse_event_set_position(
    event: RoasttyMouseEvent,
    pos: RoasttyMousePosition,
) {
    if let Some(event) = mouse_event_from_handle(event) {
        event.event.pos = mouse_encode::Position { x: pos.x, y: pos.y };
    }
}

#[no_mangle]
pub extern "C" fn roastty_mouse_event_get_position(
    event: RoasttyMouseEvent,
) -> RoasttyMousePosition {
    mouse_event_from_handle(event)
        .map(|event| RoasttyMousePosition {
            x: event.event.pos.x,
            y: event.event.pos.y,
        })
        .unwrap_or(RoasttyMousePosition { x: 0.0, y: 0.0 })
}

#[no_mangle]
pub extern "C" fn roastty_mouse_encoder_new(out: *mut RoasttyMouseEncoder) -> c_int {
    if out.is_null() {
        return ROASTTY_INVALID_VALUE;
    }

    let encoder = Box::new(MouseEncoder {
        event: mouse::MouseEventMode::None,
        format: mouse::MouseFormat::X10,
        geometry: default_mouse_geometry(),
        any_button_pressed: false,
        track_last_cell: false,
        last_cell: None,
    });
    unsafe {
        out.write(Box::into_raw(encoder).cast());
    }
    ROASTTY_SUCCESS
}

#[no_mangle]
pub extern "C" fn roastty_mouse_encoder_free(encoder: RoasttyMouseEncoder) {
    if !encoder.is_null() {
        unsafe {
            drop(Box::from_raw(encoder.cast::<MouseEncoder>()));
        }
    }
}

#[no_mangle]
pub extern "C" fn roastty_mouse_encoder_setopt(
    encoder: RoasttyMouseEncoder,
    option: c_int,
    value: *const c_void,
) -> c_int {
    let Some(encoder) = mouse_encoder_from_handle(encoder) else {
        return ROASTTY_INVALID_VALUE;
    };
    if value.is_null() {
        return ROASTTY_INVALID_VALUE;
    }

    match option {
        0 => {
            let value = unsafe { value.cast::<c_int>().read() };
            let Some(event) = mouse_event_mode_from_int(value) else {
                return ROASTTY_INVALID_VALUE;
            };
            if encoder.event != event {
                encoder.last_cell = None;
            }
            encoder.event = event;
        }
        1 => {
            let value = unsafe { value.cast::<c_int>().read() };
            let Some(format) = mouse_format_from_int(value) else {
                return ROASTTY_INVALID_VALUE;
            };
            if encoder.format != format {
                encoder.last_cell = None;
            }
            encoder.format = format;
        }
        2 => {
            let Some(geometry) = mouse_geometry_from_abi_ptr(value) else {
                return ROASTTY_INVALID_VALUE;
            };
            encoder.geometry = geometry;
            encoder.last_cell = None;
        }
        3 => {
            encoder.any_button_pressed = unsafe { value.cast::<bool>().read() };
        }
        4 => {
            encoder.track_last_cell = unsafe { value.cast::<bool>().read() };
            if !encoder.track_last_cell {
                encoder.last_cell = None;
            }
        }
        _ => return ROASTTY_INVALID_VALUE,
    }

    ROASTTY_SUCCESS
}

#[no_mangle]
pub extern "C" fn roastty_mouse_encoder_reset(encoder: RoasttyMouseEncoder) {
    if let Some(encoder) = mouse_encoder_from_handle(encoder) {
        encoder.last_cell = None;
    }
}

#[no_mangle]
pub extern "C" fn roastty_mouse_encoder_encode(
    encoder: RoasttyMouseEncoder,
    event: RoasttyMouseEvent,
    out: *mut u8,
    out_len: usize,
    out_written: *mut usize,
) -> c_int {
    let Some(encoder) = mouse_encoder_from_handle(encoder) else {
        return ROASTTY_INVALID_VALUE;
    };
    let Some(event) = mouse_event_from_handle(event) else {
        return ROASTTY_INVALID_VALUE;
    };
    if out_written.is_null() {
        return ROASTTY_INVALID_VALUE;
    }
    if out.is_null() && out_len != 0 {
        return ROASTTY_INVALID_VALUE;
    }

    let mut next_last_cell = encoder.last_cell;
    let encoded = mouse_encode::encode(
        event.event,
        mouse_encode::Options {
            event: encoder.event,
            format: encoder.format,
            geometry: encoder.geometry,
            any_button_pressed: encoder.any_button_pressed,
            last_cell: encoder.track_last_cell.then_some(&mut next_last_cell),
        },
    )
    .unwrap_or_default();

    unsafe {
        out_written.write(encoded.len());
    }

    if encoded.len() > out_len || (!encoded.is_empty() && out.is_null()) {
        return ROASTTY_OUT_OF_SPACE;
    }

    if !encoded.is_empty() {
        unsafe {
            ptr::copy_nonoverlapping(encoded.as_ptr(), out, encoded.len());
        }
    }
    if encoder.track_last_cell {
        encoder.last_cell = next_last_cell;
    }

    ROASTTY_SUCCESS
}

#[no_mangle]
pub extern "C" fn roastty_surface_config_new() -> RoasttySurfaceConfig {
    RoasttySurfaceConfig {
        platform_tag: 0,
        platform: RoasttyPlatform {
            macos: RoasttyPlatformMacos {
                nsview: ptr::null_mut(),
            },
        },
        userdata: ptr::null_mut(),
        scale_factor: 1.0,
        font_size: 0.0,
        working_directory: ptr::null(),
        command: ptr::null(),
        env_vars: ptr::null_mut(),
        env_var_count: 0,
        initial_input: ptr::null(),
        wait_after_command: false,
        context: 0,
    }
}

#[no_mangle]
pub extern "C" fn roastty_surface_new(
    app: RoasttyApp,
    config: *const RoasttySurfaceConfig,
) -> RoasttySurface {
    if app.is_null() {
        return ptr::null_mut();
    }

    let config = if config.is_null() {
        roastty_surface_config_new()
    } else {
        unsafe { *config }
    };

    Box::into_raw(Box::new(Surface {
        app,
        userdata: config.userdata,
        scale_factor_x: config.scale_factor,
        scale_factor_y: config.scale_factor,
        focused: false,
        occluded: false,
        size: RoasttySurfaceSize {
            columns: 0,
            rows: 0,
            width_px: 0,
            height_px: 0,
            cell_width_px: 0,
            cell_height_px: 0,
        },
        color_scheme: 0,
    }))
    .cast()
}

#[no_mangle]
pub extern "C" fn roastty_surface_free(surface: RoasttySurface) {
    if !surface.is_null() {
        unsafe {
            drop(Box::from_raw(surface.cast::<Surface>()));
        }
    }
}

#[no_mangle]
pub extern "C" fn roastty_surface_userdata(surface: RoasttySurface) -> *mut c_void {
    surface_from_handle(surface)
        .map(|surface| surface.userdata)
        .unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn roastty_surface_app(surface: RoasttySurface) -> RoasttyApp {
    surface_from_handle(surface)
        .map(|surface| surface.app)
        .unwrap_or(ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn roastty_surface_update_config(_surface: RoasttySurface, _config: RoasttyConfig) {}

#[no_mangle]
pub extern "C" fn roastty_surface_needs_confirm_quit(_surface: RoasttySurface) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn roastty_surface_process_exited(_surface: RoasttySurface) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn roastty_surface_set_content_scale(surface: RoasttySurface, x: f64, y: f64) {
    if let Some(surface) = surface_from_handle(surface) {
        surface.scale_factor_x = x;
        surface.scale_factor_y = y;
    }
}

#[no_mangle]
pub extern "C" fn roastty_surface_set_focus(surface: RoasttySurface, focused: bool) {
    if let Some(surface) = surface_from_handle(surface) {
        surface.focused = focused;
    }
}

#[no_mangle]
pub extern "C" fn roastty_surface_set_occlusion(surface: RoasttySurface, occluded: bool) {
    if let Some(surface) = surface_from_handle(surface) {
        surface.occluded = occluded;
    }
}

#[no_mangle]
pub extern "C" fn roastty_surface_set_size(surface: RoasttySurface, width: u32, height: u32) {
    if let Some(surface) = surface_from_handle(surface) {
        surface.size.width_px = width;
        surface.size.height_px = height;
    }
}

#[no_mangle]
pub extern "C" fn roastty_surface_size(surface: RoasttySurface) -> RoasttySurfaceSize {
    surface_from_handle(surface)
        .map(|surface| surface.size)
        .unwrap_or(RoasttySurfaceSize {
            columns: 0,
            rows: 0,
            width_px: 0,
            height_px: 0,
            cell_width_px: 0,
            cell_height_px: 0,
        })
}

#[no_mangle]
pub extern "C" fn roastty_surface_foreground_pid(_surface: RoasttySurface) -> u64 {
    0
}

#[no_mangle]
pub extern "C" fn roastty_surface_tty_name(surface: RoasttySurface) -> RoasttyString {
    if surface.is_null() {
        empty_string()
    } else {
        allocated_c_string("roastty-skeleton-tty")
    }
}

#[no_mangle]
pub extern "C" fn roastty_surface_set_color_scheme(surface: RoasttySurface, color_scheme: c_int) {
    if let Some(surface) = surface_from_handle(surface) {
        surface.color_scheme = color_scheme;
    }
}

#[no_mangle]
pub extern "C" fn roastty_surface_request_close(_surface: RoasttySurface) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string_shape_matches_roastty() {
        let value = empty_string();
        assert!(value.ptr.is_null());
        assert_eq!(value.len, 0);
        assert!(!value.sentinel);
        roastty_string_free(value);
    }

    #[test]
    fn allocated_non_sentinel_string_can_be_freed() {
        let value = roastty_config_open_path();
        assert!(!value.ptr.is_null());
        assert_eq!(value.len, "roastty-config".len());
        assert!(!value.sentinel);
        roastty_string_free(value);
    }

    #[test]
    fn allocated_sentinel_string_can_be_freed() {
        let config = roastty_config_new();
        let runtime = RoasttyRuntimeConfig {
            userdata: ptr::null_mut(),
            supports_selection_clipboard: false,
            wakeup_cb: None,
            action_cb: None,
            read_clipboard_cb: None,
            confirm_read_clipboard_cb: None,
            write_clipboard_cb: None,
            close_surface_cb: None,
        };
        let app = roastty_app_new(&runtime, config);
        let surface_config = roastty_surface_config_new();
        let surface = roastty_surface_new(app, &surface_config);

        let value = roastty_surface_tty_name(surface);
        assert!(!value.ptr.is_null());
        assert_eq!(value.len, "roastty-skeleton-tty".len());
        assert!(value.sentinel);
        roastty_string_free(value);

        roastty_surface_free(surface);
        roastty_app_free(app);
        roastty_config_free(config);
    }
}
