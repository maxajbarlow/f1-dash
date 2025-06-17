use serde_json::Value;
use timescale::{app_timing::TireDriver, timing::TimingDriver};
use tracing::trace;

use crate::models::{TimingAppDataDriver, TimingDataDriver};

// "LAP1" / "" / "+0.273" / "1L" / "20L"
pub fn parse_gap(gap: String) -> i64 {
    if gap.is_empty() {
        trace!(gap, "gap empty");
        return 0;
    }
    if gap.contains("L") {
        trace!(gap, "gap contains L");
        return 0;
    }
    if let Ok(ms) = gap.replace("+", "").parse::<f64>() {
        trace!(gap, ms, "gap parsed");
        return (ms * 1000.0) as i64;
    }

    trace!(gap, "gap failed to parse");

    return 0;
}

// "1:21.306" / ""
pub fn parse_laptime(lap: String) -> i64 {
    if lap.is_empty() {
        trace!(lap, "laptime empty");
        return 0;
    }
    let parts: Vec<&str> = lap.split(':').collect();
    if parts.len() == 2 {
        if let (Ok(minutes), Ok(seconds)) = (parts[0].parse::<i64>(), parts[1].parse::<f64>()) {
            trace!(lap, "laptime parsed");
            return minutes * 60_000 + (seconds * 1000.0) as i64;
        }
    }
    trace!(lap, "laptime failed to parse");
    return 0;
}

// "26.259" / ""
pub fn parse_sector(sector: String) -> i64 {
    if sector.is_empty() {
        trace!(sector, "sector empty");
        return 0;
    }
    if let Ok(seconds) = sector.parse::<f64>() {
        trace!(sector, "sector parsed");
        return (seconds * 1000.0) as i64;
    }
    trace!(sector, "sector failed to parse");
    return 0;
}

fn str_pointer<'a>(update: Option<&'a Value>, pointer: &str) -> Option<&'a str> {
    update
        .and_then(|v| v.pointer(pointer))
        .and_then(|v| v.as_str())
}

pub fn parse_timing_driver(
    nr: &String,
    lap: Option<i32>,
    driver: &TimingDataDriver,
    update: Option<&Value>,
) -> Option<TimingDriver> {
    let gap = str_pointer(update, "/intervalToPositionAhead/value");
    let leader_gap = str_pointer(update, "/gapToLeader");

    let laptime = str_pointer(update, "/lastLaptime/value");

    let sector_1 = str_pointer(update, "/sectors/0/value");
    let sector_2 = str_pointer(update, "/sectors/1/value");
    let sector_3 = str_pointer(update, "/sectors/2/value");

    if gap.is_none()
        && leader_gap.is_none()
        && laptime.is_none()
        && sector_1.is_none()
        && sector_2.is_none()
        && sector_3.is_none()
    {
        return None;
    }

    let gap_value = gap
        .map(|v| v.to_string())
        .or_else(|| {
            driver
                .interval_to_position_ahead
                .as_ref()
                .map(|i| i.value.clone())
        })
        .unwrap_or_default();

    Some(TimingDriver {
        nr: nr.clone(),
        lap,
        gap: parse_gap(gap_value),
        leader_gap: parse_gap(leader_gap.unwrap_or(&driver.gap_to_leader).to_string()),
        laptime: parse_laptime(laptime.unwrap_or(&driver.last_lap_time.value).to_string()),
        sector_1: parse_sector(sector_1.unwrap_or(&driver.sectors[0].value).to_string()),
        sector_2: parse_sector(sector_2.unwrap_or(&driver.sectors[1].value).to_string()),
        sector_3: parse_sector(sector_3.unwrap_or(&driver.sectors[2].value).to_string()),
    })
}

pub fn parse_tire_driver(
    nr: &String,
    lap: Option<i32>,
    driver: &TimingAppDataDriver,
    update: Option<&Value>,
) -> Option<TireDriver> {
    let update_stint = update
        .and_then(|v| v.pointer("/stints"))
        .and_then(|v| v.as_array())
        .and_then(|v| v.last());

    let last_stint = driver.stints.last();

    let compound = update_stint
        .and_then(|v| v.get("compound"))
        .and_then(|v| v.as_str());

    let laps = update_stint
        .and_then(|v| v.get("totalLaps"))
        .and_then(|v| v.as_i64());

    if compound.is_none() && laps.is_none() {
        return None;
    }

    let compound_value = compound
        .map(|v| v.to_string())
        .or_else(|| last_stint.and_then(|s| s.compound.clone()))
        .unwrap_or_default();

    let laps_value = laps
        .map(|v| v as i32)
        .or_else(|| last_stint.and_then(|s| s.total_laps))
        .unwrap_or(0);

    Some(TireDriver {
        nr: nr.clone(),
        lap,
        compound: compound_value,
        laps: laps_value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        IntervalToPositionAhead, I1, PersonalBestLapTime, Sector, Stint,
        TimingAppDataDriver, TimingDataDriver,
    };
    use serde_json::json;

    fn default_sector(value: &str) -> Sector {
        Sector {
            stopped: false,
            value: value.to_string(),
            previous_value: None,
            status: 0,
            overall_fastest: false,
            personal_fastest: false,
            segments: vec![],
        }
    }

    fn default_driver() -> TimingDataDriver {
        TimingDataDriver {
            stats: None,
            time_diff_to_fastest: None,
            time_diff_to_position_ahead: None,
            gap_to_leader: "+5.000".to_string(),
            interval_to_position_ahead: Some(IntervalToPositionAhead {
                value: "+0.273".to_string(),
                catching: false,
            }),
            line: 1,
            racing_number: "1".to_string(),
            sectors: vec![
                default_sector("26.259"),
                default_sector("26.880"),
                default_sector("27.093"),
            ],
            best_lap_time: PersonalBestLapTime {
                value: "1:20.000".to_string(),
            },
            last_lap_time: I1 {
                value: "1:21.000".to_string(),
                status: 0,
                overall_fastest: false,
                personal_fastest: false,
            },
        }
    }

    #[test]
    fn timing_driver_returns_none_when_no_updates() {
        let driver = default_driver();
        let result = parse_timing_driver(&"1".to_string(), Some(1), &driver, Some(&json!({})));
        assert!(result.is_none());
    }

    #[test]
    fn timing_driver_parses_partial_update() {
        let driver = default_driver();
        let update = json!({ "lastLaptime": { "value": "1:22.000" } });
        let parsed = parse_timing_driver(&"1".to_string(), Some(2), &driver, Some(&update)).unwrap();

        assert_eq!(parsed.laptime, 82000);
        assert_eq!(parsed.gap, 273);
        assert_eq!(parsed.leader_gap, 5000);
    }

    fn default_stint() -> Stint {
        Stint {
            total_laps: Some(10),
            compound: Some("SOFT".to_string()),
            is_new: None,
        }
    }

    #[test]
    fn tire_driver_returns_none_when_no_updates() {
        let driver = TimingAppDataDriver {
            racing_number: "1".to_string(),
            stints: vec![default_stint()],
            line: 1,
            grid_pos: "1".to_string(),
        };

        let result = parse_tire_driver(&"1".to_string(), Some(1), &driver, Some(&json!({})));
        assert!(result.is_none());
    }

    #[test]
    fn tire_driver_parses_update() {
        let driver = TimingAppDataDriver {
            racing_number: "1".to_string(),
            stints: vec![default_stint()],
            line: 1,
            grid_pos: "1".to_string(),
        };

        let update = json!({ "stints": [ { "compound": "MEDIUM", "totalLaps": 5 } ] });
        let parsed = parse_tire_driver(&"1".to_string(), Some(2), &driver, Some(&update)).unwrap();

        assert_eq!(parsed.compound, "MEDIUM");
        assert_eq!(parsed.laps, 5);
    }
}
