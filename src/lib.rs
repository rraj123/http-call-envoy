use log::{debug, warn};
use proxy_wasm::traits::*;
use proxy_wasm::types::*;
use std::time::Duration;

proxy_wasm::main! {{
    proxy_wasm::set_log_level(LogLevel::Debug);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> { Box::new(MyRootContext{}) });
}}

struct MyPlugin {
    auth_code: Option<String>,
}

impl MyPlugin {
    fn new() -> Self {
        Self { auth_code: None }
    }
}

impl HttpContext for MyPlugin {
    fn on_http_request_headers(&mut self, _num_headers: usize, _end_of_stream: bool) -> Action {
        // First, set the initial request headers
        self.set_http_request_header("version", Some("http-call-envoy-it-works"));

        if let Some(code) = &self.auth_code {
            self.set_http_request_header("auth-code", Some(code));
            debug!("Set auth-code request header to: {}", code);
        }

        match self.dispatch_http_call(
            "authservice",
            vec![
                (":method", "GET"),
                (":path", "/v1/auth/"),
                (":authority", "authservice"),
                (":scheme", "http"),
                ("content-type", "application/json"),
            ],
            None,
            vec![],
            Duration::from_secs(5),
        ) {
            Ok(_) => Action::Pause, // Changed to Pause to wait for the response
            Err(e) => {
                warn!("Failed to dispatch_http_call: {:?}", e);
                Action::Continue
            }
        }
    }

    fn on_http_response_headers(&mut self, _: usize, _: bool) -> Action {
        // Set response headers
        self.set_http_response_header("version", Some("http-call-envoy-it-works"));

        if let Some(code) = &self.auth_code {
            self.set_http_response_header("auth-code", Some(code));
            debug!("Set auth-code response header to: {}", code);
        }

        Action::Continue
    }
}

impl Context for MyPlugin {
    fn on_http_call_response(&mut self, _token_id: u32, _: usize, body_size: usize, _: usize) {
        if let Some(body) = self.get_http_call_response_body(0, body_size) {
            if let Ok(body_str) = String::from_utf8(body) {
                if let Some(code_start) = body_str.find("Generated auth code: ") {
                    let code = body_str[code_start + "Generated auth code: ".len()..].trim();
                    self.auth_code = Some(code.to_string());
                    debug!("Extracted auth code: {}", code);

                    // Add the auth code to request headers after receiving it
                    self.set_http_request_header("auth-code", Some(code));
                    debug!("Updated auth-code request header to: {}", code);
                }
            }
        }

        let headers = self.get_http_call_response_headers();
        debug!("on_http_call_response headers: {:?}", headers);

        self.resume_http_request(); // Resume the request after processing
    }
}
pub struct MyRootContext;

impl RootContext for MyRootContext {
    fn create_http_context(&self, _context_id: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(MyPlugin::new()))
    }

    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }
}

impl Context for MyRootContext {}
