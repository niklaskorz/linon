// Based on https://github.com/emilk/egui/blob/0.15.0/egui_demo_lib/src/syntax_highlighting.rs
// MIT License
use egui::{text::LayoutJob, FontId, FontFamily};

/// View some code with syntax highlighting and selection.
pub fn code_view_ui<S: egui::TextBuffer>(ui: &mut egui::Ui, code: &mut S) -> egui::Response {
    let language = "rs";

    let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
        let layout_job = highlight(ui.ctx(), string, language);
        // layout_job.wrap.max_width = wrap_width; // no wrapping
        ui.fonts(|f| f.layout_job(layout_job))
    };

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add(
            egui::TextEdit::multiline(code)
                .font(FontId::new(14.0, FontFamily::Monospace))
                .code_editor()
                .desired_rows(10)
                .lock_focus(true)
                .layouter(&mut layouter),
        )
    }).inner
}

/// Memoized Code highlighting
pub fn highlight(ctx: &egui::Context, code: &str, language: &str) -> LayoutJob {
    impl egui::util::cache::ComputerMut<(&str, &str), LayoutJob> for Highlighter {
        fn compute(&mut self, (code, lang): (&str, &str)) -> LayoutJob {
            self.highlight(code, lang)
        }
    }

    type HighlightCache = egui::util::cache::FrameCache<LayoutJob, Highlighter>;

    ctx.memory_mut(|mem| {
        mem.caches
            .cache::<HighlightCache>()
            .get((code, language))
    })
}

// ----------------------------------------------------------------------------

struct Highlighter {
    ps: syntect::parsing::SyntaxSet,
    ts: syntect::highlighting::ThemeSet,
}

impl Default for Highlighter {
    fn default() -> Self {
        Self {
            ps: syntect::parsing::SyntaxSet::load_defaults_newlines(),
            ts: syntect::highlighting::ThemeSet::load_defaults(),
        }
    }
}

impl Highlighter {
    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    fn highlight(&self, code: &str, lang: &str) -> LayoutJob {
        self.highlight_impl(code, lang).unwrap_or_else(|| {
            // Fallback:
            LayoutJob::simple(
                code.into(),
                FontId::new(14.0, FontFamily::Monospace),
                egui::Color32::LIGHT_GRAY,
                f32::INFINITY,
            )
        })
    }

    fn highlight_impl(&self, text: &str, language: &str) -> Option<LayoutJob> {
        use syntect::easy::HighlightLines;
        use syntect::highlighting::FontStyle;
        use syntect::util::LinesWithEndings;

        let syntax = self
            .ps
            .find_syntax_by_name(language)
            .or_else(|| self.ps.find_syntax_by_extension(language))?;

        let theme = "base16-mocha.dark";
        let mut h = HighlightLines::new(syntax, &self.ts.themes[theme]);

        use egui::text::{LayoutSection, TextFormat};

        let mut job = LayoutJob {
            text: text.into(),
            ..Default::default()
        };

        for line in LinesWithEndings::from(text) {
            for (style, range) in h.highlight_line(line, &self.ps).ok()? {
                let fg = style.foreground;
                let text_color = egui::Color32::from_rgb(fg.r, fg.g, fg.b);
                let italics = style.font_style.contains(FontStyle::ITALIC);
                let underline = style.font_style.contains(FontStyle::ITALIC);
                let underline = if underline {
                    egui::Stroke::new(1.0, text_color)
                } else {
                    egui::Stroke::NONE
                };
                job.sections.push(LayoutSection {
                    leading_space: 0.0,
                    byte_range: as_byte_range(text, range),
                    format: TextFormat {
                        font_id: FontId::new(14.0, FontFamily::Monospace),
                        color: text_color,
                        italics,
                        underline,
                        ..Default::default()
                    },
                });
            }
        }

        Some(job)
    }
}

fn as_byte_range(whole: &str, range: &str) -> std::ops::Range<usize> {
    let whole_start = whole.as_ptr() as usize;
    let range_start = range.as_ptr() as usize;
    assert!(whole_start <= range_start);
    assert!(range_start + range.len() <= whole_start + whole.len());
    let offset = range_start - whole_start;
    offset..(offset + range.len())
}
