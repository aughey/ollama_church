use anyhow::Result;
use ollama_rs::coordinator::Coordinator;
use ollama_rs::generation::tools::implementations::{Calculator, DDGSearcher, Scraper};
use ollama_rs::generation::tools::Tool;
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;
use tokio_stream::StreamExt;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MyToolParams {
    #[schemars(
        description = "Enables control of what camera will be presented to viewer. Possible values are:
- `ambo` for viewing the reader who would be reading from the podium
- `altar` for viewing the altar
- `wide` for a wide view of the sanctuary
- `narrow` for a narrow view of the stage "
    )]
    camera: String,
}

struct MyTool {}
impl Tool for MyTool {
    type Params = MyToolParams;
    fn name() -> &'static str {
        "my_tool"
    }
    fn description() -> &'static str {
        "My custom tool"
    }

    async fn call(
        &mut self,
        parameters: Self::Params,
    ) -> std::result::Result<String, Box<dyn std::error::Error>> {
        println!("In my tool: {:?}", parameters);
        Ok("done".into())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let ollama = Ollama::new("http://host.docker.internal".to_string(), 11434);
    if false {
        let model = "llama3:latest".to_string();
        let prompt = "Why is the sky blue?".to_string();

        let mut stream = ollama
            .generate_stream(GenerationRequest::new(model, prompt))
            .await
            .unwrap();

        let mut stdout = tokio::io::stdout();
        while let Some(res) = stream.next().await {
            let responses = res.unwrap();
            for resp in responses {
                stdout.write_all(resp.response.as_bytes()).await.unwrap();
                stdout.flush().await.unwrap();
            }
        }
    }

    let history = vec![];

    let mut coordinator =
        Coordinator::new_with_tools(ollama, "qwen2.5:7b".to_string(), history, MyTool {})
            .options(ollama_rs::generation::options::GenerationOptions::default().num_ctx(16384))
            .debug(true);

    let resp = coordinator
        .chat(vec![
            ollama_rs::generation::chat::ChatMessage::system(
            "You are a director for a video production of a church service. You have control over the camera angles. You will be given a live script of the service.  At any time, you can change the camera view to best optimize the viewers experience.".into(),
        ),
        //     ollama_rs::generation::chat::ChatMessage::user(
        //     "What is the current oil price?".into(),
        // )
        ])
        .await
        .map_err(|e| anyhow::anyhow!("Error with coordinator: {e:?}"))?;

    //println!("{}", resp.message.content);

    let captions = std::fs::read_to_string("captions.srt")?;
    let lines = captions.as_str().split("\n\n");
    for line in lines.skip(80) {
        let mut parts = line.split('\n');
        let words = parts.nth(2).unwrap_or_default();

        println!("Feeding chat: {}", words);
        let resp = coordinator
            .chat(vec![ollama_rs::generation::chat::ChatMessage::user(
                format!("Next line: {}", words),
            )])
            .await
            .map_err(|e| anyhow::anyhow!("Error with coordinator: {e:?}"))?;
        println!("Response: {}", resp.message.content);
    }

    Ok(())
}
