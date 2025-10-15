/// Error handling module
///
/// Provides unified error responses
use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    Internal {
        reason: String,
    },
    BadRequest {
        missing: Vec<String>,
        reason: Option<String>,
    },
    NotFound {
        resource: String,
    },
    ServiceUnavailable {
        details: String,
    },
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing: Option<Vec<String>>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Internal { reason } => write!(f, "Internal error: {}", reason),
            ApiError::BadRequest { missing, reason } => {
                write!(f, "Bad request: {:?}, {:?}", missing, reason)
            }
            ApiError::NotFound { resource } => write!(f, "Not found: {}", resource),
            ApiError::ServiceUnavailable { details } => {
                write!(f, "Service unavailable: {}", details)
            }
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ApiError::NotFound { .. } => StatusCode::NOT_FOUND,
            ApiError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let response = match self {
            ApiError::Internal { reason } => ErrorResponse {
                error: "Internal server error".to_string(),
                details: Some(reason.clone()),
                missing: None,
            },
            ApiError::BadRequest { missing, reason } => ErrorResponse {
                error: "Bad request".to_string(),
                details: reason.clone(),
                missing: if missing.is_empty() {
                    None
                } else {
                    Some(missing.clone())
                },
            },
            ApiError::NotFound { resource } => ErrorResponse {
                error: format!("{} not found", resource),
                details: None,
                missing: None,
            },
            ApiError::ServiceUnavailable { details } => ErrorResponse {
                error: "Service unavailable".to_string(),
                details: Some(details.clone()),
                missing: None,
            },
        };
        HttpResponse::build(status).json(response)
    }
}
