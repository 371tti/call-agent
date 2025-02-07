use std::{collections::HashMap, sync::Arc};

use reqwest::Client;

use super::{
    api::{APIRequest, APIResponse, APIResponseHeaders},
    err::ClientError,
    function::{FunctionDef, Tool},
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
}

/// Represents a client state with a prompt history.
pub struct OpenAIClientState<'a> {
    /// Conversation history messages.
    pub prompt: Vec<Message>,
    /// Reference to the OpenAIClient.
    pub client: &'a OpenAIClient,
}

/// Configuration for the model request.
pub struct ModelConfig {
    /// Model name.
    pub model: String,
    /// Temperature setting.
    pub temp: Option<f64>,
    /// Maximum token count.
    pub max_token: Option<u64>,
    /// Top-p sampling parameter.
    pub top_p: Option<f64>,
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
        }
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
        model: &ModelConfig,
        prompt: &Vec<Message>,
    ) -> Result<APIResult, ClientError> {
        match self
            .call_api(
                &model.model,
                prompt,
                None,
                model.temp,
                model.max_token,
                model.top_p,
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
    pub async fn send_use_tool(
        &self,
        model: &ModelConfig,
        prompt: &Vec<Message>,
    ) -> Result<APIResult, ClientError> {
        match self
            .call_api(
                &model.model,
                prompt,
                Some(&serde_json::Value::String("auto".to_string())),
                model.temp,
                model.max_token,
                model.top_p,
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
        model: &ModelConfig,
        prompt: &Vec<Message>,
        tool_name: &str,
    ) -> Result<APIResult, ClientError> {
        let function_call = serde_json::json!({
            "name": tool_name,
        });

        match self
            .call_api(
                &model.model,
                prompt,
                Some(&function_call),
                model.temp,
                model.max_token,
                model.top_p,
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
        model: &str,
        prompt: &Vec<Message>,
        function_call: Option<&serde_json::Value>,
        temp: Option<f64>,
        max_token: Option<u64>,
        top_p: Option<f64>,
    ) -> Result<APIResult, ClientError> {
        let url = format!("{}/chat/completions", self.end_point);
        if !url.starts_with("https://") && !url.starts_with("http://") {
            return Err(ClientError::InvalidEndpoint);
        }

        let request = APIRequest {
            model: model.to_string(),
            messages: prompt.clone(),
            functions: self.export_tool_def(),
            function_call: function_call
                .unwrap_or(&serde_json::Value::String("none".to_string()))
                .clone(),
            temperature: temp.unwrap_or(0.5),
            max_tokens: max_token.unwrap_or(4000),
            top_p: top_p.unwrap_or(1.0),
        };

        let res = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header(
                "authorization",
                format!("Bearer {}", self.api_key.as_deref().unwrap_or("")),
            )
            .json(&request)
            .send()
            .await
            .map_err(|_| ClientError::NetworkError)?;

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
            serde_json::from_str(&text).map_err(|_| ClientError::InvalidResponse)?;

        Ok(APIResult {
            response: response_body,
            headers,
        })
    }

    /// Create a new prompt conversation.
    ///
    /// # Returns
    ///
    /// A new OpenAIClientState with an empty message history.
    pub fn create_prompt(&self) -> OpenAIClientState {
        OpenAIClientState {
            prompt: Vec::new(),
            client: self,
        }
    }
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
        self.prompt.last()
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
    pub async fn generate(&mut self, model: &ModelConfig) -> Result<APIResult, ClientError> {
        let result = self.client.send(model, &self.prompt).await?;
        let choices = result.response.choices.as_ref().ok_or(ClientError::InvalidResponse)?;
        let choice = choices.get(0).ok_or(ClientError::InvalidResponse)?;

        if choice.message.content.is_some() {
            let content = choice.message.content.as_ref().unwrap().clone();
            self.add(vec![Message::Assistant {
                content: vec![MessageContext::Text(content)],
            }])
            .await;
        } else {
            return Err(ClientError::UnknownError);
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
            let (tool, enabled) = self
                .client
                .tools
                .get(&fnc.name)
                .ok_or(ClientError::ToolNotFound)?;
            if !*enabled {
                return Err(ClientError::ToolNotFound);
            }
            // Try to run the tool and add its output as a function message.
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
            let (tool, enabled) = self
                .client
                .tools
                .get(&fnc.name)
                .ok_or(ClientError::ToolNotFound)?;
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