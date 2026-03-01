use std::path::PathBuf;

#[test]
fn test_example_parallel_experimental_dev_engine_prompt_file_self_contained() {
    // ---------------------------------------------------------------------
    // 说明:
    // - 这个仓库自带的 example 目录应该是“自包含”的.
    // - 用户应当可以直接:
    //   `cd examples/parallel-experimental-dev-engine && ralph run`
    // - 因此 example 的 `ralph.yml` 里,`event_loop.prompt_file` 必须指向同目录的 `PROMPT.md`,
    //   而不是写死成“仓库根目录相对路径”.
    // ---------------------------------------------------------------------
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let example_dir = manifest_dir.join("../../examples/parallel-experimental-dev-engine");

    let config_path = example_dir.join("ralph.yml");
    let config_content = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        panic!(
            "failed to read example config {}: {e}",
            config_path.display()
        );
    });

    assert!(
        config_content.contains("prompt_file: \"PROMPT.md\""),
        "example config {} should contain `prompt_file: \"PROMPT.md\"` (self-contained)",
        config_path.display()
    );

    assert!(
        !config_content
            .contains("prompt_file: \"examples/parallel-experimental-dev-engine/PROMPT.md\""),
        "example config {} should not hardcode repo-root relative prompt path (breaks running inside the example directory)",
        config_path.display()
    );

    let prompt_path = example_dir.join("PROMPT.md");
    assert!(
        prompt_path.exists(),
        "example prompt file should exist: {}",
        prompt_path.display()
    );
}
