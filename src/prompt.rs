
use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::function::FunctionCall;
/// Promptのメッセージ構造体  
#[derive(Debug, Clone)]
pub enum Message {
    User { content: Vec<MessageContext> },
    Function { name: String, content: Vec<MessageContext> },
    Assistant { content: Vec<MessageContext> },
}

// カスタムシリアライズ実装
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

// `content` フィールドのシリアライズヘルパー関数
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

// カスタムデシリアライズ実装
impl<'de> Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;

        let role = value.get("role").and_then(Value::as_str).unwrap_or("");

        match role {
            "user" => {
                let content = serde_json::from_value(value.get("content").cloned().unwrap_or_default()).map_err(serde::de::Error::custom)?;
                Ok(Message::User { content })
            }
            "function" => {
                let name = value.get("name").and_then(Value::as_str).ok_or_else(|| serde::de::Error::missing_field("name"))?.to_string();
                let content = serde_json::from_value(value.get("content").cloned().unwrap_or_default()).map_err(serde::de::Error::custom)?;
                Ok(Message::Function { name, content })
            }
            "assistant" => {
                let content = serde_json::from_value(value.get("content").cloned().unwrap_or_default()).map_err(serde::de::Error::custom)?;
                Ok(Message::Assistant { content })
            }
            _ => Err(serde::de::Error::custom("Invalid message type")),
        }
    }
}
#[derive(Debug, Deserialize, Clone)]
pub enum MessageContext {
    Text(String),
    Image(MessageImage),
}

// カスタムシリアライズ実装
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageImage {
    /// 画像のURL  
    /// url or  
    /// base64  
    /// ex. "data:image/jpeg;base64,'{IMAGE_DATA}'"  
    /// ex. "https://example.com/image.jpg"  
    pub url: String,
    /// 画像の解像度  
    /// ex open ai api  
    /// - "low"  
    /// - "medium"  
    /// - "auto" (default)  
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}




#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: ResponseMessage,
    pub finish_reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ResponseMessage {
    pub role: String,
    pub content: Option<String>,
    pub function_call: Option<FunctionCall>,
}