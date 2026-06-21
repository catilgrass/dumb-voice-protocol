fn main() {
    #[cfg(windows)]
    {
        embed_resource::compile("icon/app.rc", embed_resource::NONE)
            .manifest_optional()
            .expect("Failed to embed app icon");
    }
}
