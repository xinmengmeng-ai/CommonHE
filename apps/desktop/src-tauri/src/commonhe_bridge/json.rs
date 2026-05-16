pub fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("serializing a string cannot fail")
}

pub fn json_string_array(values: &[String]) -> String {
    serde_json::to_string(values).expect("serializing string array cannot fail")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_windows_paths_quotes_and_newlines() {
        let value = "E:\\WorkSoft\\CommonHE\\\"payload\"\nnext";

        assert_eq!(
            json_string(value),
            "\"E:\\\\WorkSoft\\\\CommonHE\\\\\\\"payload\\\"\\nnext\""
        );
    }

    #[test]
    fn serializes_string_arrays() {
        let values = vec!["powershell".to_string(), "pwsh".to_string()];

        assert_eq!(json_string_array(&values), "[\"powershell\",\"pwsh\"]");
    }
}
