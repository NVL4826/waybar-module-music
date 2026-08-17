pub fn microseconds_to_formatted_time(microseconds: u128) -> String {
    let total_seconds = microseconds / 1_000_000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_microseconds_to_formatted_time() {
        assert_eq!(microseconds_to_formatted_time(0), "00:00");
        assert_eq!(microseconds_to_formatted_time(65_000_000), "01:05");
        assert_eq!(microseconds_to_formatted_time(3_599_000_000), "59:59");
        assert_eq!(microseconds_to_formatted_time(3_600_000_000), "01:00:00");
        assert_eq!(microseconds_to_formatted_time(3_660_000_000), "01:01:00");
        assert_eq!(microseconds_to_formatted_time(7_325_000_000), "02:02:05");
    }
}

