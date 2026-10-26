use std::rc::Rc;

use pgs::{
    PgsDisplaySet, PgsOdsSegment, PgsOdsSequenceFlag, PgsPcsSegment, PgsPdsSegment, PgsPdsSegmentPaletteEntry, PgsSegmentHeader,
    PgsWdsSegment,
};

fn entry(palette_entry_id: u8, luminance: u8, transparency: u8) -> PgsPdsSegmentPaletteEntry {
    PgsPdsSegmentPaletteEntry { palette_entry_id, luminance, color_difference_red: 128, color_difference_blue: 128, transparency }
}

/// A complete 2x2 display set: opaque light pixels on the diagonal, transparent elsewhere.
pub(crate) fn subtitle_display_set(presentation_timestamp: u32) -> PgsDisplaySet {
    let header = PgsSegmentHeader::default;
    PgsDisplaySet {
        pcs: Some(Rc::new(PgsPcsSegment {
            header: PgsSegmentHeader { presentation_timestamp, ..Default::default() },
            ..Default::default()
        })),
        wds: Some(Rc::new(PgsWdsSegment { header: header(), number_of_windows: 0, windows: Vec::new() })),
        pds: Some(Rc::new(PgsPdsSegment {
            header: header(),
            palette_id: 0,
            palette_version_number: 0,
            palette_entries: vec![entry(1, 235, 255), entry(2, 235, 0)],
        })),
        ods: Some(Rc::new(PgsOdsSegment {
            header: header(),
            object_id: 0,
            object_version_number: 0,
            last_in_sequence_flag: PgsOdsSequenceFlag::Both,
            object_data_length: 8,
            width: 2,
            height: 2,
            object_data: vec![1, 2, 0, 0, 2, 1, 0, 0],
        })),
    }
}
