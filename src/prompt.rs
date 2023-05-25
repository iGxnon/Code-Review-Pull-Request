use std::collections::HashMap;
use tera::{Context, Error, Tera};

pub static PROMPT_SYSTEM: &str = include_str!("../prompts/system.tera");
pub static PROMPT_TRANSLATION: &str = include_str!("../prompts/translation.tera");
pub static PROMPT_REVIEW_CODE: &str = include_str!("../prompts/review_code.tera");
pub static PROMPT_SUMMARIZE_DIFF: &str = include_str!("../prompts/summarize_diff.tera");

#[inline]
pub fn format_prompt(prompt: &str, map: HashMap<&str, &str>) -> Result<String, Error> {
    let context = Context::from_serialize(map)?;

    Tera::one_off(prompt, &context, false)
}
