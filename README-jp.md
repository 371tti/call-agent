# Call Agent

Call Agentは、Rustで実装された通話関連の機能とAI連携を行うためのライブラリです。このライブラリは、ユーザープロンプトの処理、関数呼び出しの実行、ツールの定義と実行、及びAPI経由での対話をサポートします。

## 特徴

- 高性能なプロンプト管理とチャット応答生成機能
- カスタムツール（関数）の定義と実行が可能
- 柔軟なエラーハンドリングと詳細なレスポンス解析
- 内部でのシリアライズ/デシリアライズ処理により、JSONとの連携が容易

## モジュール構成

- **client**: OpenAI APIとの通信を担い、ツールの登録・呼び出しやレスポンス処理を行う。  
- **prompt**: ユーザープロンプト、アシスタントメッセージ、関数呼び出しのメッセージを管理する。  
- **function**: ツール（関数）定義、実行、および引数のパース処理を提供。  
- **err**: エラー種類の定義とエラーメッセージ管理。  
- **api**: APIリクエスト/レスポンスの構造体およびシリアライズ/デシリアライズの実装。

## インストール

Cargo.tomlに以下を追加して依存関係として利用してください：

```toml
[dependencies]
call-agent = "0.1.0"
```

## 使い方

### クライアントの作成方法

以下はOpenAIClientの作成方法とカスタムツールの登録例です。

```rust
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
```

### client.rsにあるメソッドの説明

- new(end_point: &str, api_key: Option<&str>)  
  → OpenAIClientの新規作成。エンドポイントの正規化とAPIキーの設定を行います。

- def_tool<T: Tool + Send + Sync + 'static>(tool: Arc<T>)  
  → ツールの登録。既存のツール名がある場合は上書きされます。

- list_tools()  
  → 登録済みツールの一覧をタプル(ツール名, 説明, 有効状態)で返します。

- switch_tool(tool_name: &str, t_enable: bool)  
  → 指定したツールの有効/無効を切り替えます。

- export_tool_def()  
  → 有効なツールの関数定義(FunctionDef)のリストを返します。

- send(model: &ModelConfig, prompt: &Vec<Message>)  
  → 指定モデルでAPIリクエストを行い、応答を返します。

- send_use_tool(model: &ModelConfig, prompt: &Vec<Message>)  
  → 自動ツール呼び出し指定("auto")を使ってAPIリクエストを行います。

- send_with_tool(model: &ModelConfig, prompt: &Vec<Message>, tool_name: &str)  
  → 特定のツールを強制して呼び出すAPIリクエストを行います。

- call_api(...)  
  → エンドポイントへリクエストを投げ、APIResultを返す内部メソッド。ヘッダー情報とレスポンス本体のシリアライズを行います。

- create_prompt()  
  → プロンプト管理用のOpenAIClientStateを生成します。

### 基本的な利用方法

以下はmain.rsの使用例です。  
ユーザーからの入力を受け、プロンプトに追加してAIの応答およびツールの動作をチェーンして実行します。

```rust
// ...existing code in main.rs...

// プロンプトにユーザー入力と画像メッセージを追加
let prompt = vec![Message::User {
    content: vec![
        MessageContext::Text("こんにちは".to_string()),
        MessageContext::Image(MessageImage {
            url: "https://example.com/image.jpg".to_string(),
            detail: None,
        }),
    ],
}];

// プロンプトストリームに追加し、応答生成（ツール利用あり）を実行
prompt_stream.add(prompt).await;
let result = prompt_stream.generate_use_tool(&config).await;
```

### カスタムツールの定義

モジュール`function`の`Tool`トレイトを実装することで、任意のツールを定義できます。  
以下はテキストの長さを算出するサンプルツールの定義例です。

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

## API仕様

### リクエスト

- リクエストは`APIRequest`構造体で定義され、モデル名、メッセージ、関数定義、関数呼び出し情報、温度、最大トークン数、top_pが含まれます。

### レスポンス

- レスポンスは`APIResponse`構造体で受け取り、choices、モデル情報、エラーメッセージ、使用されたトークン数などを確認できます。
- ヘッダーからはレート制限情報なども取得可能です。

## エラーハンドリング

- `ClientError`は、ファイル未検出、入力エラー、ネットワークエラー、ツール未登録などの各種エラーを提供します。  
- 各エラーは独自のDisplay実装により、デバッグやユーザー通知に利用できます。

## ビルドと実行

1. Cargoプロジェクトとしてクローンまたは配置してください。  
2. `cargo build`でビルドし、`cargo run`で実行可能です。  
3. main.rsに記述されたチャットループを利用して、対話型でAIの応答とツール実行を確認できます。

## 貢献

問題の報告や改善提案、プルリクエストを歓迎します。  
詳細な仕様や変更点については、各モジュールのコメントをご参照ください。

## ライセンス

このプロジェクトはMITライセンスの下でライセンスされています。詳細はLICENSEファイルを参照してください。
