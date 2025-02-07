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
