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
            Ok(_) => Action::Continue,
            Err(e) => {
                warn!("Failed to dispatch_http_call: {:?}", e);
                Action::Continue
            }
        }
    }

    fn on_http_response_headers(&mut self, _: usize, _: bool) -> Action {
        // Set the version header
        self.set_http_response_header("version", Some("http-call-envoy-it-works"));

        // If we have an auth code, set it in the response headers
        if let Some(code) = &self.auth_code {
            self.set_http_response_header("auth-code", Some(code));
            debug!("Set auth-code header to: {}", code);
        }

        Action::Continue
    }
}

impl Context for MyPlugin {
    fn on_http_call_response(&mut self, _token_id: u32, _: usize, body_size: usize, _: usize) {
        if let Some(body) = self.get_http_call_response_body(0, body_size) {
            if let Ok(body_str) = String::from_utf8(body) {
                // Try to parse the auth code from the response
                // Assuming response format is "Generated auth code: XXXXX"
                if let Some(code_start) = body_str.find("Generated auth code: ") {
                    let code = body_str[code_start + "Generated auth code: ".len()..].trim();
                    self.auth_code = Some(code.to_string());
                    debug!("Extracted auth code: {}", code);
                }
            }
        }

        let headers = self.get_http_call_response_headers();
        debug!("on_http_call_response headers: {:?}", headers);
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
