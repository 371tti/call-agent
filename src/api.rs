use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};

use super::{prompt::{Choice, Message}, function::FunctionDef};

/// API Response Headers struct
#[derive(Debug)]
pub struct APIResponseHeaders {
    /// Retry-After header value (in seconds)
    pub retry_after: Option<u64>,
    /// X-RateLimit-Reset header value (timestamp or seconds)
    pub reset: Option<u64>,
    /// X-RateLimit-Remaining header value (number of remaining requests)
    pub rate_limit: Option<u64>,
    /// X-RateLimit-Limit header value (maximum allowed requests)
    pub limit: Option<u64>,

    /// Additional custom headers as key-value pairs
    pub extra_other: Vec<(String, String)>,
}

/// API Request structure for sending prompt and function information
#[derive(Debug, Deserialize)]
pub struct APIRequest {
    /// The model name to be used (e.g., "GPT-4o")
    pub model: String,
    /// Array of prompt messages
    pub messages: Vec<Message>,
    /// List of function definitions available for the prompt
    pub functions: Vec<FunctionDef>,
    /// Function call instruction:
    /// - "auto" to let the API decide the best function call
    /// - "none" for no function call
    /// - or an object like { "name": "get_weather" }
    pub function_call: serde_json::Value,
    /// Temperature value: a float from 0.0 and 1.0 controlling variability
    pub temperature: f64,
    /// Maximum number of tokens to use in the response
    pub max_tokens: u64,
    /// Top-p (nucleus sampling) parameter
    pub top_p: f64,
}

// Custom Serialize implementation for APIRequest
impl Serialize for APIRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize with capacity for potential optional fields
        let mut state = serializer.serialize_struct("APIRequest", 6)?;

        state.serialize_field("model", &self.model)?;
        state.serialize_field("messages", &self.messages)?;
        state.serialize_field("temperature", &self.temperature)?;
        state.serialize_field("max_tokens", &self.max_tokens)?;
        state.serialize_field("top_p", &self.top_p)?;

        // Serialize "functions" only if not empty
        if !self.functions.is_empty() {
            state.serialize_field("functions", &self.functions)?;
        }

        // Serialize "function_call" only if it is not equal to the string "none"
        if self.function_call != serde_json::Value::String("none".to_string()) {
            state.serialize_field("function_call", &self.function_call)?;
        }

        state.end()
    }
}

/// API Response structure from the server
#[derive(Debug, Deserialize)]
pub struct APIResponse {
    /// Array of choices (results) returned by the API
    pub choices: Option<Vec<Choice>>,
    /// Model name used in the response
    pub model: Option<String>,
    /// Type of the returned object (e.g., "chat.completion")
    pub object: Option<String>,
    /// Error information if the request failed
    pub error: Option<APIError>,
    /// Information regarding token usage
    pub usage: Option<APIUsage>,
}

/// API Error information structure
#[derive(Debug, Deserialize)]
pub struct APIError {
    /// Error message text
    pub message: String,
    /// Error type (renamed from "type" to avoid keyword conflict)
    #[serde(rename = "type")]
    pub err_type: String,
    /// Error code number
    pub code: i32,
}

/// API Usage information detailing token counts
#[derive(Debug, Deserialize)]
pub struct APIUsage {
    /// Number of tokens used in the prompt
    pub prompt_tokens: Option<u64>,
    /// Number of tokens used in the response
    pub completion_tokens: Option<u64>,
    /// Total number of tokens used (prompt + response)
    pub total_tokens: Option<u64>,
}