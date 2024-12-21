use std::time::{Duration, SystemTime};

use http::response::Parts;
use httpdate::HttpDate;

pub(crate) fn get_expiration_time(
    request_time: &SystemTime,
    response_time: &SystemTime,
    response_parts: &Parts,
) -> Option<SystemTime> {
    response_parts.cac
}

// https://httpwg.org/specs/rfc9111.html#rfc.section.4.2.3
pub(crate) fn get_current_age(params: &GetAgeParams) -> Option<Duration> {
    return get_age_at(params, &SystemTime::now());
}

pub(crate) fn get_age_at(params: &GetAgeParams, system_time: &SystemTime) -> Option<Duration> {
    let corrected_initial_age = std::cmp::max(
        get_apparent_age(params).unwrap_or(Duration::ZERO),
        get_corrected_age_value(params)?,
    );

    let resident_time = system_time.duration_since(params.response_time).ok()?;

    let current_age = corrected_initial_age.saturating_add(resident_time);

    return Some(current_age);
}

fn get_apparent_age(params: &GetAgeParams) -> Option<Duration> {
    let response_header_date = params.response_header_date?;

    return Some(
        params
            .response_time
            .duration_since(response_header_date)
            .unwrap_or(Duration::ZERO),
    );
}

fn get_corrected_age_value(params: &GetAgeParams) -> Option<Duration> {
    let response_delay = params
        .response_time
        .duration_since(params.request_time)
        .ok()?;

    return Some(
        params
            .response_header_age
            .unwrap_or(Duration::ZERO)
            .saturating_add(response_delay),
    );
}

pub(crate) struct GetAgeParams {
    request_time: SystemTime,
    response_time: SystemTime,
    response_header_age: Option<Duration>,
    response_header_date: Option<SystemTime>,
}

impl GetAgeParams {
    pub(crate) fn new(
        request_time: SystemTime,
        response_time: SystemTime,
        response_parts: &Parts,
    ) -> GetAgeParams {
        return GetAgeParams {
            request_time,
            response_time,
            response_header_age: Self::get_response_header_age(response_parts),
            response_header_date: Self::get_response_header_date(response_parts),
        };
    }

    fn get_response_header_age(response_parts: &Parts) -> Option<Duration> {
        return match response_parts.headers.get("Age") {
            Some(age_header_value) => match age_header_value.to_str() {
                Ok(age_str) => match age_str.parse::<u32>() {
                    Ok(age) => Some(Duration::from_secs(age.into())),
                    Err(err) => match err.kind() {
                        std::num::IntErrorKind::PosOverflow => Some(Duration::MAX),
                        _ => None,
                    },
                },
                Err(_) => None,
            },
            None => None,
        };
    }

    fn get_response_header_date(response_parts: &Parts) -> Option<SystemTime> {
        let http_date: HttpDate = response_parts
            .headers
            .get("Date")
            .and_then(|date_header_value| date_header_value.to_str().ok())
            .and_then(|date_header_str| date_header_str.parse().ok())?;

        return Some(http_date.into());
    }
}
