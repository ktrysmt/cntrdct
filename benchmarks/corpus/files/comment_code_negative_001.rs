/// Returns Err on failure.
fn doc_matches_result(input: i32) -> Result<i32, String> {
    if input < 0 {
        return Err("negative".to_string());
    }
    Ok(input)
}
