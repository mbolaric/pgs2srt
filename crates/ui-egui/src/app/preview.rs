use super::*;

impl PgsApp {
    pub fn request_sample_preview(&mut self, id: usize) {
        let Some(input) = self.items.iter().find(|i| i.id == id).map(|i| i.input.clone()) else {
            return;
        };
        if let Err(e) = self.platform.spawn(sample_job(id, input, self.mailbox())) {
            log::warn!("Cannot load preview sample: {e}");
        }
    }

    pub(super) fn active_preview(&self) -> Option<&PreviewData> {
        if let Some(sel_id) = self.selected_id {
            if let Some(live) = &self.live_preview {
                if live.item_id == sel_id {
                    return Some(live);
                }
            }
            if let Some(sample) = self.sample_previews.get(&sel_id) {
                return Some(sample);
            }
        }
        self.live_preview.as_ref()
    }

    pub(super) fn ensure_preview_texture(&mut self, ctx: &egui::Context) -> Option<(TextureHandle, egui::Vec2)> {
        let (seq, w, h, bytes) = {
            let preview = self.active_preview()?;
            let (w, h, bytes) = preview.rgba.as_ref()?;
            (preview.seq, *w, *h, bytes.clone())
        };

        if w == 0 || h == 0 || bytes.len() < w * h * 4 {
            return None;
        }

        if let Some((cached_seq, handle)) = &self.preview_texture {
            if *cached_seq == seq {
                return Some((handle.clone(), vec2(w as f32, h as f32)));
            }
        }

        let image = ColorImage::from_rgba_unmultiplied([w, h], &bytes[..w * h * 4]);
        let handle = ctx.load_texture("pgs2srt-subtitle-frame", image, TextureOptions::LINEAR);
        self.preview_texture = Some((seq, handle.clone()));
        Some((handle, vec2(w as f32, h as f32)))
    }
}
