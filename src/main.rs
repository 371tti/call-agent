use std::sync::Arc;

use call_agent::chat::{client::{ModelConfig, OpenAIClient}, function::Tool, prompt::{Message, MessageContext}};
use serde_json::Value;



// Define a custom tool
pub struct TextLengthTool;

impl TextLengthTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for TextLengthTool {
    fn def_name(&self) -> &str {
        "text_length_tool"
    }

    fn def_description(&self) -> &str {
        "Returns the length of the input text."
    }

    fn def_parameters(&self) -> Value {
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

    fn run(&self, args: Value) -> Result<String, String> {
        println!("{:?}", args);
        let text = args["text"].as_str().ok_or_else(|| "Missing 'text' parameter".to_string())?;
        let length = text.len();
        Ok(serde_json::json!({ "length": length }).to_string())
    }
}


#[tokio::main]
async fn main() {
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
        web_search_options: None, // Set to None if not using web search
    };

    // set the model configuration
    client.set_model_config(&config);

    // create a prompt stream
    let mut prompt_stream = client.create_prompt();

    // chat loop
    loop {
        // read user input
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed to read line");

        // create a prompt
        let prompt = vec![Message::User 
        {
            name:Some("user".to_string()),
            content:vec!
            [
                MessageContext::Text(input.trim().to_string()),
            ], 
        }
        ];

        // add the prompt to the stream
        prompt_stream.add(prompt).await;

        // generate a response
        let result = prompt_stream
            .generate_can_use_tool::<fn(&str, &serde_json::Value)>(None, None)
            .await;
        println!("{:?}", result);

        // get the response
        let response = prompt_stream.prompt.clone();
        println!("{:?}", response);
    }
}
