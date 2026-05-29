//! Cron tools: create, list, and delete scheduled cron entries.
//!
//! Shared infrastructure lives in the parent module (reached via `use super::*`).

use super::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct CronCreateTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct CronListTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct CronDeleteTool;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CronJob {
    id: String,
    cron: String,
    prompt: String,
    recurring: bool,
    created_at: std::time::SystemTime,
    last_fired: Option<std::time::SystemTime>,
    fire_count: u64,
    last_status: String,
}

static CRON_JOBS: OnceLock<Mutex<BTreeMap<String, CronJob>>> = OnceLock::new();
static CRON_ID_COUNTER: OnceLock<Mutex<u64>> = OnceLock::new();

impl Tool for CronCreateTool {
    fn name(&self) -> &'static str {
        "CronCreate"
    }

    fn description(&self) -> &'static str {
        "Schedule a prompt using a 5-field cron expression."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(cron) = input.get("cron").and_then(serde_json::Value::as_str) else {
            return ToolResult::error(
                "cron must be a valid 5-field expression, for example `*/5 * * * *` for every 5 minutes",
            );
        };
        if let Some(error) = cron_validation_error(cron) {
            return ToolResult::error(error);
        }
        let Some(prompt) = input
            .get("prompt")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
        else {
            return ToolResult::error("prompt is required");
        };

        let recurring = input
            .get("recurring")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or_else(|| cron_should_default_to_recurring(cron));
        let id = next_cron_id();
        let created_at = std::time::SystemTime::now();
        let job = CronJob {
            id: id.clone(),
            cron: cron.to_owned(),
            prompt: prompt.to_owned(),
            recurring,
            created_at,
            last_fired: None,
            fire_count: 0,
            last_status: String::from("scheduled"),
        };
        cron_jobs()
            .lock()
            .expect("cron jobs lock")
            .insert(id.clone(), job);

        let schedule = cron_describe(cron);
        let task_type = if recurring { "Recurring" } else { "One-shot" };
        let next = cron_next_fire(cron, created_at)
            .map(|time| format!(" (next: {})", format_cron_datetime(time)))
            .unwrap_or_default();
        ToolResult::text(format!(
            "{task_type} task {id} scheduled: {schedule}{next}\nPermission mode switched to YOLO for unattended cron execution. Use Shift+Tab to leave YOLO when you no longer need scheduled tasks to run without prompts."
        ))
    }
}

impl Tool for CronListTool {
    fn name(&self) -> &'static str {
        "CronList"
    }

    fn description(&self) -> &'static str {
        "List all scheduled cron jobs with their IDs and schedules."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, _input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let mut jobs = cron_jobs()
            .lock()
            .expect("cron jobs lock")
            .values()
            .cloned()
            .collect::<Vec<_>>();
        jobs.sort_by(|left, right| left.id.cmp(&right.id));
        if jobs.is_empty() {
            return ToolResult::text("No scheduled cron jobs.");
        }
        let lines = jobs
            .into_iter()
            .map(|job| {
                let task_type = if job.recurring { "Recurring" } else { "One-shot" };
                let schedule = cron_describe(&job.cron);
                let fired = job
                    .last_fired
                    .map(|time| format!(" (last: {})", format_cron_time(time)))
                    .unwrap_or_default();
                let next = cron_next_fire(&job.cron, job.last_fired.unwrap_or(job.created_at))
                    .map(|time| format!(" (next: {})", format_cron_datetime(time)))
                    .unwrap_or_default();
                let prompt = truncate_chars(&job.prompt, 80);
                format!(
                    "[{}] {task_type}: {schedule}{fired}{next} (status: {}, fires: {})\n  Prompt: {prompt}",
                    job.id, job.last_status, job.fire_count
                )
            })
            .collect::<Vec<_>>();
        ToolResult::text(lines.join("\n\n"))
    }
}

impl Tool for CronDeleteTool {
    fn name(&self) -> &'static str {
        "CronDelete"
    }

    fn description(&self) -> &'static str {
        "Cancel a previously scheduled cron job by its ID."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(id) = input
            .get("id")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
        else {
            return ToolResult::error("Missing job ID");
        };
        if cron_jobs()
            .lock()
            .expect("cron jobs lock")
            .remove(id)
            .is_some()
        {
            ToolResult::text(format!("Job {id} deleted."))
        } else {
            ToolResult::error(format!("Job {id} not found."))
        }
    }
}

fn cron_jobs() -> &'static Mutex<BTreeMap<String, CronJob>> {
    CRON_JOBS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn next_cron_id() -> String {
    let mut counter = CRON_ID_COUNTER
        .get_or_init(|| Mutex::new(0))
        .lock()
        .expect("cron id lock");
    *counter += 1;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{:08x}", (nanos as u64) ^ *counter)
}

fn cron_validation_error(cron: &str) -> Option<String> {
    let fields = cron.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 5 {
        return Some(String::from(
            "cron must be a valid 5-field expression, for example `*/5 * * * *` for every 5 minutes",
        ));
    }
    for (field, range, name) in [
        (fields[0], 0..60, "minute"),
        (fields[1], 0..24, "hour"),
        (fields[2], 1..32, "day-of-month"),
        (fields[3], 1..13, "month"),
        (fields[4], 0..7, "day-of-week"),
    ] {
        if !cron_field_is_valid(field, range.clone()) {
            return Some(format!(
                "invalid {name} field `{field}` in cron expression `{cron}`"
            ));
        }
    }
    None
}

fn cron_field_is_valid(field: &str, range: std::ops::Range<i32>) -> bool {
    if field == "*" {
        return true;
    }
    if field.contains(',') {
        return field
            .split(',')
            .all(|part| cron_field_is_valid(part, range.clone()));
    }
    if field.contains('/') {
        let parts = field.split('/').collect::<Vec<_>>();
        let [base, step] = parts.as_slice() else {
            return false;
        };
        let Ok(step) = step.parse::<i32>() else {
            return false;
        };
        if step <= 0 {
            return false;
        }
        if *base == "*" {
            return true;
        }
        return base
            .parse::<i32>()
            .is_ok_and(|value| range.contains(&value));
    }
    if field.contains('-') {
        let parts = field
            .split('-')
            .filter_map(|part| part.parse::<i32>().ok())
            .collect::<Vec<_>>();
        return parts.len() == 2
            && range.contains(&parts[0])
            && range.contains(&parts[1])
            && parts[0] <= parts[1];
    }
    field
        .parse::<i32>()
        .is_ok_and(|value| range.contains(&value))
}

fn cron_field_matches(field: &str, value: i32, range: std::ops::Range<i32>) -> bool {
    if field == "*" {
        return true;
    }
    if field.contains(',') {
        return field
            .split(',')
            .any(|part| cron_field_matches(part, value, range.clone()));
    }
    if field.contains('/') {
        let parts = field.split('/').collect::<Vec<_>>();
        let [base, step] = parts.as_slice() else {
            return false;
        };
        let base = if *base == "*" {
            range.start
        } else {
            base.parse::<i32>().unwrap_or(range.start)
        };
        let step = step.parse::<i32>().unwrap_or(1);
        return step > 0 && range.contains(&base) && value >= base && (value - base) % step == 0;
    }
    if field.contains('-') {
        let parts = field
            .split('-')
            .filter_map(|part| part.parse::<i32>().ok())
            .collect::<Vec<_>>();
        return parts.len() == 2 && value >= parts[0] && value <= parts[1];
    }
    field.parse::<i32>().is_ok_and(|number| number == value)
}

fn cron_next_fire(cron: &str, from: std::time::SystemTime) -> Option<std::time::SystemTime> {
    if cron_validation_error(cron).is_some() {
        return None;
    }
    let fields = cron.split_whitespace().collect::<Vec<_>>();
    let mut candidate = time::OffsetDateTime::from(from)
        .replace_second(0)
        .ok()?
        .replace_nanosecond(0)
        .ok()?
        + time::Duration::minutes(1);
    for _ in 0..(366 * 24 * 60) {
        let weekday = match candidate.weekday() {
            time::Weekday::Sunday => 0,
            time::Weekday::Monday => 1,
            time::Weekday::Tuesday => 2,
            time::Weekday::Wednesday => 3,
            time::Weekday::Thursday => 4,
            time::Weekday::Friday => 5,
            time::Weekday::Saturday => 6,
        };
        if cron_field_matches(fields[0], i32::from(candidate.minute()), 0..60)
            && cron_field_matches(fields[1], i32::from(candidate.hour()), 0..24)
            && cron_field_matches(fields[2], i32::from(candidate.day()), 1..32)
            && cron_field_matches(fields[3], i32::from(candidate.month() as u8), 1..13)
            && cron_field_matches(fields[4], weekday, 0..7)
        {
            return Some(candidate.into());
        }
        candidate += time::Duration::minutes(1);
    }
    None
}

fn cron_should_default_to_recurring(cron: &str) -> bool {
    let fields = cron.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 5 {
        return false;
    }
    let [minute, hour, day, month, weekday] = fields.as_slice() else {
        return false;
    };
    if fields
        .iter()
        .any(|field| field.contains('/') || field.contains(',') || field.contains('-'))
    {
        return true;
    }
    if *minute == "*" || *hour == "*" {
        return true;
    }
    if *day == "*" || *month == "*" {
        return true;
    }
    *weekday != "*"
}

fn cron_describe(cron: &str) -> String {
    let fields = cron.split_whitespace().collect::<Vec<_>>();
    let [minute, hour, day, month, weekday] = fields.as_slice() else {
        return format!("invalid cron: {cron}");
    };
    if *minute == "*" && *hour == "*" && *day == "*" && *month == "*" && *weekday == "*" {
        return String::from("every minute");
    }
    if let Some(mins) = minute.strip_prefix("*/")
        && *hour == "*"
        && *day == "*"
        && *month == "*"
    {
        return format!("every {mins} minutes");
    }
    if *minute == "0" && *hour == "*" && *day == "*" && *month == "*" {
        return String::from("every hour");
    }
    if *month == "*" && *weekday == "*" {
        return format!("at {hour}:{} daily", pad_left(minute, 2));
    }
    if *weekday != "*" {
        return format!(
            "at {hour}:{} on {}",
            pad_left(minute, 2),
            cron_describe_day_of_week(weekday)
        );
    }
    format!("cron: {cron}")
}

fn cron_describe_day_of_week(field: &str) -> String {
    let names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    match field {
        "*" => String::from("every day"),
        "1-5" => String::from("weekdays"),
        "0,6" => String::from("weekends"),
        _ => field
            .parse::<usize>()
            .ok()
            .and_then(|day| names.get(day).copied())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| field.to_owned()),
    }
}

fn pad_left(value: &str, width: usize) -> String {
    if value.len() >= width {
        value.to_owned()
    } else {
        format!("{}{value}", "0".repeat(width - value.len()))
    }
}

fn format_cron_time(time: std::time::SystemTime) -> String {
    let datetime = time::OffsetDateTime::from(time);
    format!(
        "{:02}:{:02}:{:02}",
        datetime.hour(),
        datetime.minute(),
        datetime.second()
    )
}

#[cfg(test)]
mod cron_tests {
    use super::{
        cron_describe, cron_next_fire, cron_should_default_to_recurring, cron_validation_error,
    };
    use time::{Date, Month, OffsetDateTime, Time, UtcOffset};

    #[test]
    fn every_five_minutes_cron_finds_next_boundary() {
        let from = OffsetDateTime::new_in_offset(
            Date::from_calendar_date(2026, Month::May, 11).expect("date"),
            Time::from_hms(10, 1, 0).expect("time"),
            UtcOffset::UTC,
        );

        let next =
            OffsetDateTime::from(cron_next_fire("*/5 * * * *", from.into()).expect("next fire"));

        assert_eq!(next.hour(), 10);
        assert_eq!(next.minute(), 5);
    }

    #[test]
    fn cron_list_field_can_contain_ranges() {
        let from = OffsetDateTime::new_in_offset(
            Date::from_calendar_date(2026, Month::May, 11).expect("date"),
            Time::from_hms(10, 0, 0).expect("time"),
            UtcOffset::UTC,
        );

        let next = OffsetDateTime::from(
            cron_next_fire("0,15-20 * * * *", from.into()).expect("next fire"),
        );

        assert_eq!(next.hour(), 10);
        assert_eq!(next.minute(), 15);
    }

    #[test]
    fn recurring_inference_matches_swift_reference() {
        assert!(cron_should_default_to_recurring("*/5 * * * *"));
        assert!(cron_should_default_to_recurring("0 * * * *"));
        assert!(cron_should_default_to_recurring("0 9 * * *"));
        assert!(!cron_should_default_to_recurring("0 9 11 5 *"));
    }

    #[test]
    fn cron_validation_and_descriptions_match_reference_shapes() {
        let error = cron_validation_error("/5 * * *").expect("error");
        assert!(error.contains("5-field"));
        assert!(error.contains("*/5 * * * *"));

        assert_eq!(cron_describe("* * * * *"), "every minute");
        assert_eq!(cron_describe("*/5 * * * *"), "every 5 minutes");
        assert_eq!(cron_describe("0 * * * *"), "every hour");
        assert_eq!(cron_describe("30 9 * * 1-5"), "at 9:30 on weekdays");
    }
}
