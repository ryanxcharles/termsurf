use crate::connection::ConnectionOps;
use crate::macos::menu::RepresentedItem;
use crate::macos::{nsstring, nsstring_to_str};
use crate::menu::{Menu, MenuItem};
use crate::{ApplicationEvent, Connection};
use config::keyassignment::KeyAssignment;
use config::WindowCloseConfirmation;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
use objc2_app_kit::NSApplicationTerminateReply;
use objc2_app_kit::NSEventModifierFlags;
use objc2_core_foundation::{CGPoint, CGRect};
use std::ffi::{c_char, c_void};
use std::sync::Once;

#[allow(non_camel_case_types)]
type id = *mut AnyObject;

#[link(name = "objc")]
extern "C" {
    fn class_getInstanceMethod(cls: *const AnyClass, name: Sel) -> *mut c_void;
    fn class_addMethod(
        cls: *const AnyClass,
        name: Sel,
        imp: *const c_void,
        types: *const c_char,
    ) -> Bool;
    fn method_exchangeImplementations(m1: *mut c_void, m2: *mut c_void);
}

fn issue_779_trace_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("TERMSURF_ISSUE_779_TRACE").is_some())
}

fn issue_779_trace_log(message: String) {
    if issue_779_trace_enabled() {
        log::info!("[issue-779-trace] {message}");
    }
}

unsafe fn objc_class_name(object: id) -> String {
    if object.is_null() {
        return "null".to_string();
    }
    let class_name: *mut AnyObject = objc2::msg_send![object as *const AnyObject, className];
    nsstring_to_str(class_name).to_string()
}

fn trace_bool(value: Bool) -> bool {
    value.as_bool()
}

fn trace_rect(rect: CGRect) -> String {
    format!(
        "{{{{{:.1}, {:.1}}}, {{{:.1}, {:.1}}}}}",
        rect.origin.x, rect.origin.y, rect.size.width, rect.size.height
    )
}

unsafe fn trace_window_summary(window: id) -> String {
    if window.is_null() {
        return "window=null".to_string();
    }

    let class_name = objc_class_name(window);
    let window_number: isize = objc2::msg_send![window as *const AnyObject, windowNumber];
    let frame: CGRect = objc2::msg_send![window as *const AnyObject, frame];
    let level: isize = objc2::msg_send![window as *const AnyObject, level];
    let is_visible: Bool = objc2::msg_send![window as *const AnyObject, isVisible];
    let ignores_mouse: Bool = objc2::msg_send![window as *const AnyObject, ignoresMouseEvents];
    let is_key: Bool = objc2::msg_send![window as *const AnyObject, isKeyWindow];
    let is_main: Bool = objc2::msg_send![window as *const AnyObject, isMainWindow];
    let first_responder: id = objc2::msg_send![window as *const AnyObject, firstResponder];
    let content_view: id = objc2::msg_send![window as *const AnyObject, contentView];
    let parent_window: id = objc2::msg_send![window as *const AnyObject, parentWindow];
    let child_windows: id = objc2::msg_send![window as *const AnyObject, childWindows];
    let child_count: usize = if child_windows.is_null() {
        0
    } else {
        objc2::msg_send![child_windows as *const AnyObject, count]
    };

    format!(
        "window={:p} window_class={} window_number={} window_frame={} window_level={} window_is_visible={} window_ignores_mouse={} window_is_key={} window_is_main={} content_view={} first_responder={} parent_window={:p} child_window_count={}",
        window,
        class_name,
        window_number,
        trace_rect(frame),
        level,
        trace_bool(is_visible),
        trace_bool(ignores_mouse),
        trace_bool(is_key),
        trace_bool(is_main),
        objc_class_name(content_view),
        objc_class_name(first_responder),
        parent_window,
        child_count
    )
}

unsafe fn trace_ordered_windows(prefix: &str) -> String {
    let ns_app: id = objc2::msg_send![objc2::class!(NSApplication), sharedApplication];
    let windows: id = objc2::msg_send![ns_app as *const AnyObject, orderedWindows];
    if windows.is_null() {
        return format!("{prefix}=null");
    }

    let count: usize = objc2::msg_send![windows as *const AnyObject, count];
    let limit = count.min(3);
    let mut parts = Vec::with_capacity(limit);
    for index in 0..limit {
        let window: id = objc2::msg_send![windows as *const AnyObject, objectAtIndex: index];
        let window_number: isize = objc2::msg_send![window as *const AnyObject, windowNumber];
        let frame: CGRect = objc2::msg_send![window as *const AnyObject, frame];
        let level: isize = objc2::msg_send![window as *const AnyObject, level];
        let visible: Bool = objc2::msg_send![window as *const AnyObject, isVisible];
        let ignores_mouse: Bool = objc2::msg_send![window as *const AnyObject, ignoresMouseEvents];
        parts.push(format!(
            "{}:{:p}:{}:{}:{}:{}:{}",
            index,
            window,
            objc_class_name(window),
            window_number,
            trace_rect(frame),
            level,
            trace_bool(visible) && !trace_bool(ignores_mouse)
        ));
    }

    format!("{prefix}_count={} {prefix}_top3={}", count, parts.join("|"))
}

unsafe fn trace_event_type(event: id) -> isize {
    objc2::msg_send![event as *const AnyObject, type]
}

fn should_trace_event(event_type: isize) -> bool {
    matches!(event_type, 1 | 2 | 3 | 4 | 25 | 26) || {
        static MOVE_SAMPLE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        matches!(event_type, 5 | 6 | 7 | 27)
            && MOVE_SAMPLE.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 20 == 0
    }
}

unsafe fn trace_app_send_event(phase: &str, this: id, event: id) {
    if !issue_779_trace_enabled() || event.is_null() {
        return;
    }
    let event_type = trace_event_type(event);
    if !should_trace_event(event_type) {
        return;
    }

    let ns_app = this;
    let event_window: id = objc2::msg_send![event as *const AnyObject, window];
    let key_window: id = objc2::msg_send![ns_app as *const AnyObject, keyWindow];
    let main_window: id = objc2::msg_send![ns_app as *const AnyObject, mainWindow];
    let modal_window: id = objc2::msg_send![ns_app as *const AnyObject, modalWindow];
    let current_event: id = objc2::msg_send![ns_app as *const AnyObject, currentEvent];
    let location: CGPoint = objc2::msg_send![event as *const AnyObject, locationInWindow];
    let mouse_location: CGPoint = objc2::msg_send![objc2::class!(NSEvent), mouseLocation];
    let button_number: isize = objc2::msg_send![event as *const AnyObject, buttonNumber];
    let click_count: isize = objc2::msg_send![event as *const AnyObject, clickCount];
    let pressed_buttons: u64 = objc2::msg_send![objc2::class!(NSEvent), pressedMouseButtons];
    let modifier_flags: NSEventModifierFlags =
        objc2::msg_send![event as *const AnyObject, modifierFlags];
    let timestamp: f64 = objc2::msg_send![event as *const AnyObject, timestamp];
    let event_number: isize = objc2::msg_send![event as *const AnyObject, eventNumber];
    let window_number: isize = objc2::msg_send![event as *const AnyObject, windowNumber];
    let app_active: Bool = objc2::msg_send![ns_app as *const AnyObject, isActive];

    issue_779_trace_log(format!(
        "wezboard_appkit_dispatch boundary=nsapp_send_event phase={} event={:p} event_type={} button_number={} click_count={} pressed_buttons={} modifiers_raw={:?} timestamp={:.6} event_number={} event_window_number={} location_in_window=({:.1},{:.1}) mouse_screen=({:.1},{:.1}) app_is_active={} current_event_same={} current_event_type={} key_window={:p} main_window={:p} modal_window={:p} event_window_summary=\"{}\" key_window_summary=\"{}\" main_window_summary=\"{}\" modal_window_summary=\"{}\" {}",
        phase,
        event,
        event_type,
        button_number,
        click_count,
        pressed_buttons,
        modifier_flags,
        timestamp,
        event_number,
        window_number,
        location.x,
        location.y,
        mouse_location.x,
        mouse_location.y,
        trace_bool(app_active),
        current_event == event,
        if current_event.is_null() {
            -1
        } else {
            trace_event_type(current_event)
        },
        key_window,
        main_window,
        modal_window,
        trace_window_summary(event_window),
        trace_window_summary(key_window),
        trace_window_summary(main_window),
        trace_window_summary(modal_window),
        trace_ordered_windows("ordered_windows")
    ));
}

extern "C" fn trace_send_event(this: *mut AnyObject, _sel: Sel, event: *mut AnyObject) {
    unsafe {
        trace_app_send_event("before", this, event);
        let () = objc2::msg_send![this as *const AnyObject, wezboardTraceSendEvent: event];
        trace_app_send_event("after", this, event);
    }
}

pub fn install_issue_779_appkit_trace() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        if !issue_779_trace_enabled() {
            return;
        }
        unsafe {
            let cls = AnyClass::get(c"NSApplication").unwrap();
            let original = class_getInstanceMethod(cls, objc2::sel!(sendEvent:));
            let trace_sel = objc2::sel!(wezboardTraceSendEvent:);
            let added = class_addMethod(
                cls,
                trace_sel,
                trace_send_event as *const c_void,
                c"v@:@".as_ptr(),
            );
            let trace_method = class_getInstanceMethod(cls, trace_sel);
            if original.is_null() || trace_method.is_null() || !added.as_bool() {
                issue_779_trace_log(format!(
                    "wezboard_appkit_dispatch boundary=nsapp_send_event outcome=install_failed original_null={} trace_null={} added={}",
                    original.is_null(),
                    trace_method.is_null(),
                    added.as_bool()
                ));
                return;
            }
            method_exchangeImplementations(original, trace_method);
            issue_779_trace_log(
                "wezboard_appkit_dispatch boundary=nsapp_send_event outcome=installed".to_string(),
            );
            issue_779_trace_log(
                "wezboard_appkit_dispatch boundary=local_event_monitor outcome=skipped reason=send_event_hooks_cover_required_boundary".to_string(),
            );
        }
    });
}

extern "C" fn application_should_terminate(
    _self: *mut AnyObject,
    _sel: Sel,
    _app: *mut AnyObject,
) -> u64 {
    log::debug!("application termination requested");
    unsafe {
        match config::configuration().window_close_confirmation {
            WindowCloseConfirmation::NeverPrompt => terminate_now(),
            WindowCloseConfirmation::AlwaysPrompt => {
                let ns_alert_cls = AnyClass::get(c"NSAlert").unwrap();
                let alert: *mut AnyObject = objc2::msg_send![ns_alert_cls, alloc];
                let alert: *mut AnyObject = objc2::msg_send![alert, init];
                let message_text = nsstring("Terminate Wezboard?");
                let info_text = nsstring("Detach and close all panes and terminate wezboard?");
                let cancel = nsstring("Cancel");
                let ok = nsstring("Ok");

                let () = objc2::msg_send![alert, setMessageText: Retained::as_ptr(&message_text)];
                let () = objc2::msg_send![alert, setInformativeText: Retained::as_ptr(&info_text)];
                let () = objc2::msg_send![alert, addButtonWithTitle: Retained::as_ptr(&cancel)];
                let () = objc2::msg_send![alert, addButtonWithTitle: Retained::as_ptr(&ok)];
                #[allow(non_upper_case_globals)]
                const NSModalResponseCancel: i64 = 1000;
                #[allow(non_upper_case_globals, dead_code)]
                const NSModalResponseOK: i64 = 1001;
                let result: i64 = objc2::msg_send![alert, runModal];
                log::info!("alert result is {result}");

                if result == NSModalResponseCancel {
                    NSApplicationTerminateReply::TerminateCancel.0 as u64
                } else {
                    terminate_now()
                }
            }
        }
    }
}

fn terminate_now() -> u64 {
    if let Some(conn) = Connection::get() {
        conn.terminate_message_loop();
    }
    NSApplicationTerminateReply::TerminateNow.0 as u64
}

extern "C" fn application_will_finish_launching(
    _self: *mut AnyObject,
    _sel: Sel,
    _notif: *mut AnyObject,
) {
    log::debug!("application_will_finish_launching");
}

extern "C" fn application_did_finish_launching(
    this: *mut AnyObject,
    _sel: Sel,
    _notif: *mut AnyObject,
) {
    log::debug!("application_did_finish_launching");
    #[allow(deprecated)]
    unsafe {
        *(&mut *this).get_mut_ivar::<Bool>("launched") = Bool::YES;
    }
}

extern "C" fn application_open_untitled_file(
    this: *mut AnyObject,
    _sel: Sel,
    _app: *mut AnyObject,
) -> Bool {
    #[allow(deprecated)]
    let launched: Bool = unsafe { *(&*this).get_ivar("launched") };
    log::debug!("application_open_untitled_file launched={launched:?}");
    if let Some(conn) = Connection::get() {
        if launched.as_bool() {
            conn.dispatch_app_event(ApplicationEvent::PerformKeyAssignment(
                KeyAssignment::SpawnWindow,
            ));
        }
        return Bool::YES;
    }
    Bool::NO
}

extern "C" fn wezboard_perform_key_assignment(
    _self: *mut AnyObject,
    _sel: Sel,
    menu_item: *mut AnyObject,
) {
    let menu_item = crate::os::macos::menu::MenuItem::with_menu_item(menu_item.cast());
    // Safe because wezboardPerformKeyAssignment: is only used with KeyAssignment
    let action = menu_item.get_represented_item();
    log::debug!("wezboard_perform_key_assignment {action:?}",);
    match action {
        Some(RepresentedItem::KeyAssignment(action)) => {
            if let Some(conn) = Connection::get() {
                conn.dispatch_app_event(ApplicationEvent::PerformKeyAssignment(action));
            }
        }
        None => {}
    }
}

extern "C" fn application_open_file(
    this: *mut AnyObject,
    _sel: Sel,
    _app: *mut AnyObject,
    file_name: *mut AnyObject,
) {
    #[allow(deprecated)]
    let launched: Bool = unsafe { *(&*this).get_ivar("launched") };
    if launched.as_bool() {
        let file_name = unsafe { nsstring_to_str(file_name) }.to_string();
        if let Some(conn) = Connection::get() {
            log::debug!("application_open_file {file_name}");
            conn.dispatch_app_event(ApplicationEvent::OpenCommandScript(file_name));
        }
    }
}

extern "C" fn application_dock_menu(
    _self: *mut AnyObject,
    _sel: Sel,
    _app: *mut AnyObject,
) -> *mut AnyObject {
    let dock_menu = Menu::new_with_title("");
    let new_window_item = MenuItem::new_with(
        "New Window",
        Some(objc2::sel!(wezboardPerformKeyAssignment:)),
        "",
    );
    new_window_item
        .set_represented_item(RepresentedItem::KeyAssignment(KeyAssignment::SpawnWindow));
    dock_menu.add_item(&new_window_item);
    dock_menu.autorelease()
}

fn get_class() -> &'static AnyClass {
    AnyClass::get(c"WezboardAppDelegate").unwrap_or_else(|| {
        let mut cls =
            ClassBuilder::new(c"WezboardAppDelegate", AnyClass::get(c"NSObject").unwrap())
                .expect("Unable to register application delegate class");

        cls.add_ivar::<Bool>(c"launched");

        unsafe {
            cls.add_method(
                objc2::sel!(applicationShouldTerminate:),
                application_should_terminate
                    as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) -> u64,
            );
            cls.add_method(
                objc2::sel!(applicationWillFinishLaunching:),
                application_will_finish_launching
                    as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );
            cls.add_method(
                objc2::sel!(applicationDidFinishLaunching:),
                application_did_finish_launching
                    as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );
            cls.add_method(
                objc2::sel!(application:openFile:),
                application_open_file
                    as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject, *mut AnyObject),
            );
            cls.add_method(
                objc2::sel!(applicationDockMenu:),
                application_dock_menu
                    as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) -> *mut AnyObject,
            );
            cls.add_method(
                objc2::sel!(wezboardPerformKeyAssignment:),
                wezboard_perform_key_assignment
                    as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );
            cls.add_method(
                objc2::sel!(applicationOpenUntitledFile:),
                application_open_untitled_file
                    as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) -> Bool,
            );
        }

        cls.register()
    })
}

pub fn create_app_delegate() -> anyhow::Result<Retained<AnyObject>> {
    let cls = get_class();
    unsafe {
        let delegate: *mut AnyObject = objc2::msg_send![cls, alloc];
        let delegate: *mut AnyObject = objc2::msg_send![delegate, init];
        // ObjC zeroes ivars on alloc, so `launched` is already false
        Retained::from_raw(delegate).ok_or_else(|| anyhow::anyhow!("AppDelegate init returned nil"))
    }
}
