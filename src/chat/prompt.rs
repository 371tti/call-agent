use std::fmt;

use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::function::FunctionCall;

/// Represents a prompt message with different roles.
///
/// This enum describes various types of messages used in prompts.
/// It supports user messages, function messages, and assistant messages.
/// Each variant holds the content of the message.
#[derive(Clone)]
pub enum Message {
    /// A message sent by a user.
    /// should the name matches the pattern '^[a-zA-Z0-9_-]+$'."
    User { 
        name: Option<String>,
        content: Vec<MessageContext> 
    },
    /// A message sent by a function, including its name.
    Tool { 
        tool_call_id: String,
        content: Vec<MessageContext> 
    },
    /// A message from the assistant.
    /// should the name matches the pattern '^[a-zA-Z0-9_-]+$'."
    Assistant { 
        name: Option<String>,
        content: Vec<MessageContext>, 
        tool_calls: Option<Vec<FunctionCall>>,
    },
    /// A system prompt.
    /// should the name matches the pattern '^[a-zA-Z0-9_-]+$'."
    System { 
        name: Option<String>,
        content: String
    },
    /// A message from the developer.
    /// Treated as a system message in unsupported models.
    /// should the name matches the pattern '^[a-zA-Z0-9_-]+$'."
    Developer { 
        name: Option<String>,
        content: String
    },
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::User { name, content } => {
                writeln!(f, "User: {}", name.as_deref().unwrap_or("Anonymous"))?;
                for ctx in content {
                    match ctx {
                        MessageContext::Text(text) => writeln!(f, "    {}", text)?,
                        MessageContext::Image(image) => writeln!(f, "    [Image URL: {}]", image.url)?,
                    }
                }
                Ok(())
            }
            Message::Tool { tool_call_id, content } => {
                writeln!(f, "Tool: {} - Tool Call", tool_call_id)?;
                for ctx in content {
                    match ctx {
                        MessageContext::Text(text) => writeln!(f, "    {}", text)?,
                        MessageContext::Image(image) => writeln!(f, "    [Image URL: {}]", image.url)?,
                    }
                }
                Ok(())
            }
            Message::Assistant { name, content, tool_calls } => {
                writeln!(f, "Assistant: {}", name.as_deref().unwrap_or("Assistant"))?;
                for ctx in content {
                    match ctx {
                        MessageContext::Text(text) => writeln!(f, "    {}", text)?,
                        MessageContext::Image(image) => writeln!(f, "    [Image URL: {}]", image.url)?,
                    }
                }
                if let Some(calls) = tool_calls {
                    for call in calls {
                        writeln!(f, "    Tool Call: {:?}", call)?;
                    }
                }
                Ok(())
            }
            Message::System { name, content } => {
                writeln!(f, "System: {}", name.as_deref().unwrap_or("System"))?;
                writeln!(f, "    {}", content)
            }
            Message::Developer { name, content } => {
                writeln!(f, "Developer: {}", name.as_deref().unwrap_or("Developer"))?;
                writeln!(f, "    {}", content)
            }
        }
    }
}

// Custom serialization implementation for Message.
impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let state = match self {
            Message::User { name, content } => {
                let mut s = serializer.serialize_struct("Message", 3)?;
                s.serialize_field("role", "user")?;
                if let Some(name) = name {
                    s.serialize_field("name", name)?;
                }
                serialize_content_field(&mut s, content)?;
                s
            }
            Message::Tool { tool_call_id, content } => {
                let mut s = serializer.serialize_struct("Message", 2)?;
                s.serialize_field("role", "tool")?;
                s.serialize_field("tool_call_id", tool_call_id)?;

                serialize_content_field(&mut s, content)?;
                s
            }
            Message::Assistant { name, content, tool_calls } => {
                let mut s = serializer.serialize_struct("Message", 3)?;
                s.serialize_field("role", "assistant")?;
                if let Some(name) = name {
                    s.serialize_field("name", name)?;
                }
                serialize_content_field(&mut s, content)?;
                if let Some(tool_calls) = tool_calls {
                    s.serialize_field("tool_calls", tool_calls)?;
                }
                s
            }
            Message::System { name, content } => {
                let mut s = serializer.serialize_struct("Message", 3)?;
                s.serialize_field("role", "system")?;
                if let Some(name) = name {
                    s.serialize_field("name", name)?;
                }
                s.serialize_field("content", content)?;
                s
            }
            Message::Developer { name, content } => {
                let mut s = serializer.serialize_struct("Message", 3)?;
                s.serialize_field("role", "developer")?;
                if let Some(name) = name {
                    s.serialize_field("name", name)?;
                }
                s.serialize_field("content", content)?;
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
            let name = value.get("name").and_then(Value::as_str).map(String::from);
            let content = serde_json::from_value(
                value.get("content").cloned().unwrap_or_default(),
            )
            .map_err(serde::de::Error::custom)?;
            Ok(Message::User { name, content })
            }
            "tool" => {
            let tool_call_id = value
                .get("tool_call_id")
                .and_then(Value::as_str)
                .ok_or_else(|| serde::de::Error::missing_field("tool_call_id"))?
                .to_string();
            let content = serde_json::from_value(
                value.get("content").cloned().unwrap_or_default(),
            )
            .map_err(serde::de::Error::custom)?;
            Ok(Message::Tool { tool_call_id, content })
            }
            "assistant" => {
                let name = value.get("name").and_then(Value::as_str).map(String::from);
                let content = serde_json::from_value(
                    value.get("content").cloned().unwrap_or_default(),
                )
                .map_err(serde::de::Error::custom)?;
                let tool_calls = value.get("tool_calls").map_or(Ok(None), |v| {
                    serde_json::from_value(v.clone()).map(Some)
                }).map_err(serde::de::Error::custom)?;
                Ok(Message::Assistant { name, content, tool_calls })
            }
            "system" => {
                let name = value.get("name").and_then(Value::as_str).map(String::from);
                let content = value
                    .get("content")
                    .and_then(Value::as_str)
                    .ok_or_else(|| serde::de::Error::missing_field("content"))?
                    .to_string();
                Ok(Message::System { name, content })
            }
            "developer" => {
                let name = value.get("name").and_then(Value::as_str).map(String::from);
                let content = value
                    .get("content")
                    .and_then(Value::as_str)
                    .ok_or_else(|| serde::de::Error::missing_field("content"))?
                    .to_string();
                Ok(Message::Developer { name, content })
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
#[derive(Debug, Deserialize, Clone)]
pub struct Choice {
    /// The index of the choice in the response.
    pub index: usize,

    /// The message associated with this choice.
    pub message: ResponseMessage,

    /// The reason for finishing, as a string.
    pub finish_reason: String,
}

/// Represents a response message from the API.
///
/// Contains the role of the responder, optional text content, and an optional function call.
#[derive(Debug, Deserialize, Clone)]
pub struct ResponseMessage {
    /// The role of the message sender.
    pub role: String,
    
    /// The text content of the message (if any).
    pub content: Option<String>,

    /// An optional function call associated with the message.
    pub tool_calls: Option<Vec<FunctionCall>>,

    /// An optional refusal message.
    pub refusal: Option<String>,

    /// annotation for web search options
    #[serde(default)]
    pub annotations: Option<serde_json::Value>
}