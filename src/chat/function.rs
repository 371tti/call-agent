use std::fmt;

use serde::{de::{self, Visitor}, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

/// function call の定義  
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolDef {
    /// ツールの種類:  
    /// 現在 "function" のみサポートされています  
    /// 固定値: "function"  
    #[serde(rename = "type")]
    pub tool_type: String,
    /// ツールの定義  
    /// ツールの関数名、説明、パラメータの定義を含みます  
    pub function: FunctionDef,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionDef {
    /// 関数名  
    pub name: String,
    /// 関数の説明  
    pub description: String,
    /// 関数のパラメータの定義(json schema)  
    /// enumなどの制約を指定することが推奨されます  
    /// 機能は明確で直感的であるべきです  
    pub parameters: serde_json::Value,
    /// 厳密に構造化します  
    /// default: false
    /// 並列ToolCallsでは強制無効化されます  
    pub strict: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FunctionCall {
    /// ツールの呼び出しID  
    /// ツールの呼び出しを一意に識別するためのID  
    /// 返却時にこのIDを指定する必要があります  
    pub id: String,
    /// ツールの種類:  
    /// 現在 "function" のみサポートされています  
    /// 固定値: "function"  
    #[serde(rename = "type", alias = "type")]
    pub tool_type: String,
    /// 関数の呼び出し内容  
    pub function: FunctionCallInner,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FunctionCallInner {
    /// 関数名  
    /// 呼び出された関数の名前  
    pub name: String,
    /// 関数の引数  
    /// JSONとして提供されます  
    /// 例: {"input": "Hello, world!"}  
    #[serde(deserialize_with = "deserialize_arguments",serialize_with = "serialize_arguments")]
    pub arguments: Value,
}

fn deserialize_arguments<'de, D>(deserializer: D) -> Result<Value, D::Error>
where
    D: Deserializer<'de>,
{
    struct ArgumentsVisitor;

    impl<'de> Visitor<'de> for ArgumentsVisitor {
        type Value = Value;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a JSON string or object representing the function arguments")
        }

        // 文字列として渡された場合
        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            // まず、文字列をJSONとしてパースを試みる
            serde_json::from_str(value)
                .or_else(|_| Ok(Value::String(value.to_owned())))
                .map_err(|e| de::Error::custom::<String>(e))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            serde_json::from_str(&value)
                .or_else(|_| Ok(Value::String(value)))
                .map_err(|e| de::Error::custom::<String>(e))
        }

        // 既にオブジェクト（マップ）として渡された場合
        fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
        where
            M: de::MapAccess<'de>,
        {
            Value::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(ArgumentsVisitor)
}

fn serialize_arguments<S>(value: &Value, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Value を JSON 文字列に変換する
    let s = value.to_string();
    serializer.serialize_str(&s)
}


/// toolの定義  
/// The Tool trait defines the interface for executable tools within the crate.  
/// Implementers of this trait must provide concrete definitions for:
/// 
/// - Identifying the tool via a unique name (used as the function or tool identifier).
/// - Describing the tool's functionality in plain text.
/// - Defining the expected input parameters in the form of a JSON schema.
/// - Executing the tool's operation with the provided JSON parameters, returning either a result string on success or an error string on failure.
///
/// # Methods
///
/// - def_name()
///   - Returns the unique name of the tool as a string slice.
///   - This name acts as the primary identifier when selecting or referencing the tool.
///
/// - def_description()
///   - Provides a concise description of what the tool does.
///   - Intended to offer an overview of the tool's purpose and behavior.
///
/// - def_parameters()
///   - Returns a JSON value representing the input parameters' schema.
///   - The schema should detail the expected keys and value types, ensuring consumers provide input adhering to the specification.
///
/// - run(args: serde_json::Value)
///   - Executes the tool's functionality using the provided JSON arguments.
///   - Returns a Result containing a string on success or an error description string on failure.
///
/// # Example
///
/// ```rust
/// // Assuming MyTool implements the Tool trait:
/// struct MyTool;
/// 
/// impl Tool for MyTool {
///     fn def_name(&self) -> &str {
///         "my_tool"
///     }
/// 
///     fn def_description(&self) -> &str {
///         "Performs a specific operation on the provided data."
///     }
/// 
///     fn def_parameters(&self) -> serde_json::Value {
///         serde_json::json!({
///             "type": "object",
///             "properties": {
///                 "input": { "type": "string", "description": "Input data for the tool" }
///             },
///             "required": ["input"]
///         })
///     }
/// 
///     fn run(&self, args: serde_json::Value) -> Result<String, String> {
///         // Execute the tool's operation based on the provided arguments.
///         Ok("Operation completed successfully".to_string())
///     }
/// }
/// ```
///
/// # Error Handling
///
/// - The run() method returns an Err variant with a descriptive error message if the execution fails.
pub trait Tool {
    /// 関数名  
    /// ツール名として使用される  
    fn def_name(&self) -> &str;
    /// 関数の説明  
    fn def_description(&self) -> &str;
    /// 関数のパラメータの定義(json schema)  
    fn def_parameters(&self) -> serde_json::Value;
    /// 関数の実行  
    fn run(&self, args: serde_json::Value) -> Result<String, String>;
}
