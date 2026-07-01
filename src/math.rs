/// Pure mathematical calculations for Keysor mouse emulation

/// Calculates the cursor speed based on elapsed time, acceleration rate, and limits.
pub fn calculate_speed(base: f64, elapsed: f64, accel: f64, max: f64) -> f64 {
    (base + elapsed * accel * 18.0).min(max)
}

/// Calculates the movement delta for X and Y coordinates, taking into account
/// the current speed, direction, DPI scale, pixel mode, and sub-pixel remainders.
pub fn calculate_movement_delta(
    speed: f64,
    dx_dir: f64,
    dy_dir: f64,
    pixel_mode: bool,
    dpi_scale: f64,
    remainder_x: &mut f64,
    remainder_y: &mut f64,
) -> (i32, i32) {
    if pixel_mode {
        let target_dx = speed * dx_dir + *remainder_x;
        let target_dy = speed * dy_dir + *remainder_y;
        let mx = target_dx.round();
        let my = target_dy.round();
        *remainder_x = target_dx - mx;
        *remainder_y = target_dy - my;
        (mx as i32, my as i32)
    } else {
        // Diagonal speed correction (1 / sqrt(2))
        let factor = if dx_dir != 0.0 && dy_dir != 0.0 { std::f64::consts::FRAC_1_SQRT_2 } else { 1.0 };
        let target_dx = speed * dx_dir * factor * dpi_scale + *remainder_x;
        let target_dy = speed * dy_dir * factor * dpi_scale + *remainder_y;
        let mx = target_dx.round();
        let my = target_dy.round();
        *remainder_x = target_dx - mx;
        *remainder_y = target_dy - my;
        (mx as i32, my as i32)
    }
}
