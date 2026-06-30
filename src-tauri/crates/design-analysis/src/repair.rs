use crate::prompt::FIXED_ANALYSIS_INSTRUCTIONS;

pub fn repair_prompt(invalid_output: &str, error: &str) -> String {
    let invalid_output_len = invalid_output.len();
    format!(
        "{FIXED_ANALYSIS_INSTRUCTIONS}\n\n\
         Repair the previous design analysis response so it returns only JSON matching the supplied schema.\n\
         Re-analyze the attached screenshots; do not copy invalid previous output into the repaired JSON.\n\
         Remove unsupported, sensitive, or invalid fields.\n\
         Validation error: {error}\n\
         Previous response byte length: {invalid_output_len}"
    )
}
