use alloc::boxed::Box;

use bon::Builder;
use ratatui_core::{buffer::Buffer, layout::Rect, style::Color};

use crate::{
    cell_filter::FilterProcessor,
    default_shader_impl,
    effect_timer::EffectTimer,
    pattern::{AnyPattern, InstancedPattern, Pattern},
    shader::Shader,
    CellFilter, ColorCache, ColorSpace, Duration,
};

#[derive(Builder, Clone, Debug)]
pub(super) struct FadeColors {
    fg: Option<Color>,
    bg: Option<Color>,
    #[builder(into)]
    timer: EffectTimer,
    area: Option<Rect>,
    cell_filter: Option<FilterProcessor>,
    color_space: ColorSpace,
    #[builder(default)]
    pattern: AnyPattern,
}

impl Shader for FadeColors {
    default_shader_impl!(area, timer, filter, color_space, clone);

    fn name(&self) -> &'static str {
        if self.timer.is_reversed() {
            "fade_from"
        } else {
            "fade_to"
        }
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let global_alpha = self.timer.alpha();
        let fg = self.fg;
        let bg = self.bg;
        let color_space = self.color_space;

        let mut pattern = self.pattern.clone().for_frame(global_alpha, area);
        let cell_iter = self.cell_iter(buf, area);
        let mut color_cache: ColorCache<(Color, u8), 8> = ColorCache::new();

        cell_iter.for_each_cell(move |pos, cell| {
            let alpha = pattern.map_alpha(pos);

            let cache_key = (alpha.clamp(0.0, 1.0) * 255.0) as u8;

            if let Some(fg) = fg.as_ref() {
                let color = color_cache.memoize_fg(cell.fg, (*fg, cache_key), |c| {
                    color_space.lerp(c, fg, alpha)
                });
                cell.set_fg(color);
            }

            if let Some(bg) = bg.as_ref() {
                let color = color_cache.memoize_bg(cell.bg, (*bg, cache_key), |c| {
                    color_space.lerp(c, bg, alpha)
                });
                cell.set_bg(color);
            }
        });
    }

    fn set_pattern(&mut self, pattern: AnyPattern) {
        self.pattern = pattern;
    }

    #[cfg(feature = "dsl")]
    fn to_dsl(&self) -> Result<crate::dsl::EffectExpression, crate::dsl::DslError> {
        use crate::dsl::DslFormat;

        let s = if let Some(bg) = self.bg {
            format!(
                "fx::{}({}, {}, {})",
                self.name(),
                self.fg.unwrap().dsl_format(),
                bg.dsl_format(),
                self.timer.dsl_format(),
            )
        } else {
            format!(
                "fx::{}_fg({}, {})",
                self.name(),
                self.fg.unwrap().dsl_format(),
                self.timer.dsl_format()
            )
        };
        crate::dsl::EffectExpression::parse(&s)
    }
}

#[cfg(test)]
mod plain_test {
    use ratatui_core::{buffer::Buffer, layout::Rect, style::Color};

    use crate::{fx, pattern::SweepPattern, ColorSpace, Duration, ToRgbComponents};

    #[test]
    fn test_fade_with_sweep_patterns() {
        let area = Rect::new(0, 0, 10, 3);

        // Helper function to setup and run fade test
        let test_fade_at_progress = |pattern: SweepPattern, progress_ms: u32| -> Buffer {
            let mut fade = fx::fade_to_fg(Color::Red, 1000).with_pattern(pattern);
            let mut buf = Buffer::empty(area);
            // Fill with white background
            for y in 0..3 {
                for x in 0..10 {
                    buf[(x, y)].fg = Color::White;
                }
            }
            fade.process(Duration::from_millis(progress_ms as _), &mut buf, area);
            buf
        };

        // Test left_to_right sweep at 25% and 75% progress
        let left_25 = test_fade_at_progress(SweepPattern::left_to_right(5), 250);
        let left_75 = test_fade_at_progress(SweepPattern::left_to_right(5), 750);

        // Test right_to_left sweep at 25% and 75% progress
        let right_25 = test_fade_at_progress(SweepPattern::right_to_left(5), 250);
        let right_75 = test_fade_at_progress(SweepPattern::right_to_left(5), 750);

        // At 25% progress, left_to_right should have more effect on left side
        // At 75% progress, left_to_right should have more effect on right side
        let left_25_left = left_25[(2, 0)].fg;
        let left_25_right = left_25[(7, 0)].fg;
        let left_75_left = left_75[(2, 0)].fg;
        let left_75_right = left_75[(7, 0)].fg;

        // At 25% progress, right_to_left should have more effect on right side
        // At 75% progress, right_to_left should have more effect on left side
        let right_25_left = right_25[(2, 0)].fg;
        let right_25_right = right_25[(7, 0)].fg;
        let right_75_left = right_75[(2, 0)].fg;
        let right_75_right = right_75[(7, 0)].fg;

        // For left_to_right: at 25%, left should be more faded than right
        assert_ne!(
            left_25_left, left_25_right,
            "Left-to-right at 25% should show gradient"
        );

        // For right_to_left: at 25%, right should be more faded than left
        assert_ne!(
            right_25_left, right_25_right,
            "Right-to-left at 25% should show gradient"
        );

        // The sweep directions should produce opposite patterns
        // At 25%: left_to_right affects left more, right_to_left affects right more
        assert_ne!(
            left_25_left, right_25_left,
            "Different directions should affect left side differently at 25%"
        );
        assert_ne!(
            left_25_right, right_25_right,
            "Different directions should affect right side differently at 25%"
        );

        // At 75%: effect should have progressed in the respective directions
        assert_ne!(
            left_75_left, left_75_right,
            "Left-to-right at 75% should show gradient"
        );
        assert_ne!(
            right_75_left, right_75_right,
            "Right-to-left at 75% should show gradient"
        );
    }

    #[test]
    fn test_fade_reversal_works_correctly() {
        let area = Rect::new(0, 0, 1, 1);
        let mut buf1 = Buffer::empty(area);
        let mut buf2 = Buffer::empty(area);

        // Set initial color to white
        buf1[(0, 0)].fg = Color::White;
        buf2[(0, 0)].fg = Color::White;

        // Test at 25% progress where effects should clearly differ
        let mut normal_fade = fx::fade_to_fg(Color::Red, 1000);
        let mut reversed_fade = fx::fade_from_fg(Color::Red, 1000);

        normal_fade.process(Duration::from_millis(250), &mut buf1, area);
        reversed_fade.process(Duration::from_millis(250), &mut buf2, area);

        let normal_color = buf1[(0, 0)].fg;
        let reversed_color = buf2[(0, 0)].fg;

        // At 25%:
        // - fade_to_fg: alpha=0.25, lerp(White, Red, 0.25) = mostly white
        // - fade_from_fg: alpha=0.75, lerp(White, Red, 0.75) = mostly red
        assert_ne!(
            normal_color, reversed_color,
            "fade_to_fg and fade_from_fg should produce different results"
        );
    }

    #[test]
    fn test_fade_in_with_sweep_pattern() {
        // Create a fade effect with sweep pattern (left to right)
        let mut effect =
            fx::fade_from_fg(Color::Red, 1000).with_pattern(SweepPattern::left_to_right(20));

        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);
        // Fill entire area with same color to test sweep effect
        for y in 0..5 {
            for x in 0..20 {
                buf[(x, y)].fg = Color::White;
            }
        }

        // Process partway through the effect
        effect.process(Duration::from_millis(500), &mut buf, area);

        // With left_to_right sweep pattern, cells on the left should have different
        // alpha values than cells on the right due to the sweeping pattern
        let left_color = buf.cell((0, 0)).unwrap().fg;
        let middle_color = buf.cell((10, 0)).unwrap().fg;
        let right_color = buf.cell((19, 0)).unwrap().fg;

        // The sweep pattern should create different alpha values across positions
        // We can't predict exact colors due to timing/interpolation complexity,
        // but we can verify that the pattern is being applied by checking
        // that not all positions have identical colors
        let colors_differ = left_color != middle_color || middle_color != right_color;

        assert!(
            colors_differ,
            "Sweep pattern should create different colors at different positions. \
            Left: {left_color:?}, Middle: {middle_color:?}, Right: {right_color:?}"
        );
    }

    #[test]
    fn test_fade_over_buffer_reset_cells() {
        use ratatui_core::{buffer::Buffer, layout::Rect, style::Color};

        use crate::{fx, Duration};

        let area = Rect::new(0, 0, 1, 1);
        let mut buf = Buffer::empty(area);

        let mut fade_effect = fx::fade_to_fg(Color::Black, 1000).with_color_space(ColorSpace::Rgb);

        // Process the effect halfway
        fade_effect.process(Duration::from_millis(500), &mut buf, area);

        // Check that cells have been modified (not equal to initial colors)
        let cell = &buf[(0, 0)];

        // confirm 50% fade towards white
        let (r, g, b) = Color::White.to_rgb();
        assert_eq!(cell.fg, Color::Rgb(r / 2, g / 2, b / 2));
    }
}

#[cfg(test)]
#[cfg(feature = "dsl")]
mod dsl_tests {
    use indoc::indoc;
    use ratatui_core::style::Color;

    use crate::{effect_timer::EffectTimer, fx, Interpolation::QuadOut};

    #[test]
    fn to_dsl_fade_to_fg() {
        let dsl = fx::fade_to_fg(Color::from_u32(0), EffectTimer::from_ms(1000, QuadOut))
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, indoc! {
            "fx::fade_to_fg(Color::from_u32(0), EffectTimer::from_ms(1000, Interpolation::QuadOut))"
        });
    }

    #[test]
    fn to_dsl_fade_to() {
        let dsl = fx::fade_to(
            Color::from_u32(0),
            Color::from_u32(0),
            EffectTimer::from_ms(1000, QuadOut),
        )
        .to_dsl()
        .unwrap()
        .to_string();

        assert_eq!(dsl, indoc! {
            "fx::fade_to(
                     Color::from_u32(0),
                     Color::from_u32(0),
                     EffectTimer::from_ms(1000, Interpolation::QuadOut)
                 )"
        });
    }

    #[test]
    fn to_dsl_fade_from_fg() {
        let dsl = fx::fade_from_fg(Color::from_u32(0), EffectTimer::from_ms(1000, QuadOut))
            .to_dsl()
            .unwrap()
            .to_string();

        assert_eq!(dsl, indoc! {
            "fx::fade_from_fg(Color::from_u32(0), EffectTimer::from_ms(1000, Interpolation::QuadOut))"
        });
    }

    #[test]
    fn to_dsl_fade_from() {
        let dsl = fx::fade_from(
            Color::from_u32(0),
            Color::from_u32(0),
            EffectTimer::from_ms(1000, QuadOut),
        )
        .to_dsl()
        .unwrap()
        .to_string();

        assert_eq!(dsl, indoc! {
            "fx::fade_from(
                     Color::from_u32(0),
                     Color::from_u32(0),
                     EffectTimer::from_ms(1000, Interpolation::QuadOut)
                 )"
        });
    }
}
