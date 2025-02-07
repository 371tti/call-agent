use std::sync::Arc;
use call_agent::{
    client::{ModelConfig, OpenAIClient},
    function::Tool,
    prompt::{Message, MessageContext, MessageImage},
};
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
        "https://example.com/v1",
        Some("API_KEY"),
    );

    // register the custom tool
    client.def_tool(Arc::new(TextLengthTool::new()));

    // create a model configuration
    let config = ModelConfig {
        model: "gpt-4o-mini".to_string(),
        temp: Some(0.5),
        max_token: Some(100),
        top_p: Some(1.0),
    };

    // create a prompt stream
    let mut prompt_stream = client.create_prompt();

    // chat loop
    loop {
        // read user input
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).expect("Failed to read line");

        // create a prompt
        let prompt = vec![Message::User {
            content: vec![
                // create a text message
                MessageContext::Text(input.trim().to_string()),
                // create an image message
                MessageContext::Image(MessageImage {
                    url: "https://pbs.twimg.com/media/GjHbHvXbIAEUrSs?format=jpg&name=900x900".to_string(),
                    detail: None,
                }),
            ],
        }];

        // add the prompt to the stream
        prompt_stream.add(prompt).await;

        // generate a response
        let result = prompt_stream.generate_use_tool(&config).await;
        println!("{:?}", result);

        // get the response
        let response = prompt_stream.last().await.unwrap();
        println!("{:?}", response);
    }
}
