use serde::{Deserialize, Deserializer, Serialize};

/// function call の定義  
#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    #[serde(deserialize_with = "deserialize_arguments")]
    pub arguments: serde_json::Value,
}

fn deserialize_arguments<'de, D>(deserializer: D) -> Result<serde_json::Value, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    
    // 一度パース（StringからJSONのValueへ変換）
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&s);
    match parsed {
        Ok(value) => Ok(value),
        Err(_) => {
            // JSONが文字列として二重エスケープされている場合、もう一度パース
            let cleaned_s: String = serde_json::from_str(&s).map_err(serde::de::Error::custom)?;
            serde_json::from_str(&cleaned_s).map_err(serde::de::Error::custom)
        }
    }
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
