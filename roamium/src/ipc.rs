use std::ffi::c_void;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::Mutex;

use prost::Message;

use crate::ffi;
use crate::proto::TermSurfMessage;

/// Global write half of the socket. Callbacks write from the main thread;
/// the reader thread uses a separate clone.
static WRITER: Mutex<Option<UnixStream>> = Mutex::new(None);

/// Connect to the GUI's Unix socket. Returns the read half for the reader
/// thread and stores the write half in WRITER.
pub fn connect(path: &str) -> Option<UnixStream> {
    let stream = match UnixStream::connect(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[Roamium] connect failed: {e}");
            return None;
        }
    };
    let reader = stream.try_clone().ok()?;
    *WRITER.lock().unwrap() = Some(stream);
    Some(reader)
}

/// Send a protobuf message over the socket (4-byte LE length prefix).
pub fn send(msg: &TermSurfMessage) {
    let payload = msg.encode_to_vec();
    let len = (payload.len() as u32).to_le_bytes();
    if let Some(ref mut stream) = *WRITER.lock().unwrap() {
        let _ = stream.write_all(&len);
        let _ = stream.write_all(&payload);
    }
}

/// Reader loop: runs on a background thread. Reads length-prefixed protobuf
/// messages and posts each to the UI thread via ts_post_task.
pub fn reader_loop(mut stream: UnixStream) {
    let mut buf = Vec::with_capacity(4096);
    let mut tmp = [0u8; 4096];

    loop {
        let n = match stream.read(&mut tmp) {
            Ok(0) => {
                eprintln!("[Roamium] socket EOF — requesting quit");
                unsafe { ffi::ts_post_task(Some(quit_trampoline), std::ptr::null_mut()) };
                return;
            }
            Ok(n) => n,
            Err(e) => {
                eprintln!("[Roamium] socket read error: {e} — requesting quit");
                unsafe { ffi::ts_post_task(Some(quit_trampoline), std::ptr::null_mut()) };
                return;
            }
        };
        buf.extend_from_slice(&tmp[..n]);

        while buf.len() >= 4 {
            let msg_len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
            if buf.len() < 4 + msg_len {
                break;
            }

            let payload = &buf[4..4 + msg_len];
            if let Ok(msg) = TermSurfMessage::decode(payload) {
                // Box the message and post to the UI thread.
                let ptr = Box::into_raw(Box::new(msg)) as *mut c_void;
                unsafe {
                    ffi::ts_post_task(Some(dispatch_trampoline), ptr);
                }
            }
            buf.drain(..4 + msg_len);
        }
    }
}

/// Trampoline called on the UI thread to quit the browser process.
unsafe extern "C" fn quit_trampoline(_data: *mut c_void) {
    unsafe { ffi::ts_quit() };
}

/// Trampoline called on the UI thread by ts_post_task.
unsafe extern "C" fn dispatch_trampoline(data: *mut c_void) {
    let msg = unsafe { Box::from_raw(data as *mut TermSurfMessage) };
    crate::dispatch::handle_message(&msg);
}
