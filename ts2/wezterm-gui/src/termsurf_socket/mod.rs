//! Unix domain socket server for CLI-to-GUI communication.
//! Creates a socket at `/tmp/termsurf-{pid}.sock` and listens for connections.

mod connection;
pub mod protocol;

use connection::TermsurfConnection;
use mux::pane::PaneId;
use mux::Mux;
use protocol::{TermsurfEvent, TermsurfRequest, TermsurfResponse};
use serde_json::json;
use std::collections::HashMap;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock, RwLock, Weak};
use std::thread;

/// Global socket server instance
static SERVER: OnceLock<Arc<TermsurfSocketServer>> = OnceLock::new();

/// Get the global socket server instance
pub fn get_server() -> Option<Arc<TermsurfSocketServer>> {
    SERVER.get().cloned()
}

/// Start the global socket server
pub fn start_server() -> anyhow::Result<()> {
    let server = TermsurfSocketServer::new()?;
    let server = Arc::new(server);

    // Store globally
    SERVER
        .set(server.clone())
        .map_err(|_| anyhow::anyhow!("Socket server already started"))?;

    // Set environment variable for child processes
    std::env::set_var("TERMSURF_SOCKET", server.socket_path());

    // Start accept loop in background thread
    server.start();

    Ok(())
}

/// Unix domain socket server for TermSurf CLI communication
pub struct TermsurfSocketServer {
    socket_path: PathBuf,
    listener: UnixListener,
    connections: RwLock<HashMap<String, Arc<TermsurfConnection>>>,
    running: Mutex<bool>,
    /// Maps browser_id to the connection that created it (weak ref to avoid cycles)
    browser_connections: RwLock<HashMap<String, Weak<TermsurfConnection>>>,
    /// Maps browser_id to request_id for event correlation
    browser_request_ids: RwLock<HashMap<String, String>>,
}

impl TermsurfSocketServer {
    /// Create a new socket server
    fn new() -> anyhow::Result<Self> {
        let pid = std::process::id();
        let socket_path = PathBuf::from(format!("/tmp/termsurf-{}.sock", pid));

        // Remove existing socket file if present
        if socket_path.exists() {
            std::fs::remove_file(&socket_path)?;
        }

        // Create and bind the socket
        let listener = UnixListener::bind(&socket_path)?;

        // Set socket permissions (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))?;
        }

        log::info!("[TermsurfSocket] Server created at {:?}", socket_path);

        Ok(Self {
            socket_path,
            listener,
            connections: RwLock::new(HashMap::new()),
            running: Mutex::new(false),
            browser_connections: RwLock::new(HashMap::new()),
            browser_request_ids: RwLock::new(HashMap::new()),
        })
    }

    /// Get the socket path
    pub fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    /// Start accepting connections in a background thread
    fn start(self: &Arc<Self>) {
        let mut running = self.running.lock().unwrap();
        if *running {
            log::warn!("[TermsurfSocket] Server already running");
            return;
        }
        *running = true;
        drop(running);

        let server = self.clone();
        thread::spawn(move || {
            server.accept_loop();
        });

        log::info!("[TermsurfSocket] Server started");
    }

    /// Accept loop - runs in background thread
    fn accept_loop(self: &Arc<Self>) {
        log::info!("[TermsurfSocket] Accept loop started");

        for stream in self.listener.incoming() {
            if !*self.running.lock().unwrap() {
                break;
            }

            match stream {
                Ok(stream) => {
                    let conn = Arc::new(TermsurfConnection::new(stream));
                    let conn_id = conn.id().to_string();

                    // Store connection
                    self.connections
                        .write()
                        .unwrap()
                        .insert(conn_id.clone(), conn.clone());

                    // Handle connection in a new thread
                    let server = self.clone();
                    let conn_id_for_handler = conn_id.clone();
                    thread::spawn(move || {
                        conn.read_loop(|conn, request| {
                            server.handle_request(conn, &conn_id_for_handler, request);
                        });

                        // Remove connection when done
                        server.connections.write().unwrap().remove(&conn_id);
                    });
                }
                Err(e) => {
                    log::error!("[TermsurfSocket] Accept error: {}", e);
                }
            }
        }

        log::info!("[TermsurfSocket] Accept loop ended");
    }

    /// Handle a request from a client
    fn handle_request(&self, conn: &TermsurfConnection, conn_id: &str, request: TermsurfRequest) {
        let response = match request.action.as_str() {
            "ping" => TermsurfResponse::ok(request.id.clone(), Some(json!({"pong": true}))),
            "open" => self.handle_open(conn, conn_id, &request),
            "close" => self.handle_close(&request),
            _ => TermsurfResponse::error(
                request.id.clone(),
                format!("Unknown action: {}", request.action),
            ),
        };

        if let Err(e) = conn.send_response(&response) {
            log::error!("[TermsurfSocket] Failed to send response: {}", e);
        }
    }

    /// Validate profile name: lowercase alphanumeric, must start with letter
    fn validate_profile_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Profile name cannot be empty".to_string());
        }
        if !name.chars().next().unwrap().is_ascii_lowercase() {
            return Err("Profile name must start with a lowercase letter".to_string());
        }
        if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
            return Err("Profile name must contain only lowercase letters and digits".to_string());
        }
        Ok(())
    }

    /// Handle "open" action - open a URL in a browser pane
    fn handle_open(
        &self,
        conn: &TermsurfConnection,
        conn_id: &str,
        request: &TermsurfRequest,
    ) -> TermsurfResponse {
        let url = match request.get_string("url") {
            Some(url) => url.to_string(),
            None => {
                return TermsurfResponse::error(
                    request.id.clone(),
                    "Missing 'url' in data".to_string(),
                )
            }
        };

        let pane_id = match request.pane_id {
            Some(id) => id as PaneId,
            None => {
                return TermsurfResponse::error(
                    request.id.clone(),
                    "Missing 'pane_id'".to_string(),
                )
            }
        };

        // Extract incognito flag (defaults to false)
        let incognito = request
            .data
            .as_ref()
            .and_then(|d| d.get("incognito"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Extract and validate profile (only if not incognito)
        let profile = if incognito {
            None
        } else {
            let profile_name = request
                .data
                .as_ref()
                .and_then(|d| d.get("profile"))
                .and_then(|v| v.as_str())
                .unwrap_or("default");

            if let Err(e) = Self::validate_profile_name(profile_name) {
                return TermsurfResponse::error(request.id.clone(), e);
            }

            Some(profile_name.to_string())
        };

        // Validate pane exists
        let mux = Mux::get();
        if mux.get_pane(pane_id).is_none() {
            return TermsurfResponse::error(
                request.id.clone(),
                format!("Pane {} not found", pane_id),
            );
        }

        // Generate browser_id for this browser instance
        let browser_id = format!("browser-{}", pane_id);

        // Look up the Arc<TermsurfConnection> and register the browser
        if let Some(conn_arc) = self.connections.read().unwrap().get(conn_id).cloned() {
            self.register_browser(browser_id.clone(), conn_arc, request.id.clone());
        }

        // Subscribe connection to events for this pane (legacy, kept for compatibility)
        conn.subscribe_to_pane(pane_id);

        // Notify GUI to open browser (using existing MuxNotification)
        mux.notify(mux::MuxNotification::WebOpen {
            pane_id,
            url: url.clone(),
            browser_id: browser_id.clone(),
            profile: profile.clone(),
            incognito,
        });

        TermsurfResponse::ok(
            request.id.clone(),
            Some(json!({
                "message": format!("Opening {}", url),
                "browser_id": browser_id
            })),
        )
    }

    /// Handle "close" action - close browser in a pane
    fn handle_close(&self, request: &TermsurfRequest) -> TermsurfResponse {
        let pane_id = match request.pane_id {
            Some(id) => id as PaneId,
            None => {
                return TermsurfResponse::error(
                    request.id.clone(),
                    "Missing 'pane_id'".to_string(),
                )
            }
        };

        // Notify GUI to close browser
        let mux = Mux::get();
        mux.notify(mux::MuxNotification::WebClosed { pane_id });

        TermsurfResponse::ok(request.id.clone(), None)
    }

    /// Broadcast an event to all connections subscribed to a pane
    pub fn broadcast_event(&self, pane_id: PaneId, event: &TermsurfEvent) {
        let connections = self.connections.read().unwrap();
        for conn in connections.values() {
            if let Err(e) = conn.send_event(event, pane_id) {
                log::error!("[TermsurfSocket] Failed to send event: {}", e);
            }
        }
    }

    /// Register a browser with its creating connection
    pub fn register_browser(
        &self,
        browser_id: String,
        connection: Arc<TermsurfConnection>,
        request_id: String,
    ) {
        log::info!(
            "[TermsurfSocket] Registering browser {} with request {}",
            browser_id,
            request_id
        );
        self.browser_connections
            .write()
            .unwrap()
            .insert(browser_id.clone(), Arc::downgrade(&connection));
        self.browser_request_ids
            .write()
            .unwrap()
            .insert(browser_id, request_id);
    }

    /// Send event to the connection that created a browser
    pub fn send_browser_event(
        &self,
        browser_id: &str,
        event_type: &str,
        data: serde_json::Value,
    ) {
        let conn = self
            .browser_connections
            .read()
            .unwrap()
            .get(browser_id)
            .and_then(|w| w.upgrade());
        let request_id = self
            .browser_request_ids
            .read()
            .unwrap()
            .get(browser_id)
            .cloned();

        if let (Some(conn), Some(request_id)) = (conn, request_id) {
            let event = TermsurfEvent::new(request_id, event_type.to_string(), Some(data));
            if let Err(e) = conn.send_event_direct(&event) {
                log::error!("[TermsurfSocket] Failed to send browser event: {}", e);
            }
        } else {
            log::debug!(
                "[TermsurfSocket] No connection found for browser {} (may have disconnected)",
                browser_id
            );
        }
    }

    /// Unregister a browser when it closes
    pub fn unregister_browser(&self, browser_id: &str) {
        log::info!("[TermsurfSocket] Unregistering browser {}", browser_id);
        self.browser_connections
            .write()
            .unwrap()
            .remove(browser_id);
        self.browser_request_ids.write().unwrap().remove(browser_id);
    }
}

impl Drop for TermsurfSocketServer {
    fn drop(&mut self) {
        *self.running.lock().unwrap() = false;

        // Remove socket file
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }

        log::info!("[TermsurfSocket] Server stopped");
    }
}
