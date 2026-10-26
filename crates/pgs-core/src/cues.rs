use std::fs;
use std::path::Path;

use pgs::{PgsDisplaySet, PgsDisplaySetState, PgsParser};

use crate::error::Result;
use crate::helpers::get_rgba_image;

/// Cue duration used when no following display set ends the cue.
const DEFAULT_CUE_DURATION: u32 = 180_000;

/// Decoded sample subtitle cue used for quick inspection before or during conversion.
#[derive(Clone, Debug)]
pub struct SampleCue {
    pub cue_index: usize,
    pub total_cues: usize,
    pub start_ts: u32,
    pub stop_ts: u32,
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

pub(crate) struct Cue<'a> {
    pub display_set: &'a PgsDisplaySet,
    pub start_ts: u32,
    pub stop_ts: u32,
}

pub(crate) fn parse_file(path: &Path) -> Result<PgsParser<'static>> {
    Ok(PgsParser::parse_bytes(fs::read(path)?)?)
}

/// The complete display sets of a stream, each ending where the next display set starts.
pub(crate) fn cues(display_sets: &[PgsDisplaySet]) -> Vec<Cue<'_>> {
    display_sets
        .iter()
        .enumerate()
        .filter(|(_, display_set)| display_set.state() == PgsDisplaySetState::Complete)
        .map(|(index, display_set)| {
            let start_ts = display_set.pcs.as_ref().map_or(0, |pcs| pcs.header.presentation_timestamp);
            Cue { display_set, start_ts, stop_ts: stop_ts_or_default(display_sets, index, start_ts) }
        })
        .collect()
}

fn stop_ts_or_default(display_sets: &[PgsDisplaySet], index: usize, start_ts: u32) -> u32 {
    display_sets
        .get(index + 1)
        .and_then(|ds| ds.pcs.as_ref())
        .map_or(start_ts.saturating_add(DEFAULT_CUE_DURATION), |pcs| pcs.header.presentation_timestamp)
}

/// Extracts a representative subtitle bitmap frame and total cue count from a `.sup` file.
pub fn extract_sample_cue(path: &Path) -> Result<Option<SampleCue>> {
    let parser = parse_file(path)?;
    let cues = cues(parser.get_display_sets());
    // Prefer the second cue when available (the first cue is occasionally a studio/title card).
    let chosen = usize::from(cues.len() > 1);
    let Some(cue) = cues.get(chosen) else {
        return Ok(None);
    };
    let (width, height, rgba) = get_rgba_image(cue.display_set)?;
    Ok(Some(SampleCue {
        cue_index: chosen + 1,
        total_cues: cues.len(),
        start_ts: cue.start_ts,
        stop_ts: cue.stop_ts,
        width,
        height,
        rgba,
    }))
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use pgs::{PgsPcsSegment, PgsSegmentHeader};

    use super::*;

    fn display_set_at(presentation_timestamp: u32) -> PgsDisplaySet {
        let header = PgsSegmentHeader { presentation_timestamp, ..Default::default() };
        PgsDisplaySet { pcs: Some(Rc::new(PgsPcsSegment { header, ..Default::default() })), ..Default::default() }
    }

    #[test]
    fn stop_timestamp_uses_next_display_set_or_falls_back_for_trailing_cue() {
        let display_sets = [display_set_at(90_000), display_set_at(270_000), PgsDisplaySet::new()];
        assert_eq!(stop_ts_or_default(&display_sets, 0, 90_000), 270_000);
        // Next display set has no PCS, and the last one has no successor at all.
        assert_eq!(stop_ts_or_default(&display_sets, 1, 270_000), 270_000 + DEFAULT_CUE_DURATION);
        assert_eq!(stop_ts_or_default(&display_sets, 2, 450_000), 450_000 + DEFAULT_CUE_DURATION);
        assert_eq!(stop_ts_or_default(&display_sets, 2, u32::MAX), u32::MAX);
    }
}
