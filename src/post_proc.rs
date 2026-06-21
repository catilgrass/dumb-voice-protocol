mod r#trait;
pub use r#trait::*;

pub mod pinyin;

/// Look up a processor by name and run it.
pub fn run(text: &str, name: &str, args: &[&str]) -> Option<String> {
    macro_rules! match_processor {
        ($($processor:ty),+ $(,)?) => {
            match name {
                $(
                    n if n == <$processor as TextPostProcesser>::processer_name() => {
                        Some(<$processor as TextPostProcesser>::process(args, text))
                    }
                )+
                _ => None,
            }
        };
    }

    match_processor!(
        // +pinyin
        crate::post_proc::pinyin::PinyinPostProcesser,
    )
}
