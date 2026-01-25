use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

use crate::filter_engine::FilterEngine;
use crate::rules::FilterAction;

/// HTTP proxy server for content filtering
pub struct WebProxy {
    filter_engine: Arc<FilterEngine>,
    ca_cert_path: Option<String>,
    ca_key_path: Option<String>,
}

impl WebProxy {
    /// Create a new web proxy with the given filter engine
    pub fn new(filter_engine: FilterEngine) -> Self {
        Self { filter_engine: Arc::new(filter_engine), ca_cert_path: None, ca_key_path: None }
    }

    /// Create a new web proxy with SSL interception enabled
    #[allow(dead_code)]
    pub fn with_ssl_intercept(
        filter_engine: FilterEngine,
        ca_cert_path: String,
        ca_key_path: String,
    ) -> Self {
        Self {
            filter_engine: Arc::new(filter_engine),
            ca_cert_path: Some(ca_cert_path),
            ca_key_path: Some(ca_key_path),
        }
    }

    /// Start the proxy server on the specified address and port
    pub async fn start(&self, bind_address: &str, port: u16) -> Result<()> {
        let addr: SocketAddr =
            format!("{}:{}", bind_address, port).parse().context("Failed to parse bind address")?;

        let listener = TcpListener::bind(addr).await.context("Failed to bind proxy server")?;

        info!("Web proxy server listening on {}", addr);

        loop {
            let (stream, peer_addr) =
                listener.accept().await.context("Failed to accept connection")?;

            debug!("New connection from {}", peer_addr);

            let filter_engine = Arc::clone(&self.filter_engine);
            let io = TokioIo::new(stream);
            let ca_cert_path = self.ca_cert_path.clone();
            let ca_key_path = self.ca_key_path.clone();

            tokio::task::spawn(async move {
                let service = service_fn(move |req| {
                    Self::handle_request(
                        Arc::clone(&filter_engine),
                        req,
                        ca_cert_path.clone(),
                        ca_key_path.clone(),
                    )
                });

                if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                    error!("Connection error: {}", err);
                }
            });
        }
    }

    /// Handle an incoming HTTP request
    async fn handle_request(
        filter_engine: Arc<FilterEngine>,
        req: Request<Incoming>,
        ca_cert_path: Option<String>,
        ca_key_path: Option<String>,
    ) -> Result<Response<BoxBody<Bytes, Infallible>>, hyper::Error> {
        debug!("Handling {} request to {}", req.method(), req.uri());

        match req.method() {
            &Method::CONNECT => {
                Self::handle_connect_request(filter_engine, req, ca_cert_path, ca_key_path).await
            }
            _ => Self::handle_http_request(filter_engine, req).await,
        }
    }

    /// Handle standard HTTP requests (GET, POST, etc.)
    async fn handle_http_request(
        filter_engine: Arc<FilterEngine>,
        req: Request<Incoming>,
    ) -> Result<Response<BoxBody<Bytes, Infallible>>, hyper::Error> {
        let uri = req.uri();
        let url = uri.to_string();
        let method = req.method().as_str();

        debug!("HTTP request: {} {}", method, url);

        // Check if the URL is allowed
        match filter_engine.evaluate_request(&url, method).await {
            Ok(decision) => match decision.action {
                FilterAction::Allow => {
                    debug!("Allowing request to {}", url);
                    Self::forward_request(req).await
                }
                FilterAction::Block => {
                    warn!("Blocking request to {} - {}", url, decision.reason);
                    Self::create_block_response(&url, &decision.reason, "blocked")
                }
                FilterAction::Warn => {
                    warn!("Warning for request to {} - {}", url, decision.reason);
                    // For now, treat warn as allow
                    Self::forward_request(req).await
                }
            },
            Err(err) => {
                error!("Error evaluating request to {}: {}", url, err);
                Self::create_error_response("Filter evaluation failed")
            }
        }
    }

    /// Handle CONNECT requests for HTTPS tunneling
    async fn handle_connect_request(
        filter_engine: Arc<FilterEngine>,
        req: Request<Incoming>,
        ca_cert_path: Option<String>,
        ca_key_path: Option<String>,
    ) -> Result<Response<BoxBody<Bytes, Infallible>>, hyper::Error> {
        let uri = req.uri();
        let host = uri.host().unwrap_or("unknown").to_string();
        let port = uri.port_u16().unwrap_or(443);

        debug!("CONNECT request to {}:{}", host, port);

        // Check if the domain is allowed
        let test_url = format!("https://{}:{}", host, port);
        match filter_engine.evaluate_request(&test_url, "CONNECT").await {
            Ok(decision) => match decision.action {
                FilterAction::Allow => {
                    debug!("Allowing CONNECT to {}", host);

                    // Establish the CONNECT tunnel
                    match Self::establish_tunnel(req, &host, port, ca_cert_path, ca_key_path).await
                    {
                        Ok(()) => {
                            // Tunnel established, but we don't return a response
                            // The tunnel handles everything from here
                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .body(Empty::<Bytes>::new().map_err(|never| match never {}).boxed())
                                .unwrap())
                        }
                        Err(e) => {
                            error!("Failed to establish tunnel to {}:{} - {}", host, port, e);
                            Self::create_error_response(&format!(
                                "Failed to connect to {}:{}",
                                host, port
                            ))
                        }
                    }
                }
                FilterAction::Block => {
                    warn!("Blocking CONNECT to {} - {}", host, decision.reason);
                    Self::create_block_response(&test_url, &decision.reason, "blocked")
                }
                FilterAction::Warn => {
                    warn!("Warning for CONNECT to {} - {}", host, decision.reason);
                    // For now, treat warn as allow
                    match Self::establish_tunnel(req, &host, port, ca_cert_path, ca_key_path).await
                    {
                        Ok(()) => Ok(Response::builder()
                            .status(StatusCode::OK)
                            .body(Empty::<Bytes>::new().map_err(|never| match never {}).boxed())
                            .unwrap()),
                        Err(e) => Self::create_error_response(&format!(
                            "Failed to connect to {}:{}",
                            host, e
                        )),
                    }
                }
            },
            Err(err) => {
                error!("Error evaluating CONNECT request to {}: {}", host, err);
                Self::create_error_response("Filter evaluation failed")
            }
        }
    }

    /// Forward an allowed HTTP request to the target server
    async fn forward_request(
        req: Request<Incoming>,
    ) -> Result<Response<BoxBody<Bytes, Infallible>>, hyper::Error> {
        let uri = req.uri().clone();
        let method = req.method().clone();
        let headers = req.headers().clone();

        // Create a reqwest client for forwarding the request
        let client = reqwest::Client::new();

        // Convert hyper request to reqwest request
        let url = uri.to_string();
        let body = match req.into_body().collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                error!("Failed to read request body: {}", e);
                return Self::create_error_response("Failed to read request body");
            }
        };

        let reqwest_method = match method.as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "HEAD" => reqwest::Method::HEAD,
            "PATCH" => reqwest::Method::PATCH,
            _ => {
                error!("Unsupported HTTP method: {}", method);
                return Self::create_error_response("Unsupported HTTP method");
            }
        };

        let mut builder = client.request(reqwest_method, &url);

        // Copy headers (skip host and connection headers)
        for (name, value) in headers.iter() {
            let name_str = name.as_str();
            if name_str != "host" && name_str != "connection" {
                if let Ok(value_str) = value.to_str() {
                    builder = builder.header(name_str, value_str);
                }
            }
        }

        if !body.is_empty() {
            builder = builder.body(body.to_vec());
        }

        // Execute the request
        match builder.send().await {
            Ok(response) => {
                debug!("Forwarded request to {} - status: {}", url, response.status());

                let status = StatusCode::from_u16(response.status().as_u16())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                let mut builder = Response::builder().status(status);

                // Copy response headers
                for (name, value) in response.headers().iter() {
                    if let Ok(value_str) = value.to_str() {
                        builder = builder.header(name.as_str(), value_str);
                    }
                }

                let body_bytes = match response.bytes().await {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        error!("Failed to read response body: {}", e);
                        return Self::create_error_response("Failed to read response body");
                    }
                };

                Ok(builder
                    .body(Full::new(body_bytes).map_err(|never| match never {}).boxed())
                    .unwrap())
            }
            Err(err) => {
                error!("Failed to forward request to {}: {}", url, err);
                Self::create_error_response("Failed to forward request")
            }
        }
    }

    /// Create a response for blocked content
    fn create_block_response(
        url: &str,
        reason: &str,
        category: &str,
    ) -> Result<Response<BoxBody<Bytes, Infallible>>, hyper::Error> {
        let block_page = Self::generate_block_page(url, reason, category);

        Ok(Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Content-Length", block_page.len())
            .body(Full::new(Bytes::from(block_page)).map_err(|never| match never {}).boxed())
            .unwrap())
    }

    /// Create an error response
    fn create_error_response(
        message: &str,
    ) -> Result<Response<BoxBody<Bytes, Infallible>>, hyper::Error> {
        let error_page = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Proxy Error</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; background: #f5f5f5; }}
        .container {{ background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }}
        .error {{ color: #d32f2f; }}
    </style>
</head>
<body>
    <div class="container">
        <h1 class="error">Proxy Error</h1>
        <p>{}</p>
        <p>Please contact your system administrator if this problem persists.</p>
    </div>
</body>
</html>"#,
            html_escape::encode_text(message)
        );

        Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Content-Length", error_page.len())
            .body(Full::new(Bytes::from(error_page)).map_err(|never| match never {}).boxed())
            .unwrap())
    }

    /// Generate a block page for filtered content
    fn generate_block_page(url: &str, reason: &str, category: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Content Blocked - DOTS Family Mode</title>
    <style>
        body {{ 
            font-family: Arial, sans-serif; 
            margin: 0; 
            padding: 0;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }}
        .container {{ 
            background: white; 
            padding: 40px; 
            border-radius: 12px; 
            box-shadow: 0 10px 25px rgba(0,0,0,0.2);
            max-width: 500px;
            text-align: center;
        }}
        .logo {{ 
            width: 64px; 
            height: 64px; 
            margin: 0 auto 20px auto; 
            background: #ff6b6b; 
            border-radius: 50%; 
            display: flex; 
            align-items: center; 
            justify-content: center;
            font-size: 32px;
        }}
        h1 {{ 
            color: #2c3e50; 
            margin: 0 0 10px 0; 
            font-size: 24px;
        }}
        .subtitle {{ 
            color: #7f8c8d; 
            margin: 0 0 30px 0; 
            font-size: 16px;
        }}
        .url {{ 
            background: #f8f9fa; 
            padding: 15px; 
            border-radius: 6px; 
            font-family: monospace; 
            word-break: break-all; 
            margin: 20px 0;
            color: #495057;
        }}
        .reason {{ 
            background: #fff3cd; 
            border: 1px solid #ffeaa7; 
            padding: 15px; 
            border-radius: 6px; 
            margin: 20px 0;
            color: #856404;
        }}
        .category {{ 
            display: inline-block; 
            background: #e74c3c; 
            color: white; 
            padding: 6px 12px; 
            border-radius: 20px; 
            font-size: 12px; 
            font-weight: bold; 
            text-transform: uppercase;
            margin: 10px 0;
        }}
        .footer {{ 
            margin-top: 30px; 
            padding-top: 20px; 
            border-top: 1px solid #ecf0f1; 
            color: #95a5a6; 
            font-size: 14px;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="logo">🛡️</div>
        <h1>Content Blocked</h1>
        <p class="subtitle">This page has been blocked by DOTS Family Mode</p>
        
        <div class="url">{}</div>
        
        <div class="reason">
            <strong>Reason:</strong> {}
        </div>
        
        <div class="category">{}</div>
        
        <div class="footer">
            <p>If you believe this page should be accessible, please contact your parent or guardian.</p>
            <p><small>DOTS Family Mode - Keeping families safe online</small></p>
        </div>
    </div>
</body>
</html>"#,
            html_escape::encode_text(url),
            html_escape::encode_text(reason),
            html_escape::encode_text(category)
        )
    }

    /// Establish a CONNECT tunnel to the target server
    async fn establish_tunnel(
        _req: Request<Incoming>,
        host: &str,
        port: u16,
        _ca_cert_path: Option<String>,
        _ca_key_path: Option<String>,
    ) -> Result<()> {
        let target_addr = format!("{}:{}", host, port);

        // Connect to the target server
        let _target_stream =
            TcpStream::connect(&target_addr).await.context("Failed to connect to target server")?;

        // In a real implementation we would upgrade the connection and tunnel traffic
        // But for this prototype we are simplifying

        debug!("Tunnel established to {}", target_addr);
        Ok(())
    }
}

/// Simple HTML escaping for user input
mod html_escape {
    pub fn encode_text(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FilterConfig;

    async fn create_test_engine() -> FilterEngine {
        let config = FilterConfig::default();
        FilterEngine::new(config).await.unwrap()
    }

    #[tokio::test]
    async fn test_web_proxy_creation() {
        let filter_engine = create_test_engine().await;
        let _proxy = WebProxy::new(filter_engine);
        // Just test that we can create a proxy instance
    }

    #[test]
    fn test_html_escaping() {
        let input = "<script>alert('xss')</script>";
        let escaped = html_escape::encode_text(input);
        assert_eq!(escaped, "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
    }

    #[test]
    fn test_block_page_generation() {
        let url = "https://example.com";
        let reason = "Adult content detected";
        let category = "adult";

        let page = WebProxy::generate_block_page(url, reason, category);

        assert!(page.contains("Content Blocked"));
        assert!(page.contains("example.com"));
        assert!(page.contains("Adult content detected"));
        assert!(page.contains("adult"));
    }
}
