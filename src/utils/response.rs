use actix_web::HttpResponse;
use serde::Serialize;

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub status_code: u16,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct MessageData {
    pub message: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            status: "success".to_string(),
            status_code: 200,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(status_code: u16, message: String) -> Self {
        Self {
            status: "error".to_string(),
            status_code,
            data: None,
            error: Some(message),
        }
    }
}

pub fn json_response<T: Serialize>(response: ApiResponse<T>) -> HttpResponse {
    HttpResponse::build(
        actix_web::http::StatusCode::from_u16(response.status_code)
            .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR),
    )
    .json(response)
}
