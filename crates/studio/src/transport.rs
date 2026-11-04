//! Minimal HTTP seam so provider parsing/mapping is testable without network.

use std::io::Read;
use std::time::Duration;

pub enum Method {
    Get,
    Post,
}

pub enum Body {
    Empty,
    Json(serde_json::Value),
    Multipart {
        field: &'static str,
        file_name: String,
        mime: String,
        bytes: Vec<u8>,
    },
}

/// Deliberately not `Debug`: it carries the bearer credential.
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub bearer: Option<String>,
    pub body: Body,
    /// Use the long (download) timeout, e.g. for result streams that end when a job finishes.
    pub long_running: bool,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
    pub retry_after_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    Unreachable(String),
    Timeout,
    TooLarge,
    InvalidRequest(String),
}

pub trait Transport: Send + Sync {
    fn send(
        &self,
        request: HttpRequest,
        max_body_bytes: u64,
    ) -> Result<HttpResponse, TransportError>;
}

#[derive(Debug, Clone, Copy)]
pub struct HttpConfig {
    pub connect_timeout: Duration,
    pub api_timeout: Duration,
    pub download_timeout: Duration,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            api_timeout: Duration::from_secs(60),
            download_timeout: Duration::from_secs(600),
        }
    }
}

/// HTTPS-only blocking transport. Callers run it off the UI thread.
#[derive(Clone)]
pub struct ReqwestTransport {
    api: reqwest::blocking::Client,
    download: reqwest::blocking::Client,
}

impl ReqwestTransport {
    pub fn new(config: HttpConfig) -> Result<Self, TransportError> {
        let build = |timeout: Duration| {
            reqwest::blocking::Client::builder()
                .https_only(true)
                .connect_timeout(config.connect_timeout)
                .timeout(timeout)
                .user_agent(concat!("miyora-studio/", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|error| TransportError::InvalidRequest(error.without_url().to_string()))
        };
        Ok(Self {
            api: build(config.api_timeout)?,
            download: build(config.download_timeout)?,
        })
    }
}

impl Transport for ReqwestTransport {
    fn send(
        &self,
        request: HttpRequest,
        max_body_bytes: u64,
    ) -> Result<HttpResponse, TransportError> {
        let client = if request.bearer.is_some() && !request.long_running {
            &self.api
        } else {
            &self.download
        };
        let mut builder = match request.method {
            Method::Get => client.get(&request.url),
            Method::Post => client.post(&request.url),
        };
        if let Some(token) = &request.bearer {
            builder = builder.bearer_auth(token);
        }
        builder = match request.body {
            Body::Empty => builder,
            Body::Json(value) => builder.json(&value),
            Body::Multipart {
                field,
                file_name,
                mime,
                bytes,
            } => {
                let part = reqwest::blocking::multipart::Part::bytes(bytes)
                    .file_name(file_name)
                    .mime_str(&mime)
                    .map_err(|error| {
                        TransportError::InvalidRequest(error.without_url().to_string())
                    })?;
                builder.multipart(reqwest::blocking::multipart::Form::new().part(field, part))
            }
        };
        let response = builder.send().map_err(map_reqwest_error)?;
        let status = response.status().as_u16();
        if response
            .content_length()
            .is_some_and(|length| length > max_body_bytes)
        {
            return Err(TransportError::TooLarge);
        }
        let retry_after_secs = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok());
        let mut body = Vec::new();
        response
            .take(max_body_bytes.saturating_add(1))
            .read_to_end(&mut body)
            .map_err(|error| TransportError::Unreachable(error.kind().to_string()))?;
        if body.len() as u64 > max_body_bytes {
            return Err(TransportError::TooLarge);
        }
        Ok(HttpResponse {
            status,
            body,
            retry_after_secs,
        })
    }
}

fn map_reqwest_error(error: reqwest::Error) -> TransportError {
    if error.is_timeout() {
        TransportError::Timeout
    } else if error.is_builder() {
        TransportError::InvalidRequest(error.without_url().to_string())
    } else {
        // Signed CDN URLs can carry tokens, so never echo the URL.
        TransportError::Unreachable(error.without_url().to_string())
    }
}
