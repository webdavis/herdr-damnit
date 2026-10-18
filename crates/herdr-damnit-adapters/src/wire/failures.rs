use herdr_damnit_domain::ErrorDocument;
use serde::Deserialize;

#[derive(Deserialize)]
struct WireEnvelope {
    error: WireError,
}

#[derive(Deserialize)]
struct WireError {
    kind: String,
    message: String,
}

pub fn error_document(stderr: &str) -> Option<ErrorDocument> {
    let envelope: WireEnvelope = serde_json::from_str(stderr.trim()).ok()?;
    Some(ErrorDocument {
        store: envelope.error.kind == "store",
        message: envelope.error.message,
    })
}

#[cfg(test)]
mod tests;
