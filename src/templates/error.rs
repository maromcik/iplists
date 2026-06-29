use askama::Template;
use serde::Serialize;

#[derive(Template, Serialize)]
#[template(path = "error.html")]
pub struct GenericError {
    pub code: u16,
    pub status_code: String,
    pub description: String,
}
