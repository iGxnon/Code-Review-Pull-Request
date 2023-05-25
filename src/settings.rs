use crate::prompt::{PROMPT_REVIEW_CODE, PROMPT_SUMMARIZE_DIFF, PROMPT_SYSTEM, PROMPT_TRANSLATION};
use openai_flows::chat::ChatModel;
use std::{env, str::FromStr};
use strum_macros::{Display, EnumString};

const DEFAULT_SOURCE_FILETYPES: &[&str; 20] = &[
    ".js", ".py", ".java", ".ts", ".c", ".cc", ".cpp", ".cs", ".go", ".rs", ".sh", ".rb", ".php",
    ".lua", ".kt", ".swift", ".scala", ".pl", ".dart", ".jl",
];

// A wrapper for ChatModel to allow parsing from string
#[derive(Debug, Clone, Copy, Display, EnumString, Default)]
pub(crate) enum OpenAIModel {
    #[strum(serialize = "gpt4-32k")]
    GPT4_32K,
    #[strum(serialize = "gpt4")]
    GPT4,
    #[default]
    #[strum(serialize = "gpt3.5-turbo")]
    GPT35Turbo,
}

impl From<OpenAIModel> for ChatModel {
    fn from(model: OpenAIModel) -> Self {
        match model {
            OpenAIModel::GPT4_32K => ChatModel::GPT4_32K,
            OpenAIModel::GPT4 => ChatModel::GPT4,
            OpenAIModel::GPT35Turbo => ChatModel::GPT35Turbo,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PromptSettings {
    pub system_prompt: String,
    pub translation: String,
    pub review_code: String,
    pub summarize_diff: String,
}

#[derive(Debug, Clone)]
pub(crate) struct OutputSettings {
    pub lang: Option<String>, // None means no translation, default language is English
}

#[derive(Debug, Clone)]
pub(crate) struct Settings {
    pub model: OpenAIModel,
    pub prompt: PromptSettings,
    pub source_filetypes: Vec<String>,
    pub output: OutputSettings,
    pub trigger_phrase: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: OpenAIModel::default(),
            prompt: PromptSettings {
                system_prompt: PROMPT_SYSTEM.to_string(),
                translation: PROMPT_TRANSLATION.to_string(),
                review_code: PROMPT_REVIEW_CODE.to_string(),
                summarize_diff: PROMPT_SUMMARIZE_DIFF.to_string(),
            },
            source_filetypes: DEFAULT_SOURCE_FILETYPES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            output: OutputSettings { lang: None },
            trigger_phrase: "flows review".to_string(),
        }
    }
}

impl Settings {
    // Parse settings from environment variables.
    pub(crate) fn from_env() -> Self {
        let trigger_phrase = env::var("trigger_phrase").unwrap_or("flows review".to_string());
        let openai_model = env::var("openai_model").unwrap_or("gpt3.5-turbo".to_string());
        let prompt_system = env::var("prompt_system").unwrap_or(PROMPT_SYSTEM.to_string());
        let prompt_translation =
            env::var("prompt_translation").unwrap_or(PROMPT_TRANSLATION.to_string());
        let prompt_review_code =
            env::var("prompt_review_code").unwrap_or(PROMPT_REVIEW_CODE.to_string());
        let prompt_summarize_diff =
            env::var("prompt_summarize_diff").unwrap_or(PROMPT_SUMMARIZE_DIFF.to_string());
        let source_filetypes = env::var("source_filetypes")
            .unwrap_or(DEFAULT_SOURCE_FILETYPES.join(","))
            .split(',')
            .map(|s| s.to_string())
            .collect();
        let language = env::var("language").ok();
        Self {
            model: OpenAIModel::from_str(openai_model.as_str()).unwrap_or(OpenAIModel::default()),
            prompt: PromptSettings {
                system_prompt: prompt_system,
                translation: prompt_translation,
                review_code: prompt_review_code,
                summarize_diff: prompt_summarize_diff,
            },
            source_filetypes,
            output: OutputSettings { lang: language },
            trigger_phrase,
        }
    }

    pub(crate) async fn detect_file_type(mut self) -> Self {
        todo!()
    }

    #[inline]
    pub(crate) fn check_file_type(&self, filename: &str) -> bool {
        let mut check = false;
        for file_type in &self.source_filetypes {
            if filename.ends_with(file_type) {
                check = true;
                break;
            }
        }
        check
    }
}
