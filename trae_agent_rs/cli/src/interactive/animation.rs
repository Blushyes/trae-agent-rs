//! Animation system for interactive UI
//! 
//! This module provides animation configuration, easing functions,
//! and animation utilities for the interactive user interface.

/// Easing options for token animation
#[derive(Debug, Clone, Copy)]
pub enum Easing {
    Linear,
    EaseOutCubic,
    EaseInOutCubic,
}

/// Apply easing function to a normalized time value (0.0 to 1.0)
pub fn apply_easing(easing: Easing, t: f64) -> f64 {
    match easing {
        Easing::Linear => t,
        Easing::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
        Easing::EaseInOutCubic => {
            if t < 0.5 {
                4.0 * t * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
            }
        }
    }
}

/// Configuration for UI animations
#[derive(Debug, Clone)]
pub struct UiAnimationConfig {
    pub easing: Easing,
    pub frame_interval_ms: u64,
    pub duration_ms: u64,
}

impl UiAnimationConfig {
    /// Create new animation config with environment variable overrides
    pub fn from_env() -> Self {
        // Load UI animation config from env (fallback to defaults)
        let easing = std::env::var("TRAE_UI_EASING")
            .ok()
            .and_then(|v| match v.to_lowercase().as_str() {
                "linear" => Some(Easing::Linear),
                "ease_in_out_cubic" | "easeinoutcubic" | "ease-in-out-cubic" => {
                    Some(Easing::EaseInOutCubic)
                }
                "ease_out_cubic" | "easeoutcubic" | "ease-out-cubic" => Some(Easing::EaseOutCubic),
                _ => None,
            })
            .unwrap_or(Easing::EaseOutCubic);
        
        let frame_interval_ms = std::env::var("TRAE_UI_FRAME_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(10);
        
        let duration_ms = std::env::var("TRAE_UI_DURATION_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(3000);

        Self {
            easing,
            frame_interval_ms,
            duration_ms,
        }
    }

    /// Create default animation config
    pub fn default() -> Self {
        Self {
            easing: Easing::EaseOutCubic,
            frame_interval_ms: 10,
            duration_ms: 3000,
        }
    }
}

/// Animation state for token counting
#[derive(Debug, Clone)]
pub struct TokenAnimation {
    pub current_tokens: u32,
    pub target_tokens: u32,
    pub start_time: std::time::Instant,
    pub duration: std::time::Duration,
}

impl TokenAnimation {
    /// Create new token animation
    pub fn new(target_tokens: u32, duration: std::time::Duration) -> Self {
        Self {
            current_tokens: 0,
            target_tokens,
            start_time: std::time::Instant::now(),
            duration,
        }
    }

    /// Update animation and return current token count
    pub fn update(&mut self, easing: Easing) -> u32 {
        if self.current_tokens >= self.target_tokens {
            return self.target_tokens;
        }

        let elapsed = self.start_time.elapsed();
        if elapsed >= self.duration {
            self.current_tokens = self.target_tokens;
            return self.target_tokens;
        }

        let progress = elapsed.as_secs_f64() / self.duration.as_secs_f64();
        let eased_progress = apply_easing(easing, progress);
        let new_tokens = ((self.target_tokens as f64) * eased_progress) as u32;
        
        self.current_tokens = new_tokens.min(self.target_tokens);
        self.current_tokens
    }

    /// Set new target tokens and restart animation
    pub fn set_target(&mut self, target_tokens: u32) {
        self.target_tokens = target_tokens;
        self.start_time = std::time::Instant::now();
    }

    /// Check if animation is complete
    pub fn is_complete(&self) -> bool {
        self.current_tokens >= self.target_tokens
    }
}

/// Spinner animation for status display
pub struct SpinnerAnimation {
    chars: &'static [&'static str],
    start_time: std::time::Instant,
}

impl SpinnerAnimation {
    /// Create new spinner animation
    pub fn new() -> Self {
        Self {
            chars: &["✻", "✦", "✧", "✶"],
            start_time: std::time::Instant::now(),
        }
    }

    /// Get current spinner character
    pub fn current_char(&self) -> &'static str {
        let elapsed_secs = self.start_time.elapsed().as_secs();
        let index = (elapsed_secs % self.chars.len() as u64) as usize;
        self.chars[index]
    }

    /// Reset spinner animation
    pub fn reset(&mut self) {
        self.start_time = std::time::Instant::now();
    }
}

impl Default for SpinnerAnimation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_linear() {
        assert_eq!(apply_easing(Easing::Linear, 0.0), 0.0);
        assert_eq!(apply_easing(Easing::Linear, 0.5), 0.5);
        assert_eq!(apply_easing(Easing::Linear, 1.0), 1.0);
    }

    #[test]
    fn test_easing_ease_out_cubic() {
        let result = apply_easing(Easing::EaseOutCubic, 0.5);
        assert!(result > 0.5); // Should be faster than linear
    }

    #[test]
    fn test_token_animation() {
        let mut anim = TokenAnimation::new(100, std::time::Duration::from_millis(1000));
        assert_eq!(anim.current_tokens, 0);
        
        // Should animate towards target
        let updated = anim.update(Easing::Linear);
        assert!(updated <= 100);
    }

    #[test]
    fn test_spinner_animation() {
        let spinner = SpinnerAnimation::new();
        let char1 = spinner.current_char();
        assert!(["✻", "✦", "✧", "✶"].contains(&char1));
    }

    #[test]
    fn test_animation_config_default() {
        let config = UiAnimationConfig::default();
        assert_eq!(config.frame_interval_ms, 10);
        assert_eq!(config.duration_ms, 3000);
    }
}
