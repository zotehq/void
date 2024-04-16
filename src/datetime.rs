use std::time::{SystemTime, UNIX_EPOCH};

pub struct DateTime {
  pub date: String,
  pub time: String,
}

impl DateTime {
  pub fn new() -> Self {
    let now = SystemTime::now();
    let since_epoch = now.duration_since(UNIX_EPOCH).expect("Time went backwards");

    let days = since_epoch.as_secs() / (24 * 60 * 60);
    let hours = (since_epoch.as_secs() / (60 * 60)) % 24;
    let minutes = (since_epoch.as_secs() / 60) % 60;
    let seconds = since_epoch.as_secs() % 60;

    let year = 1970 + (days / 365);
    let days_in_year = days % 365;
    let mut month = 1;
    let mut day = days_in_year + 1;
    let days_per_month = [
      31,
      if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
        29
      } else {
        28
      },
      31,
      30,
      31,
      30,
      31,
      31,
      30,
      31,
      30,
      31,
    ];
    for &days_in_month in &days_per_month {
      if day <= days_in_month {
        break;
      }
      day -= days_in_month;
      month += 1;
    }

    Self {
      date: format!("{:02}/{:02}/{:04}", day, month, year),
      time: format!("{:02}:{:02}:{:02}", hours, minutes, seconds),
    }
  }
}
