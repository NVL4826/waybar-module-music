pub fn microseconds_to_formatted_time(microseconds: u128) -> String {
    let seconds = microseconds / 1000 / 1000;
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_microseconds_to_formatted_time() {
        assert_eq!(microseconds_to_formatted_time(0), "00:00");
        assert_eq!(microseconds_to_formatted_time(65_000_000), "01:05");
        assert_eq!(microseconds_to_formatted_time(3_660_000_000), "61:00");
    }
}
