# Call Agent

Call Agent is a library implemented in Rust for handling call-related functions and integrating with AI. This library supports processing user prompts, executing function calls, defining and executing tools, and interacting via APIs.

## Features

- High-performance prompt management and chat response generation
- Ability to define and execute custom tools (functions)
- Flexible error handling and detailed response analysis
- Easy integration with JSON through internal serialization/deserialization

## Module Structure

- **client**: Handles communication with the OpenAI API, registers and calls tools, and processes responses.
- **prompt**: Manages user prompts, assistant messages, and function call messages.
- **function**: Provides tool (function) definition, execution, and argument parsing.
- **err**: Defines error types and manages error messages.
- **api**: Implements the structures for API requests/responses and serialization/deserialization.

## Installation

Add the following to your 

Cargo.toml

 to use it as a dependency:

```toml
[dependencies]
call-agent = "1.0.0"
```

## Usage

### Example of Creating a Client and Registering Tools

```rust
// create a new OpenAI client
let mut client = OpenAIClient::new(
    "https://api.openai.com/v1/",
    Some("YOUR_API_KEY"),
);

// register the custom tool
client.def_tool(Arc::new(TextLengthTool::new()));

// create a model configuration
let config = ModelConfig {
    model: "gpt-4o-mini".to_string(),
    strict: None,
    max_completion_tokens: Some(1000),
    temperature: Some(0.8),
    top_p: Some(1.0),
    parallel_tool_calls: None,
    presence_penalty: Some(0.0),
    model_name: None,
    reasoning_effort: None,
};

// set the model configuration
client.set_model_config(&config);
```

### Methods in `client.rs`

- `new(end_point: &str, api_key: Option<&str>)`
  → Creates a new `OpenAIClient`. Normalizes the endpoint and sets the API key.

- `def_tool<T: Tool + Send + Sync + 'static>(tool: Arc<T>)`
  → Registers a tool. Overwrites if a tool with the same name exists.

- `list_tools()`
  → Returns a list of registered tools as tuples (tool name, description, enabled status).

- `switch_tool(tool_name: &str, t_enable: bool)`
  → Enables or disables the specified tool.

- `export_tool_def()`
  → Returns a list of function definitions (`FunctionDef`) for enabled tools.

- `send(model: &ModelConfig, prompt: &Vec<Message>)`
  → Makes an API request with the specified model and returns the response.

- `send_use_tool(model: &ModelConfig, prompt: &Vec<Message>)`
  → Makes an API request using the "auto" tool call specification.

- `send_with_tool(model: &ModelConfig, prompt: &Vec<Message>, tool_name: &str)`
  → Makes an API request forcing the use of a specific tool.

- `call_api(...)`
  → Internal method that sends a request to the endpoint and returns an `APIResult`. Serializes header information and the response body.

- `create_prompt()`
  → Generates an `OpenAIClientState` for prompt management.

### Basic Usage

Below is an example of usage in `main.rs`.  
It receives user input, adds it to the prompt, and generates AI responses and tool actions in a chain.

```rust
// ...existing code in main.rs...

// Add user input and image message to the prompt
let prompt = vec![Message::User {
    content: vec![
        MessageContext::Text("Hello".to_string()),
        MessageContext::Image(MessageImage {
            url: "https://example.com/image.jpg".to_string(),
            detail: None,
        }),
    ],
}];

// Add to the prompt stream and generate response (with tool usage)
prompt_stream.add(prompt).await;
let result = prompt_stream.generate_use_tool(&config).await;
```

### Example of Using a Chat Loop

```rust
// create a prompt stream
let mut prompt_stream = client.create_prompt();

// chat loop: Get user input → Add to prompt → Generate response with tool usage
loop {
    // Get user input
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("Failed to read line");

    // Create the prompt
    let prompt = vec![Message::User {
        name: Some("user".to_string()),
        content: vec![
            MessageContext::Text(input.trim().to_string()),
        ],
    }];

    // Add to the prompt
    prompt_stream.add(prompt).await;

    // Generate response using `generate_can_use_tool`
    let result = prompt_stream.generate_can_use_tool(None).await;
    println!("{:?}", result);

    // Optionally check the latest state of the prompt
    let response = prompt_stream.prompt.clone();
    println!("{:?}", response);
}
```

### Defining Custom Tools

You can define any tool by implementing the `Tool` trait in the `function` module.  
Below is an example of defining a tool that calculates the length of a text.

```rust
// ...existing code in main.rs...
impl Tool for TextLengthTool {
    fn def_name(&self) -> &str {
        "text_length_tool"
    }
    fn def_description(&self) -> &str {
        "Returns the length of the input text."
    }
    fn def_parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Input text to calculate its length"
                }
            },
            "required": ["text"]
        })
    }
    fn run(&self, args: serde_json::Value) -> Result<String, String> {
        let text = args["text"]
            .as_str()
            .ok_or_else(|| "Missing 'text' parameter".to_string())?;
        let length = text.len();
        Ok(serde_json::json!({ "length": length }).to_string())
    }
}
```

## API Specifications

### Request

- Requests are defined by the `APIRequest` structure, which includes the model name, messages, function definitions, function call information, temperature, maximum token count, and top_p.

### Response

- Responses are received in the `APIResponse` structure, where you can check choices, model information, error messages, and the number of tokens used.
- Rate limit information can also be obtained from the headers.

## Error Handling

- `ClientError` provides various errors such as file not found, input errors, network errors, and tool not registered.
- Each error has its own `Display` implementation, useful for debugging and user notifications.

## Build and Run

1. Clone or place it as a Cargo project.
2. Build with `cargo build` and run with `cargo run`.
3. Use the chat loop described in `main.rs` to interactively check AI responses and tool execution.

## Contribution

We welcome issue reports, improvement suggestions, and pull requests.  
For detailed specifications and changes, please refer to the comments in each module.

## License

This project is licensed under the MIT License. See the LICENSE file for details.