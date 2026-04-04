use std::time::Duration;

use tokio::process::Command;

use crate::error::{AppError, Result};

const CLAUDE_MODEL: &str = "haiku";
const SUMMARY_TIMEOUT: Duration = Duration::from_secs(90);

pub struct Summarizer;

impl Default for Summarizer {
    fn default() -> Self {
        Summarizer
    }
}

impl Summarizer {
    pub fn new() -> Self {
        Summarizer
    }

    pub async fn generate_summary(
        &self,
        article_title: &str,
        article_content: &str,
    ) -> Result<String> {
        // Truncate content if too long (find valid UTF-8 boundary)
        let content = if article_content.len() > 10000 {
            let mut end = 10000;
            while end > 0 && !article_content.is_char_boundary(end) {
                end -= 1;
            }
            &article_content[..end]
        } else {
            article_content
        };

        let prompt = format!(
            r#"You are a journalist summarizing articles using the nut graph structure. Summarize the article below using the appropriate format.

First, determine: Is this article primarily about a specific PRODUCT (hardware, software, app, device) or is it EDITORIAL (news, policy, analysis, industry event)?

RULES:
1. Use ONLY information from the article - no external knowledge
2. If the article has insufficient content, respond with just: "Insufficient content for summary"
3. QUOTE must be copied VERBATIM from the article — the exact words as they appear, with clear speaker attribution. Do not paraphrase or alter the quote in any way.

If EDITORIAL, respond with:
One strong sentence identifying WHO is involved and WHAT happened or was announced.

A paragraph (2-4 sentences) explaining WHY this matters. Contextualize the most important facts and give the reader a clear understanding of the central issue or topic.

"exact verbatim quote from the article" -- Speaker Name

If PRODUCT, respond with:
What the product is and what it does (1-2 sentences).

Pricing details. Omit if not mentioned.

When and where it is available. Omit if not mentioned.

What platforms or operating systems it runs on. Omit for hardware-only products or if not mentioned.

"exact verbatim quote from the article" -- Speaker Name

Do NOT include any labels, prefixes, or headings. Just the plain text. Omit the quote line if there are no direct quotes with clear speaker attribution in the article.

Title: {}

Article:
{}"#,
            article_title, content
        );

        let output = tokio::time::timeout(
            SUMMARY_TIMEOUT,
            Command::new("claude")
                .args([
                    "-p",
                    &prompt,
                    "--model",
                    CLAUDE_MODEL,
                    "--output-format",
                    "text",
                    "--bare",
                    "--no-session-persistence",
                    "--max-turns",
                    "1",
                ])
                .output(),
        )
        .await
        .map_err(|_| AppError::Claude("Summary timed out after 90s".to_string()))?
        .map_err(|e| AppError::Claude(format!("Failed to run claude CLI: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Claude(format!("claude CLI error: {}", stderr)));
        }

        let summary = String::from_utf8_lossy(&output.stdout).to_string();

        // Strip format/type labels the model sometimes adds despite instructions
        let summary = summary
            .lines()
            .filter(|line| {
                let upper = line.trim().to_uppercase();
                !upper.starts_with("FORMAT:")
                    && upper != "EDITORIAL"
                    && upper != "PRODUCT"
                    && upper != "**EDITORIAL**"
                    && upper != "**PRODUCT**"
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        Ok(summary)
    }

    pub fn model_version(&self) -> &'static str {
        CLAUDE_MODEL
    }
}
