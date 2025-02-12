use std::{collections::{HashMap, VecDeque}, sync::Arc};

use reqwest::{Client, Response};

use super::{
    api::{APIRequest, APIResponse, APIResponseHeaders},
    err::ClientError,
    function::{FunctionDef, Tool, ToolDef},
    prompt::{Message, MessageContext},
};

/// Main client structure for interacting with the OpenAI API.
pub struct OpenAIClient {
    /// HTTP client
    pub client: Client,
    /// API endpoint
    pub end_point: String,
    /// Optional API key
    pub api_key: Option<String>,
    /// Registered tools: key is the tool name, value is a tuple (tool, is_enabled)
    pub tools: HashMap<String, (Arc<dyn Tool + Send + Sync>, bool)>,
    /// Configuration for the model request.
    pub model_config: Option<ModelConfig>,
}

/// Configuration for the model request.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// Model name.
    pub model: String,
    /// Optional model name.
    pub model_name: Option<String>,
    /// Top-p sampling parameter.
    pub top_p: Option<f64>,
    /// Specifies whether to perform parallel ToolCalls.
    /// default: true
    pub parallel_tool_calls: Option<bool>,
    /// Specifies the diversity of tokens generated by the model.
    pub temperature: Option<f64>,
    /// Specifies the maximum number of tokens generated by the model.
    pub max_completion_tokens: Option<u64>,
    /// Specifies the level of effort for reasoning in the inference model:
    /// - "low": Low effort
    /// - "medium": Medium effort
    /// - "high": High effort
    /// default: "medium"
    pub reasoning_effort: Option<String>,
    /// Specifies whether to apply a presence penalty to the model.
    /// Range: 2.0..-2.0
    pub presence_penalty: Option<f64>,
    /// Strictly structured
    /// default: false
    /// Forced disabled in parallel ToolCalls
    pub strict: Option<bool>,
}

/// Contains the API response and its headers.
#[derive(Debug)]
pub struct APIResult {
    /// The parsed API response.
    pub response: APIResponse,
    /// Headers returned by the API.
    pub headers: APIResponseHeaders,
}

impl OpenAIClient {
    /// Create a new OpenAIClient.
    ///
    /// # Arguments
    ///
    /// * `end_point` - The endpoint of the OpenAI API.
    /// * `api_key` - Optional API key.
    pub fn new(end_point: &str, api_key: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            end_point: end_point.trim_end_matches('/').to_string(),
            api_key: api_key.map(|s| s.to_string()),
            tools: HashMap::new(),
            model_config: None,
        }
    }

    /// Set the default model configuration.
    /// 
    /// # Arguments
    /// 
    /// * `model_config` - The model configuration.
    pub fn set_model_config(&mut self, model_config: &ModelConfig) {
        self.model_config = Some(model_config.clone());
    }

    /// Register a tool.
    ///
    /// If a tool with the same name already exists, it will be overwritten.
    ///
    /// # Arguments
    ///
    /// * `tool` - Reference-counted tool implementing the Tool trait.
    pub fn def_tool<T: Tool + Send + Sync + 'static>(&mut self, tool: Arc<T>) {
        self.tools
            .insert(tool.def_name().to_string(), (tool, true));
    }

    /// List all registered tools.
    ///
    /// # Returns
    ///
    /// A list of tuples containing (tool name, tool description, enabled flag).
    pub fn list_tools(&self) -> Vec<(String, String, bool)> {
        let mut tools = Vec::new();
        for (tool_name, (tool, enable)) in self.tools.iter() {
            tools.push((
                tool_name.to_string(),
                tool.def_description().to_string(),
                *enable,
            ));
        }
        tools
    }

    /// Switch the enable/disable state of a tool.
    ///
    /// # Arguments
    ///
    /// * `tool_name` - The name of the tool.
    /// * `t_enable` - True to enable, false to disable.
    pub fn switch_tool(&mut self, tool_name: &str, t_enable: bool) {
        if let Some((_, enable)) = self.tools.get_mut(tool_name) {
            *enable = t_enable;
        }
    }

    /// Export the definitions of all enabled tools.
    ///
    /// # Returns
    ///
    /// A vector of function definitions.
    pub fn export_tool_def(&self) -> Result<Vec<ToolDef>, ClientError> {
        let mut defs = Vec::new();
        for (tool_name, (tool, enable)) in self.tools.iter() {
            if *enable {
                defs.push(ToolDef {
                    tool_type: "function".to_string(),
                    function: FunctionDef {
                        name: tool_name.clone(),
                        description: tool.def_description().to_string(),
                        parameters: tool.def_parameters(),
                        strict: self.model_config.as_ref().ok_or(ClientError::ModelConfigNotSet)?.strict.unwrap_or(false),
                    },
                });
            }
        }
        Ok(defs)
    }

    /// Send a chat request to the API.
    ///
    /// # Arguments
    ///
    /// * `model` - The model configuration.
    /// * `prompt` - A vector of user and system messages.
    ///
    /// # Returns
    ///
    /// The API result or a ClientError.
    pub async fn send(
        &self,
        prompt: &VecDeque<Message>,
        model: Option<&ModelConfig>,
    ) -> Result<APIResult, ClientError> {
        match self
            .call_api(
                prompt,
                Some(&serde_json::json!("none")),
                model,
            )
            .await
        {
            Ok(res) => Ok(res),
            Err(e) => Err(e),
        }
    }

    /// Send a chat request with tool auto-selection.
    ///
    /// # Arguments
    ///
    /// * `model` - The model configuration.
    /// * `prompt` - A vector of messages.
    ///
    /// # Returns
    ///
    /// The API result or a ClientError.
    pub async fn send_can_use_tool(
        &self,
        prompt: &VecDeque<Message>,
        model: Option<&ModelConfig>,
    ) -> Result<APIResult, ClientError> {
        match self
            .call_api(
                prompt,
                Some(&serde_json::json!("auto")),
                model,
            )
            .await
        {
            Ok(res) => Ok(res),
            Err(e) => Err(e),
        }
    }

    /// Send a chat request requiring the use of a tool.
    /// 
    /// # Arguments
    /// 
    /// * `model` - The model configuration.
    /// * `prompt` - A vector of messages.
    /// 
    /// # Returns
    /// 
    /// The API result or a ClientError.
    pub async fn send_use_tool(
        &self,
        prompt: &VecDeque<Message>,
        model: Option<&ModelConfig>,
    ) -> Result<APIResult, ClientError> {
        match self
            .call_api(
                prompt,
                Some(&serde_json::json!("required")),
                model,
            )
            .await
        {
            Ok(res) => Ok(res),
            Err(e) => Err(e),
        }
    }

    /// Send a chat request forcing the use of a specific tool.
    ///
    /// # Arguments
    ///
    /// * `model` - The model configuration.
    /// * `prompt` - A vector of messages.
    /// * `tool_name` - The name of the tool to force.
    ///
    /// # Returns
    ///
    /// The API result or a ClientError.
    pub async fn send_with_tool(
        &self,
        prompt: &VecDeque<Message>,
        tool_name: &str,
        model: Option<&ModelConfig>,
    ) -> Result<APIResult, ClientError> {
        let function_call = serde_json::json!({"type": "function", "function": {"name": tool_name}});

        match self
            .call_api(
                prompt,
                Some(&function_call),
                model,
            )
            .await
        {
            Ok(res) => Ok(res),
            Err(e) => Err(e),
        }
    }

    /// Calls the OpenAI chat completions API.
    ///
    /// # Arguments
    ///
    /// * `model` - The model name; e.g. "GPT-4o".
    /// * `prompt` - The list of messages.
    /// * `function_call` - Indicates function call mode:
    ///   - "auto"
    ///   - "none"
    ///   - { "name": "get_weather" }
    /// * `temp` - Temperature parameter.
    /// * `max_token` - Maximum tokens parameter.
    /// * `top_p` - Top-p sampling parameter.
    ///
    /// # Returns
    ///
    /// An APIResult on success or a ClientError on failure.
    pub async fn call_api(
        &self,
        prompt: &VecDeque<Message>,
        tool_choice: Option<&serde_json::Value>,
        model_config: Option<&ModelConfig>,
    ) -> Result<APIResult, ClientError> {
        let url = format!("{}/chat/completions", self.end_point);
        if !url.starts_with("https://") && !url.starts_with("http://") {
            return Err(ClientError::InvalidEndpoint);
        }

        let model_config = model_config.unwrap_or(self.model_config.as_ref().ok_or(ClientError::ModelConfigNotSet)?);
        let tools = self.export_tool_def()?;
        let res = self.request_api(&self.end_point, self.api_key.as_deref(), model_config, prompt, &tools, tool_choice.unwrap_or(&serde_json::Value::Null)).await?;

        let headers = APIResponseHeaders {
            retry_after: res
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok().and_then(|v| v.parse().ok())),
            reset: res
                .headers()
                .get("X-RateLimit-Reset")
                .and_then(|v| v.to_str().ok().and_then(|v| v.parse().ok())),
            rate_limit: res
                .headers()
                .get("X-RateLimit-Remaining")
                .and_then(|v| v.to_str().ok().and_then(|v| v.parse().ok())),
            limit: res
                .headers()
                .get("X-RateLimit-Limit")
                .and_then(|v| v.to_str().ok().and_then(|v| v.parse().ok())),
            extra_other: res
                .headers()
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().to_string(),
                        v.to_str().unwrap_or("").to_string(),
                    )
                })
                .collect(),
        };
        let text = res.text().await.map_err(|_| ClientError::InvalidResponse)?;
        let response_body: APIResponse =
            serde_json::from_str(&text).map_err(|_| {
            ClientError::InvalidResponse
            })?;

        Ok(APIResult {
            response: response_body,
            headers,
        })
    }

    pub async fn request_api(&self ,end_point: &str, api_key: Option<&str>, model_config: &ModelConfig ,message: &VecDeque<Message>, tools: &Vec<ToolDef>, tool_choice: &serde_json::Value) -> Result<Response, ClientError> {
        let request = APIRequest {
            model:                  model_config.model.clone(),
            messages:               message.clone(),
            tools:                  tools.clone(),
            tool_choice:            tool_choice.clone(),
            parallel_tool_calls:    model_config.parallel_tool_calls,
            temperature:            model_config.temperature,
            max_completion_tokens:  model_config.max_completion_tokens,
            top_p:                  model_config.top_p,
            reasoning_effort:       model_config.reasoning_effort.clone(),
            presence_penalty:       model_config.presence_penalty,
        };

        let res = self
            .client
            .post(&format!("{}/chat/completions", end_point))
            .header("Content-Type", "application/json")
            .header(
                "authorization",
                format!("Bearer {}", api_key.as_deref().unwrap_or("")),
            )
            .json(&request)
            .send()
            .await
            .map_err(|_| ClientError::NetworkError)?;

        Ok(res)
    }

    /// Create a new prompt conversation.
    ///
    /// # Returns
    ///
    /// A new OpenAIClientState with an empty message history.
    pub fn create_prompt(&self) -> OpenAIClientState {
        OpenAIClientState {
            prompt: VecDeque::new(),
            client: self,
        }
    }
}

/// Represents a client state with a prompt history.
#[derive(Clone)]
pub struct OpenAIClientState<'a> {
    /// Conversation history messages.
    pub prompt: VecDeque<Message>,
    /// Reference to the OpenAIClient.
    pub client: &'a OpenAIClient,
}

impl<'a> OpenAIClientState<'a> {
    /// Add messages to the conversation prompt.
    ///
    /// # Arguments
    ///
    /// * `messages` - A vector of messages to add.
    ///
    /// # Returns
    ///
    /// A mutable reference to self.
    pub async fn add(&mut self, messages: Vec<Message>) -> &mut Self {
        self.prompt.extend(messages);
        self
    }

    /// Clear all messages from the conversation prompt.
    ///
    /// # Returns
    ///
    /// A mutable reference to self.
    pub async fn clear(&mut self) -> &mut Self {
        self.prompt.clear();
        self
    }

    /// Retrieve the last message in the prompt.
    ///
    /// # Returns
    ///
    /// An Option containing a reference to the last Message.
    pub async fn last(&mut self) -> Option<&Message> {
        self.prompt.front()
    }

    /// Generate an AI response.
    ///
    /// This method sends the prompt to the API and, upon successful response,
    /// adds the assistant's message to the prompt.
    ///
    /// # Arguments
    ///
    /// * `model` - The model configuration.
    ///
    /// # Returns
    ///
    /// An APIResult with the API response or a ClientError.
    pub async fn generate(&mut self, model: Option<&ModelConfig>) -> Result<APIResult, ClientError> {
        let model = model.unwrap_or(self.client.model_config.as_ref().ok_or(ClientError::ModelConfigNotSet)?);
        let result = self.client.send(&self.prompt, Some(model)).await?;
        let choices = result.response.choices.as_ref().ok_or(ClientError::InvalidResponse)?;
        if let Some(choice) = choices.first() {
            if let Some(content) = &choice.message.content {
            self.add(vec![Message::Assistant {
                name: model.model_name.clone(),
                content: vec![MessageContext::Text(content.clone())],
                tool_calls: None,
            }])
            .await;
            } else {
            return Err(ClientError::UnknownError);
            }
        } else {
            return Err(ClientError::InvalidResponse);
        }

        Ok(result)
    }

    /// Generate an AI response, possibly calling a tool.
    ///
    /// If the API response includes a function call, it will run the corresponding tool.
    ///
    /// # Arguments
    ///
    /// * `model` - The model configuration.
    ///
    /// # Returns
    ///
    /// An APIResult with the API response or a ClientError.
    pub async fn generate_can_use_tool(&mut self, model: Option<&ModelConfig>) -> Result<APIResult, ClientError> {
        let model = model.unwrap_or(self.client.model_config.as_ref().ok_or(ClientError::ModelConfigNotSet)?);
        let result = self.client.send_can_use_tool(&self.prompt, Some(model)).await?;
        let choices = result.response.choices.as_ref().ok_or(ClientError::InvalidResponse)?;
        if let Some(choice) = choices.first() {
            if let Some(content) = &choice.message.content {
                if let Some(tool_calls) = &choice.message.tool_calls {
                    self.add(vec![Message::Assistant {
                        name: model.model_name.clone(),
                        content: vec![MessageContext::Text(content.clone())],
                        tool_calls: choice.message.tool_calls.clone(),
                    }]).await;
                    for fnc in tool_calls {
                        let (tool, enabled) = self
                            .client
                            .tools
                            .get(&fnc.function.name)
                            .ok_or(ClientError::ToolNotFound)?;
                        if !*enabled {
                            return Err(ClientError::ToolNotFound);
                        }
                        match tool.run(fnc.function.arguments.clone()) {
                            Ok(res) => {
                                self.add(vec![Message::Tool {
                                    tool_call_id: fnc.id.clone(),
                                    content: vec![MessageContext::Text(res)],
                                }])
                                .await;
                            }
                            Err(e) => {
                                self.add(vec![Message::Tool {
                                    tool_call_id: fnc.id.clone(),
                                    content: vec![MessageContext::Text(format!("Error: {}", e))],
                                }])
                                .await;
                            }
                        }
                    }
                } else {
                    self.add(vec![Message::Assistant {
                        name: model.model_name.clone(),
                        content: vec![MessageContext::Text(content.clone())],
                        tool_calls: None,
                    }])
                    .await;
                }
            } else if let Some(tool_calls) = &choice.message.tool_calls {
                self.add(vec![Message::Assistant {
                    name: model.model_name.clone(),
                    content: vec![MessageContext::Text("".to_string())],
                    tool_calls: choice.message.tool_calls.clone(),
                }]).await;
                for fnc in tool_calls {
                    let (tool, enabled) = self
                        .client
                        .tools
                        .get(&fnc.function.name)
                        .ok_or(ClientError::ToolNotFound)?;
                    if !*enabled {
                        return Err(ClientError::ToolNotFound);
                    }
                    match tool.run(fnc.function.arguments.clone()) {
                        Ok(res) => {
                            self.add(vec![Message::Tool {
                                tool_call_id: fnc.id.clone(),
                                content: vec![MessageContext::Text(res)],
                            }])
                            .await;
                        }
                        Err(e) => {
                            self.add(vec![Message::Tool {
                                tool_call_id: fnc.id.clone(),
                                content: vec![MessageContext::Text(format!("Error: {}", e))],
                            }])
                            .await;
                        }
                    }
                }
            } else {
                return Err(ClientError::UnknownError);
            }
        }

        Ok(result)
    }

    /// Generate an AI response while forcing the use of a specific tool.
    /// 
    /// If the response includes a function call, the specified tool will be executed
    /// 
    /// # Arguments
    /// 
    /// * `model` - The model configuration.
    /// * `tool_name` - The name of the tool to use.
    /// 
    /// # Returns
    /// 
    /// An APIResult with the API response or a ClientError.
    pub async fn generate_use_tool(&mut self, model: Option<&ModelConfig>) -> Result<APIResult, ClientError> {
        let model = model.unwrap_or(
            self.client
                .model_config
                .as_ref()
                .ok_or(ClientError::ModelConfigNotSet)?
        );
        let result = self.client.send_use_tool(&self.prompt, Some(model)).await?;
        let choices = result.response.choices.as_ref().ok_or(ClientError::InvalidResponse)?;
        
        if let Some(choice) = choices.first() {
            if let Some(content) = &choice.message.content {
                if let Some(tool_calls) = &choice.message.tool_calls {
                    self.add(vec![Message::Assistant {
                        name: model.model_name.clone(),
                        content: vec![MessageContext::Text(content.clone())],
                        tool_calls: choice.message.tool_calls.clone(),
                    }])
                    .await;
                    for fnc in tool_calls {
                        let (tool, enabled) = self
                            .client
                            .tools
                            .get(&fnc.function.name)
                            .ok_or(ClientError::ToolNotFound)?;
                        if !*enabled {
                            return Err(ClientError::ToolNotFound);
                        }
                        match tool.run(fnc.function.arguments.clone()) {
                            Ok(res) => {
                                self.add(vec![Message::Tool {
                                    tool_call_id: fnc.id.clone(),
                                    content: vec![MessageContext::Text(res)],
                                }])
                                .await;
                            }
                            Err(e) => {
                                self.add(vec![Message::Tool {
                                    tool_call_id: fnc.id.clone(),
                                    content: vec![MessageContext::Text(format!("Error: {}", e))],
                                }])
                                .await;
                            }
                        }
                    }
                } else {
                    self.add(vec![Message::Assistant {
                        name: model.model_name.clone(),
                        content: vec![MessageContext::Text(content.clone())],
                        tool_calls: None,
                    }])
                    .await;
                }
            } else if let Some(tool_calls) = &choice.message.tool_calls {
                self.add(vec![Message::Assistant {
                    name: model.model_name.clone(),
                    content: vec![MessageContext::Text("call tools".to_string())],
                    tool_calls: choice.message.tool_calls.clone(),
                }])
                .await;
                for fnc in tool_calls {
                    let (tool, enabled) = self
                        .client
                        .tools
                        .get(&fnc.function.name)
                        .ok_or(ClientError::ToolNotFound)?;
                    if !*enabled {
                        return Err(ClientError::ToolNotFound);
                    }
                    match tool.run(fnc.function.arguments.clone()) {
                        Ok(res) => {
                            self.add(vec![Message::Tool {
                                tool_call_id: fnc.id.clone(),
                                content: vec![MessageContext::Text(res)],
                            }])
                            .await;
                        }
                        Err(e) => {
                            self.add(vec![Message::Tool {
                                tool_call_id: fnc.id.clone(),
                                content: vec![MessageContext::Text(format!("Error: {}", e))],
                            }])
                            .await;
                        }
                    }
                }
            } else {
                return Err(ClientError::UnknownError);
            }
        } else {
            return Err(ClientError::UnknownError);
        }
        Ok(result)
    }

    /// Generate an AI response while forcing the use of a specific tool.
    ///
    /// If the response includes a function call, the specified tool will be executed.
    ///
    /// # Arguments
    ///
    /// * `model` - The model configuration.
    /// * `tool_name` - The name of the tool to use.
    ///
    /// # Returns
    ///
    /// An APIResult with the API response or a ClientError.
    pub async fn generate_with_tool(&mut self, model: Option<&ModelConfig>, tool_name: &str) -> Result<APIResult, ClientError> {
        let model = model.unwrap_or(
            self.client.model_config.as_ref().ok_or(ClientError::ModelConfigNotSet)?
        );
        let result = self.client.send_with_tool(&self.prompt, tool_name, Some(model)).await?;
        let choices = result.response.choices.as_ref().ok_or(ClientError::InvalidResponse)?;
        
        if let Some(choice) = choices.first() {
            if let Some(content) = &choice.message.content {
                if let Some(tool_calls) = &choice.message.tool_calls {
                    self.add(vec![Message::Assistant {
                        name: model.model_name.clone(),
                        content: vec![MessageContext::Text(content.clone())],
                        tool_calls: choice.message.tool_calls.clone(),
                    }])
                    .await;
                    for fnc in tool_calls {
                        let (tool, enabled) = self
                            .client
                            .tools
                            .get(&fnc.function.name)
                            .ok_or(ClientError::ToolNotFound)?;
                        if !*enabled {
                            return Err(ClientError::ToolNotFound);
                        }
                        match tool.run(fnc.function.arguments.clone()) {
                            Ok(res) => {
                                self.add(vec![Message::Tool {
                                    tool_call_id: fnc.id.clone(),
                                    content: vec![MessageContext::Text(res)],
                                }])
                                .await;
                            }
                            Err(e) => {
                                self.add(vec![Message::Tool {
                                    tool_call_id: fnc.id.clone(),
                                    content: vec![MessageContext::Text(format!("Error: {}", e))],
                                }])
                                .await;
                            }
                        }
                    }
                } else {
                    self.add(vec![Message::Assistant {
                        name: model.model_name.clone(),
                        content: vec![MessageContext::Text(content.clone())],
                        tool_calls: None,
                    }])
                    .await;
                }
            } else if let Some(tool_calls) = &choice.message.tool_calls {
                self.add(vec![Message::Assistant {
                    name: model.model_name.clone(),
                    content: vec![MessageContext::Text("call tools".to_string())],
                    tool_calls: choice.message.tool_calls.clone(),
                }])
                .await;
                for fnc in tool_calls {
                    let (tool, enabled) = self
                        .client
                        .tools
                        .get(&fnc.function.name)
                        .ok_or(ClientError::ToolNotFound)?;
                    if !*enabled {
                        return Err(ClientError::ToolNotFound);
                    }
                    match tool.run(fnc.function.arguments.clone()) {
                        Ok(res) => {
                            self.add(vec![Message::Tool {
                                tool_call_id: fnc.id.clone(),
                                content: vec![MessageContext::Text(res)],
                            }])
                            .await;
                        }
                        Err(e) => {
                            self.add(vec![Message::Tool {
                                tool_call_id: fnc.id.clone(),
                                content: vec![MessageContext::Text(format!("Error: {}", e))],
                            }])
                            .await;
                        }
                    }
                }
            } else {
                return Err(ClientError::UnknownError);
            }
        } else {
            return Err(ClientError::UnknownError);
        }
        Ok(result)
    }
    
}