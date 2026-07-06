//! Helper binary: creates a secret Gist seeded with an empty `TodoDoc`
//! and prints its id so the user can configure clients once.
//!
//! Usage:
//!   GITHUB_TOKEN=ghp_... cargo run -p core --bin bootstrap-gist

use todo_core::model::TodoDoc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN env required");
    let doc = TodoDoc::new("bootstrap");

    let payload = serde_json::json!({
        "description": "desktop-todo-agents (private)",
        "public": false,
        "files": {
            "todos.json": {
                "content": serde_json::to_string_pretty(&doc)?
            }
        }
    });

    let resp = reqwest::Client::new()
        .post("https://api.github.com/gists")
        .bearer_auth(&token)
        .header("Accept", "application/vnd.github+json")
        .json(&payload)
        .send()
        .await?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await?;
    if !status.is_success() {
        eprintln!("error {status}: {body}");
        std::process::exit(1);
    }
    let id = body["id"].as_str().expect("gist id");
    println!("Created secret Gist.");
    println!("  Set TODO_GIST_ID={id}");
    println!("  Web URL: https://gist.github.com/{id}");
    Ok(())
}
