use tachyonfx::{fx, Duration, Effect, Interpolation, Motion, IntoEffect};
use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct EffectInstance {
    pub effect: Effect,
    pub position: (usize, usize),
    pub effect_type: EffectType,
    pub remaining_time: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EffectType {
    Explosion,
    MoveTrail,
    Highlight,
    Selection,
    Check,
    BackgroundMatrix,
    PieceGlow,
    ValidMoveHint,
}

pub struct EffectManager {
    effects: Vec<EffectInstance>,
    background_effects: Vec<Effect>,
    board_highlight_positions: Vec<(usize, usize)>,
    selected_position: Option<(usize, usize)>,
    valid_move_positions: Vec<(usize, usize)>,
    check_position: Option<(usize, usize)>,
}

impl EffectManager {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
            background_effects: vec![
                Self::create_background_matrix(),
                Self::create_ambient_effects(),
            ],
            board_highlight_positions: Vec::new(),
            selected_position: None,
            valid_move_positions: Vec::new(),
            check_position: None,
        }
    }

    pub fn update(&mut self, elapsed: Duration) {
        // Update all effects
        self.effects.retain_mut(|effect_instance| {
            if effect_instance.remaining_time > elapsed {
                effect_instance.remaining_time = effect_instance.remaining_time - elapsed;
                true
            } else {
                false
            }
        });

        // Update background effects
        for _bg_effect in &mut self.background_effects {
            // Background effects run continuously
        }
    }

    pub fn add_explosion(&mut self, position: (usize, usize)) {
        let explosion_effect = Self::create_explosion_effect();
        self.effects.push(EffectInstance {
            effect: explosion_effect,
            position,
            effect_type: EffectType::Explosion,
            remaining_time: Duration::from_millis(1500),
        });
    }

    pub fn add_move_trail(&mut self, from: (usize, usize), to: (usize, usize)) {
        let trail_effect = Self::create_move_trail_effect();
        self.effects.push(EffectInstance {
            effect: trail_effect,
            position: from,
            effect_type: EffectType::MoveTrail,
            remaining_time: Duration::from_millis(800),
        });

        // Add glowing effect at destination
        let glow_effect = Self::create_piece_glow_effect();
        self.effects.push(EffectInstance {
            effect: glow_effect,
            position: to,
            effect_type: EffectType::PieceGlow,
            remaining_time: Duration::from_millis(1200),
        });
    }

    pub fn set_selected_position(&mut self, position: Option<(usize, usize)>) {
        self.selected_position = position;
    }

    pub fn set_valid_moves(&mut self, positions: Vec<(usize, usize)>) {
        self.valid_move_positions = positions;
    }

    pub fn set_check_position(&mut self, position: Option<(usize, usize)>) {
        self.check_position = position;
    }

    pub fn get_selection_effect(&self) -> Option<Effect> {
        if self.selected_position.is_some() {
            Some(Self::create_selection_effect())
        } else {
            None
        }
    }

    pub fn get_valid_move_effects(&self) -> Vec<Effect> {
        self.valid_move_positions.iter()
            .map(|_| Self::create_valid_move_hint_effect())
            .collect()
    }

    pub fn get_check_effect(&self) -> Option<Effect> {
        if self.check_position.is_some() {
            Some(Self::create_check_warning_effect())
        } else {
            None
        }
    }

    pub fn get_background_effects(&self) -> &Vec<Effect> {
        &self.background_effects
    }

    pub fn get_active_effects(&self) -> &Vec<EffectInstance> {
        &self.effects
    }

    // Effect creation methods
    fn create_explosion_effect() -> Effect {
        fx::parallel(&[
            fx::explode(
                2.0, 1.0, // force and force_rng_factor
                (1000, Interpolation::QuadOut)
            ),
            fx::sequence(&[
                fx::fade_to(
                    Color::Red,
                    Color::Yellow,
                    (200, Interpolation::QuadOut)
                ),
                fx::fade_to(
                    Color::Yellow,
                    Color::Red,
                    (300, Interpolation::QuadIn)
                ),
                fx::dissolve((500, Interpolation::QuadOut))
            ]),
            fx::hsl_shift(
                Some([0.0, 100.0, 50.0]),  // High saturation red
                Some([30.0, 100.0, 30.0]), // Orange background
                (800, Interpolation::QuadOut)
            ),
        ])
    }

    fn create_move_trail_effect() -> Effect {
        fx::sequence(&[
            fx::fade_from(
                Color::Cyan,
                Color::Blue,
                (300, Interpolation::QuadOut)
            ),
            fx::sweep_out(
                Motion::LeftToRight,
                8,
                2,
                Color::Blue,
                (500, Interpolation::QuadInOut)
            ),
        ])
    }

    fn create_piece_glow_effect() -> Effect {
        fx::parallel(&[
            fx::ping_pong(
                fx::hsl_shift(
                    Some([60.0, 30.0, 20.0]), // Golden glow
                    None,
                    (400, Interpolation::SineInOut)
                )
            ),
            fx::fade_from_fg(
                Color::Yellow,
                (600, Interpolation::QuadOut)
            ),
        ])
    }

    fn create_selection_effect() -> Effect {
        fx::repeating(
            fx::parallel(&[
                fx::fade_from(
                    Color::Green,
                    Color::LightGreen,
                    (500, Interpolation::SineInOut)
                ),
                fx::ping_pong(
                    fx::hsl_shift_fg(
                        [120.0, 50.0, 10.0], // Green highlight
                        (750, Interpolation::SineInOut)
                    )
                ),
            ])
        )
    }

    fn create_valid_move_hint_effect() -> Effect {
        fx::repeating(
            fx::sequence(&[
                fx::fade_from_fg(
                    Color::LightBlue,
                    (400, Interpolation::SineInOut)
                ),
                fx::fade_to_fg(
                    Color::Blue,
                    (400, Interpolation::SineInOut)
                ),
            ])
        )
    }

    fn create_check_warning_effect() -> Effect {
        fx::repeating(
            fx::parallel(&[
                fx::fade_from(
                    Color::Red,
                    Color::LightRed,
                    (300, Interpolation::SineInOut)
                ),
                fx::ping_pong(
                    fx::hsl_shift(
                        Some([0.0, 100.0, 30.0]), // Intense red
                        Some([0.0, 80.0, 20.0]),  // Dark red background
                        (400, Interpolation::SineInOut)
                    )
                ),
            ])
        )
    }

    fn create_background_matrix() -> Effect {
        fx::never_complete(
            fx::fade_from(Color::Green, Color::Black, (5000, Interpolation::Linear)).into_effect()
        )
    }

    fn create_ambient_effects() -> Effect {
        fx::never_complete(
            fx::parallel(&[
                fx::fade_from(Color::Blue, Color::Black, (3000, Interpolation::SineInOut)).into_effect(),
                fx::hsl_shift(
                    None,
                    Some([0.0, 5.0, 2.0]), // Subtle background color shift
                    (8000, Interpolation::SineInOut)
                ),
            ])
        )
    }

    pub fn create_menu_effects() -> Vec<Effect> {
        vec![
            fx::never_complete(
                fx::parallel(&[
                    fx::fade_from(Color::Red, Color::Black, (4000, Interpolation::Linear)).into_effect(),
                    fx::fade_from(Color::Yellow, Color::Black, (6000, Interpolation::Linear)).into_effect(),
                ])
            ),
            fx::never_complete(
                fx::sequence(&[
                    fx::hsl_shift(
                        None,
                        Some([30.0, 20.0, 5.0]), // Warm background
                        (5000, Interpolation::SineInOut)
                    ),
                    fx::hsl_shift(
                        None,
                        Some([-30.0, 20.0, 5.0]), // Cool background
                        (5000, Interpolation::SineInOut)
                    ),
                ])
            ),
        ]
    }

    pub fn create_victory_effects() -> Vec<Effect> {
        vec![
            fx::sequence(&[
                fx::explode(3.0, 1.5, (2000, Interpolation::QuadOut)),
                fx::parallel(&[
                    fx::fade_from(
                        Color::Yellow,
                        Color::Yellow,
                        (1000, Interpolation::QuadOut)
                    ),
                    fx::hsl_shift(
                        Some([45.0, 100.0, 30.0]), // Golden celebration
                        Some([60.0, 80.0, 20.0]),
                        (2000, Interpolation::QuadOut)
                    ),
                ]),
            ]),
            fx::repeating(
                fx::parallel(&[
                    fx::fade_from(Color::Cyan, Color::Black, (3000, Interpolation::Linear)).into_effect(),
                    fx::fade_from(Color::Magenta, Color::Black, (2000, Interpolation::SineInOut)).into_effect(),
                ])
            ),
        ]
    }

    pub fn create_game_over_effects() -> Vec<Effect> {
        vec![
            fx::sequence(&[
                fx::fade_to(
                    Color::DarkGray,
                    Color::Black,
                    (1500, Interpolation::QuadInOut)
                ),
                fx::dissolve((1000, Interpolation::QuadOut)),
            ]),
            fx::never_complete(
                fx::hsl_shift(
                    Some([0.0, 50.0, -20.0]), // Desaturated, darker
                    Some([0.0, 30.0, -30.0]),
                    (4000, Interpolation::SineInOut)
                )
            ),
        ]
    }

    pub fn add_piece_capture_celebration(&mut self, position: (usize, usize)) {
        let celebration_effect = fx::sequence(&[
            fx::parallel(&[
                fx::explode(2.5, 1.2, (800, Interpolation::BounceOut)),
                fx::hsl_shift(
                    Some([120.0, 100.0, 40.0]), // Bright green celebration
                    None,
                    (600, Interpolation::QuadOut)
                ),
            ]),
            fx::coalesce((400, Interpolation::BounceOut)),
        ]);

        self.effects.push(EffectInstance {
            effect: celebration_effect,
            position,
            effect_type: EffectType::Explosion,
            remaining_time: Duration::from_millis(1200),
        });
    }

    pub fn add_promotion_effect(&mut self, position: (usize, usize)) {
        let promotion_effect = fx::parallel(&[
            fx::sequence(&[
                fx::fade_from(
                    Color::Magenta,
                    Color::LightMagenta,
                    (300, Interpolation::QuadOut)
                ),
                fx::ping_pong(
                    fx::hsl_shift_fg(
                        [270.0, 80.0, 30.0], // Purple sparkle
                        (500, Interpolation::SineInOut)
                    )
                ),
            ]),
            fx::fade_from(Color::White, Color::Black, (800, Interpolation::Linear)).into_effect(),
        ]);

        self.effects.push(EffectInstance {
            effect: promotion_effect,
            position,
            effect_type: EffectType::PieceGlow,
            remaining_time: Duration::from_millis(1300),
        });
    }

    pub fn add_castling_effect(&mut self, king_pos: (usize, usize), rook_pos: (usize, usize)) {
        let castling_effect = fx::parallel(&[
            fx::slide_in(
                Motion::LeftToRight,
                6,
                1,
                Color::Blue,
                (800, Interpolation::QuadInOut)
            ),
            fx::hsl_shift(
                Some([240.0, 60.0, 20.0]), // Royal blue
                None,
                (1000, Interpolation::QuadOut)
            ),
        ]);

        // Add effect for both king and rook
        self.effects.push(EffectInstance {
            effect: castling_effect.clone(),
            position: king_pos,
            effect_type: EffectType::MoveTrail,
            remaining_time: Duration::from_millis(1000),
        });

        self.effects.push(EffectInstance {
            effect: castling_effect,
            position: rook_pos,
            effect_type: EffectType::MoveTrail,
            remaining_time: Duration::from_millis(1000),
        });
    }
} 