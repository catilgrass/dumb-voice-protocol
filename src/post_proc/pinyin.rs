use crate::post_proc::TextPostProcesser;

/// Converts Chinese characters to pinyin (romanized phonetic representation).
///
/// # Styles
/// - `plain` — ni hao (default)
/// - `tone` — nǐ hǎo (with tone marks, requires `with_tone` feature)
/// - `tone-num` — ni3 hao3 (tone number at end)
/// - `first-letter` — n h (first letter only)
pub struct PinyinPostProcesser;

impl TextPostProcesser for PinyinPostProcesser {
    fn processer_name() -> &'static str {
        "pinyin"
    }

    fn process(param: &[&str], text: &str) -> String {
        let style = param.first().copied().unwrap_or("plain");
        let result: Vec<&'static str> = match style {
            "tone" => pinyin::to_pinyin_vec(text, pinyin::Pinyin::with_tone),
            "tone-num" | "tone_num" => {
                pinyin::to_pinyin_vec(text, pinyin::Pinyin::with_tone_num_end)
            }
            "first-letter" | "first_letter" => {
                pinyin::to_pinyin_vec(text, pinyin::Pinyin::first_letter)
            }
            _ => pinyin::to_pinyin_vec(text, pinyin::Pinyin::plain),
        };
        result.join(" ")
    }
}
