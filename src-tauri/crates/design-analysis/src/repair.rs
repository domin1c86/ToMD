pub fn repair_prompt(invalid_output: &str, error: &str) -> String {
    format!(
        "Repair the previous design analysis response so it returns only JSON matching the supplied schema.\n\
         Preserve visually supported evidence and remove unsupported or invalid fields.\n\
         Validation error: {error}\n\
         Previous response:\n{invalid_output}"
    )
}
