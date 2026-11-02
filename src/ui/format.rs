//! Turning data into the short text rows show.

use std::{borrow::Cow, path::Path, time::SystemTime};

/// `path` with the home folder as `~`.
#[must_use]
pub fn abbreviate_home(path: &str) -> Cow<'_, str> {
    directories::BaseDirs::new()
        .and_then(|dirs| {
            let rest = Path::new(path).strip_prefix(dirs.home_dir()).ok()?;
            Some(Cow::Owned(
                Path::new("~").join(rest).to_string_lossy().into_owned(),
            ))
        })
        .unwrap_or(Cow::Borrowed(path))
}

/// How long ago `then` was, in the largest unit that fits ("2d ago").
#[must_use]
pub fn relative_time(then: SystemTime, now: SystemTime) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    const MONTH: u64 = 30 * DAY;
    const YEAR: u64 = 365 * DAY;

    let seconds = now.duration_since(then).map_or(0, |age| age.as_secs());
    match seconds {
        s if s < MINUTE => "just now".to_owned(),
        s if s < HOUR => format!("{}m ago", s / MINUTE),
        s if s < DAY => format!("{}h ago", s / HOUR),
        s if s < MONTH => format!("{}d ago", s / DAY),
        s if s < YEAR => format!("{}mo ago", s / MONTH),
        s => format!("{}y ago", s / YEAR),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::{abbreviate_home, relative_time};

    #[test]
    fn relative_times_use_the_largest_unit() {
        let now = SystemTime::now();
        let ago = |seconds| relative_time(now - Duration::from_secs(seconds), now);
        assert_eq!(ago(5), "just now");
        assert_eq!(ago(5 * 60), "5m ago");
        assert_eq!(ago(3 * 3600), "3h ago");
        assert_eq!(ago(2 * 86_400), "2d ago");
        assert_eq!(ago(60 * 86_400), "2mo ago");
        assert_eq!(ago(800 * 86_400), "2y ago");
    }

    #[test]
    fn a_time_in_the_future_is_just_now() {
        let now = SystemTime::now();
        assert_eq!(
            relative_time(now + Duration::from_secs(60), now),
            "just now"
        );
    }

    #[test]
    fn only_the_home_folder_is_abbreviated() {
        assert_eq!(abbreviate_home("/not/home/Game"), "/not/home/Game");
        let home = directories::BaseDirs::new().expect("a home folder");
        let inside = home.home_dir().join("Game");
        assert!(abbreviate_home(&inside.to_string_lossy()).starts_with('~'));
    }
}
