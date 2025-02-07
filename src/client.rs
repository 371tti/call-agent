use std::{collections::HashMap, sync::Arc};

use reqwest::Client;

use super::{api::{APIRequest, APIResponse, APIResponseHeaders}, err::ClientError, function::{FunctionDef, Tool}, prompt::{Message, MessageContext}};

/// ルート構造体  
pub struct OpenAIClient {
    pub client: Client,
    pub end_point: String,
    pub api_key: Option<String>,
    pub tools: HashMap<String, (Arc<dyn Tool + Send + Sync>, bool)>,
}

pub struct OpenAIClientState<'a> {
    pub prompt: Vec<Message>,
    pub client: &'a OpenAIClient,
}

pub struct ModelConfig {
    pub model: String,
    pub temp: Option<f64>,
    pub max_token: Option<u64>,
    pub top_p: Option<f64>,
}

#[derive(Debug)]
pub struct APIResult {
    pub response: APIResponse,
    pub headers: APIResponseHeaders,
}

impl OpenAIClient {
    /// OpenAIClientを生成  
    /// end_point: OpenAI APIのエンドポイント  
    /// api_key: OpenAI APIのAPIキー  
    pub fn new(end_point: &str,api_key: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            end_point: end_point.trim_end_matches('/').to_string(),
            api_key: api_key.map(|s| s.to_string()),
            tools: HashMap::new(),
        }
    }

    /// ツールを登録  
    /// tool: ツールへの参照  
    /// T: Toolを実装した型  
    /// ツール名が重複している場合は上書きされる
    pub fn def_tool<T: Tool + Send + Sync + 'static>(&mut self, tool: Arc<T>) {
        self.tools.insert(tool.def_name().to_string(), (tool, true));
    }

    /// ツールの一覧を取得  
    /// return: (ツール名, ツールの説明, 有効/無効)のリスト
    pub fn list_tools(&self) -> Vec<(String, String, bool)> {
        let mut tools = Vec::new();
        for (tool_name, (tool, enable)) in self.tools.iter() {
            tools.push((tool_name.to_string(), tool.def_description().to_string(), *enable));
        }
        tools
    }

    /// ツールの有効/無効を切り替え  
    /// tool_name: ツール名  
    /// t_enable: 有効/無効  
    pub fn switch_tool(&mut self, tool_name: &str, t_enable: bool) {
        if let Some((_, enable)) = self.tools.get_mut(tool_name) {
            *enable = t_enable;
        }
    }

    /// ツールの定義をエクスポート  
    pub fn export_tool_def(&self) -> Vec<FunctionDef> {
        let mut defs = Vec::new();
        for (tool_name, (tool, enable)) in self.tools.iter() {
            if *enable {
                defs.push(FunctionDef {
                    name: tool_name.to_string(),
                    description: tool.def_description().to_string(),
                    parameters: tool.def_parameters(),
                });
            }
        }
        defs
    }

    pub async fn send(&self, model: &ModelConfig, prompt: &Vec<Message>) -> Result<APIResult, ClientError> {
        match self.call_api(&model.model, prompt, None, model.temp, model.max_token, model.top_p).await {
            Ok(res) => {
                return Ok(res);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    pub async fn send_use_tool(&self, model: &ModelConfig, prompt: &Vec<Message>) -> Result<APIResult, ClientError> {
        match self.call_api(&model.model, prompt, Some(&serde_json::Value::String("auto".to_string())), model.temp, model.max_token, model.top_p).await {
            Ok(res) => {
                return Ok(res);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    pub async fn send_with_tool(&self, model: &ModelConfig, prompt: &Vec<Message>, tool_name: &str) -> Result<APIResult, ClientError> {
        let function_call = serde_json::json!({
            "name": tool_name,
        });

        match self.call_api(&model.model, prompt, Some(&function_call), model.temp, model.max_token, model.top_p).await {
            Ok(res) => {
                return Ok(res);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    /// APIを呼び出す  
    /// model: モデル名  
    /// - "GPT-4o"
    /// prompt: プロンプト  
    /// function_call: 関数呼び出し
    /// - "auto"  
    /// - "none"  
    /// - { "name": "get_weather" }  
    pub async fn call_api(
        &self, 
        model: &str, 
        prompt: &Vec<Message>, 
        function_call: Option<&serde_json::Value>, 
        temp: Option<f64>, max_token: Option<u64>, 
        top_p: Option<f64>) 
        -> Result<APIResult, ClientError> 
        {
        let url = format!("{}/chat/completions", self.end_point);
        if !url.starts_with("https://") && !url.starts_with("http://") {
            return Err(ClientError::InvalidEndpoint);
        }

        let request = APIRequest {
            model: model.to_string(),
            messages: prompt.clone(),
            functions: self.export_tool_def(),
            function_call: function_call.unwrap_or(&serde_json::Value::String("none".to_string())).clone(),
            temperature: temp.unwrap_or(0.5),
            max_tokens: max_token.unwrap_or(4000),
            top_p: top_p.unwrap_or(1.0),
        };

        let res = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("authorization", format!("Bearer {}", self.api_key.as_deref().unwrap_or("")))
            .json(&request)
            .send()
            .await.map_err(|_| ClientError::NetworkError)?;

        let headers = APIResponseHeaders {
            retry_after: res.headers().get("Retry-After").and_then(|v| v.to_str().ok().and_then(|v| v.parse().ok())),
            reset: res.headers().get("X-RateLimit-Reset").and_then(|v| v.to_str().ok().and_then(|v| v.parse().ok())),
            rate_limit: res.headers().get("X-RateLimit-Remaining").and_then(|v| v.to_str().ok().and_then(|v| v.parse().ok())),
            limit: res.headers().get("X-RateLimit-Limit").and_then(|v| v.to_str().ok().and_then(|v| v.parse().ok())),
            extra_other: res.headers().iter().map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string())).collect(),
        };
        let text = res.text().await.map_err(|_| ClientError::InvalidResponse)?;

        let response_body: APIResponse = serde_json::from_str(&text).map_err(|_| ClientError::InvalidResponse)?;

        Ok(APIResult {
            response: response_body,
            headers,
        })
    }

    /// プロンプトを生成  
    pub fn create_prompt(&self) -> OpenAIClientState {
        OpenAIClientState {
            prompt: Vec::new(),
            client: self,
        }
    }
}

impl<'a> OpenAIClientState<'a> {
    /// メッセージを追加  
    /// messages: メッセージのリスト  
    /// return: self  
    pub async fn add(&mut self, messages: Vec<Message>) -> &mut Self {
        self.prompt.extend(messages);
        self
    }

    /// メッセージをクリア  
    /// return: self
    pub async fn clear(&mut self) -> &mut Self {
        self.prompt.clear();
        self
    }

    /// 最後のメッセージを取得  
    pub async fn last(&mut self) -> Option<&Message> {
        self.prompt.last()
    }


    /// AIが応答を生成
    pub async fn generate(&mut self, model: &ModelConfig) -> Result<APIResult, ClientError> {
        let result = self.client.send(model, &self.prompt).await?;
        let choices = result.response.choices.as_ref().ok_or(ClientError::InvalidResponse)?;
        let choice = choices.get(0).ok_or(ClientError::InvalidResponse)?;

        if choice.message.content.is_some() {
            let content = choice.message.content.as_ref().unwrap().clone();
            self.add(vec![Message::Assistant {
                content: vec![MessageContext::Text(content)]
            }]).await;
        } else {
            return Err(ClientError::UnknownError);
        }

        Ok(result)
    }

    /// AIが応答を生成  
    /// toolを使用することもある  
    pub async fn generate_use_tool(&mut self, model: &ModelConfig) -> Result<APIResult, ClientError> {
        let result = self.client.send_use_tool(model, &self.prompt).await?;
        let choices = result.response.choices.as_ref().ok_or(ClientError::InvalidResponse)?;
        let choice = choices.get(0).ok_or(ClientError::InvalidResponse)?;

        if choice.message.content.is_some() {
            let content = choice.message.content.as_ref().unwrap().clone();
            self.add(vec![Message::Assistant {
            content: vec![MessageContext::Text(content)],
            }])
            .await;
        } else if choice.message.function_call.is_some() {
            let fnc = choice.message.function_call.as_ref().unwrap();
            let (tool, enabled) = self.client.tools.get(&fnc.name).ok_or(ClientError::ToolNotFound)?;
            if !*enabled {
                return Err(ClientError::ToolNotFound);
            }
            if let Ok(res) = tool.run(fnc.arguments.clone()) {
                self.add(vec![Message::Function {
                    name: fnc.name.clone(),
                    content: vec![MessageContext::Text(res)],
                }])
                .await;
            } else if let Err(e) = tool.run(fnc.arguments.clone()) {
                self.add(vec![Message::Function {
                    name: fnc.name.clone(),
                    content: vec![MessageContext::Text(format!("Error: {}", e))],
                }])
                .await;
            }
        } else {
            return Err(ClientError::UnknownError);
        }

        Ok(result)
    }

    /// AIが応答を生成
    /// tool使用を強制
    pub async fn generate_with_tool(&mut self, model: &ModelConfig, tool_name: &str) -> Result<APIResult, ClientError> {
        let result = self.client.send_with_tool(model, &self.prompt, tool_name).await?;
        let choices = result.response.choices.as_ref().ok_or(ClientError::InvalidResponse)?;
        let choice = choices.get(0).ok_or(ClientError::InvalidResponse)?;

        if choice.message.content.is_some() {
            let content = choice.message.content.as_ref().unwrap().clone();
            self.add(vec![Message::Assistant {
                content: vec![MessageContext::Text(content)],
            }])
            .await;
        } else if choice.message.function_call.is_some() {
            let fnc = choice.message.function_call.as_ref().unwrap();
            let (tool, enabled) = self.client.tools.get(&fnc.name).ok_or(ClientError::ToolNotFound)?;
            if !*enabled {
                return Err(ClientError::ToolNotFound);
            }
            if let Ok(res) = tool.run(fnc.arguments.clone()) {
                self.add(vec![Message::Function {
                    name: fnc.name.clone(),
                    content: vec![MessageContext::Text(res)],
                }])
                .await;
            } else if let Err(e) = tool.run(fnc.arguments.clone()) {
                self.add(vec![Message::Function {
                    name: fnc.name.clone(),
                    content: vec![MessageContext::Text(format!("Error: {}", e))],
                }])
                .await;
            }
        } else {
            return Err(ClientError::UnknownError);
        }

        Ok(result)
    }
}