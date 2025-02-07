use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::function::FunctionCall;

/// Represents a prompt message with different roles.
///
/// This enum describes the various types of messages used in prompts. It supports user messages,
/// function messages, and assistant messages. Each variant holds message content.
#[derive(Debug, Clone)]
pub enum Message {
    /// A message sent by a user.
    User { content: Vec<MessageContext> },
    /// A message sent by a function, including its name.
    Function { name: String, content: Vec<MessageContext> },
    /// A message sent by an assistant.
    Assistant { content: Vec<MessageContext> },
}

// Custom serialization implementation for Message.
impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let state = match self {
            Message::User { content } => {
                let mut s = serializer.serialize_struct("Message", 2)?;
                s.serialize_field("role", "user")?;
                serialize_content_field(&mut s, content)?;
                s
            }
            Message::Function { name, content } => {
                let mut s = serializer.serialize_struct("Message", 3)?;
                s.serialize_field("role", "function")?;
                s.serialize_field("name", name)?;
                serialize_content_field(&mut s, content)?;
                s
            }
            Message::Assistant { content } => {
                let mut s = serializer.serialize_struct("Message", 2)?;
                s.serialize_field("role", "assistant")?;
                serialize_content_field(&mut s, content)?;
                s
            }
        };
        state.end()
    }
}

/// Helper function for serializing the "content" field of a message.
///
/// If the `content` vector has exactly one element and it is a text message, it serializes the
/// element directly. Otherwise, it serializes the entire vector.
fn serialize_content_field<S>(
    state: &mut S,
    content: &Vec<MessageContext>,
) -> Result<(), S::Error>
where
    S: SerializeStruct,
{
    if content.len() == 1 {
        if let MessageContext::Text(text) = &content[0] {
            state.serialize_field("content", text)?;
        } else {
            state.serialize_field("content", content)?;
        }
    } else {
        state.serialize_field("content", content)?;
    }
    Ok(())
}

// Custom deserialization implementation for Message.
impl<'de> Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;

        let role = value.get("role").and_then(Value::as_str).unwrap_or("");

        match role {
            "user" => {
                let content = serde_json::from_value(
                    value.get("content").cloned().unwrap_or_default(),
                )
                .map_err(serde::de::Error::custom)?;
                Ok(Message::User { content })
            }
            "function" => {
                let name = value
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| serde::de::Error::missing_field("name"))?
                    .to_string();
                let content = serde_json::from_value(
                    value.get("content").cloned().unwrap_or_default(),
                )
                .map_err(serde::de::Error::custom)?;
                Ok(Message::Function { name, content })
            }
            "assistant" => {
                let content = serde_json::from_value(
                    value.get("content").cloned().unwrap_or_default(),
                )
                .map_err(serde::de::Error::custom)?;
                Ok(Message::Assistant { content })
            }
            _ => Err(serde::de::Error::custom("Invalid message type")),
        }
    }
}

/// Represents a context within a message.
///
/// This enum supports either textual content or image content.
#[derive(Debug, Deserialize, Clone)]
pub enum MessageContext {
    /// A text message context.
    Text(String),
    /// An image message context.
    Image(MessageImage),
}

// Custom serialization implementation for MessageContext.
impl Serialize for MessageContext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            MessageContext::Text(text) => {
                let mut state = serializer.serialize_struct("MessageContext", 2)?;
                state.serialize_field("type", "text")?;
                state.serialize_field("text", text)?;
                state.end()
            }
            MessageContext::Image(image) => {
                let mut state = serializer.serialize_struct("MessageContext", 2)?;
                state.serialize_field("type", "image_url")?;
                state.serialize_field("image_url", image)?;
                state.end()
            }
        }
    }
}

/// Represents an image used within a message.
///
/// Contains a URL for the image and an optional detail representing the image resolution.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageImage {
    /// The image URL, which may be an HTTP URL or a base64-encoded data URI.
    ///
    /// For example:
    /// - "data:image/jpeg;base64,{IMAGE_DATA}"
    /// - "https://example.com/image.jpg"
    pub url: String,
    /// The resolution detail of the image.
    ///
    /// For example, for OpenAI API, valid values are:
    /// - "low"
    /// - "medium"
    /// - "auto" (default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Represents a choice from the API response.
///
/// A choice contains a response message and a finish reason.
#[derive(Debug, Deserialize)]
pub struct Choice {
    /// The message associated with this choice.
    pub message: ResponseMessage,
    /// The reason for finishing, as a string.
    pub finish_reason: String,
}

/// Represents a response message from the API.
///
/// Contains the role of the responder, optional text content, and an optional function call.
#[derive(Debug, Deserialize)]
pub struct ResponseMessage {
    /// The role of the message sender.
    pub role: String,
    /// The text content of the message (if any).
    pub content: Option<String>,
    /// An optional function call associated with the message.
    pub function_call: Option<FunctionCall>,
}