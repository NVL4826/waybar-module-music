/// Formats seconds into "mm:ss" or "hh:mm:ss" if duration >= 1 hour.
pub fn seconds_to_formatted_time(total_seconds: u64) -> String {
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
    fn test_seconds_to_formatted_time() {
        assert_eq!(seconds_to_formatted_time(0), "00:00");
        assert_eq!(seconds_to_formatted_time(65), "01:05");
        assert_eq!(seconds_to_formatted_time(3599), "59:59");
        assert_eq!(seconds_to_formatted_time(3600), "01:00:00");
        assert_eq!(seconds_to_formatted_time(3665), "01:01:05");
        assert_eq!(seconds_to_formatted_time(7325), "02:02:05");
    }
}



