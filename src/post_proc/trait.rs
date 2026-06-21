/// A text post-processor that transforms transcribed text.
pub trait TextPostProcesser {
    /// Name used in `--post="+name(args)"`.
    fn processer_name() -> &'static str;

    /// Execute the post-processing.
    ///
    /// # Arguments
    /// * `param` — arguments parsed from `--post="+name(arg1, arg2)"`
    /// * `text` — the transcribed text to process
    fn process(param: &[&str], text: &str) -> String;
}
