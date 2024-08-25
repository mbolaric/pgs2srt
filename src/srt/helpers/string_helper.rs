use std::fmt::Write;

pub fn presentation_ts_into_string(presentation_timestamp: u32) -> String {
    let prep: f64 = presentation_timestamp as f64 / 90.0;

    let temp = prep.floor() as u64;
	let hours = temp / 3600000;
    let minutes = (temp % 3600000) / 60000;
    let seconds = (temp % 60000) / 1000;
    let milliseconds = temp % 1000;

    let mut result = String::new();
    write!(
        &mut result,
        "{:02}:{:02}:{:02},{:03}",
        hours, minutes, seconds, milliseconds
    )
    .unwrap();

    result
}