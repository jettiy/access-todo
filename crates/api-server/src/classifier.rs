//! Best-effort note classifier using the LOCAL Ollama instance (the pilot's
//! "local jev" — qwen on 127.0.0.1:11434). Entirely optional: when Ollama is
//! down, slow, or unconfigured the caller falls back to the active workspace
//! tag, so the core Access flow never depends on an LLM.

use serde_json::Value;

/// Classify a dev note into one of `candidates` (tags like "game:마을도우미").
/// Returns None on any failure — callers treat that as "no opinion".
pub async fn classify_note(
    base_url: &str,
    model: &str,
    title: &str,
    body: &str,
    candidates: &[String],
) -> Option<String> {
    if candidates.is_empty() {
        return None;
    }
    let list = candidates.join(" | ");
    let snippet: String = {
        let t = format!("{title}\n{body}");
        t.chars().take(600).collect()
    };
    let payload = serde_json::json!({
        "model": model,
        "stream": false,
        "options": { "temperature": 0 },
        "messages": [
            { "role": "system", "content":
                "당신은 개발노트 분류기입니다. 사용자 노트를 작업 공간 후보 중 정확히 하나로 분류합니다. \
                 답은 후보 태그 하나만 출력하고 다른 설명은 하지 마세요. 확실하지 않으면 '없음'이라고만 답하세요." },
            { "role": "user", "content":
                format!("후보: {list} | 없음\n\n노트:\n{snippet}\n\n이 노트에 맞는 후보 태그 하나만 답하세요.") }
        ]
    });

    let client = reqwest::Client::builder()
        // 27B 로컬 모델은 웜 ~7–10초, 콜드 로드는 더 걸린다 — 분류 품질 우선(노트는 드물게 쓰임).
        // 45초 안에 안 오면 폴백(활성 작업 공간)으로 떨어진다.
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .ok()?;
    let resp = client
        .post(format!("{base_url}/api/chat"))
        .json(&payload)
        .send()
        .await
        .ok()?;
    let v: Value = resp.json().await.ok()?;
    let content = v["message"]["content"].as_str()?;
    // 추론 모델은 <think>…</think>를 앞에 붙일 수 있음 — 제거 후 첫 매칭 후보 사용
    let cleaned = strip_think(content);
    // 1) 전체 응답에서 후보 토큰 정확 매칭 (답만 출력했다는 전제)
    for line in cleaned.lines().rev() {
        let l = line.trim();
        if let Some(hit) = candidates.iter().find(|c| l.contains(c.as_str())) {
            return Some(hit.clone());
        }
    }
    // 2) 후보가 안 보이면 '없음'이 아니더라도 확신 없음으로 간주
    None
}

fn strip_think(s: &str) -> String {
    match (s.find("<think>"), s.find("</think>")) {
        (Some(a), Some(b)) if b > a => format!("{}{}", &s[..a], &s[b + 8..]),
        _ => s.to_string(),
    }
}
