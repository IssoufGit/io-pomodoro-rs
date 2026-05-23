// Procedurally drawn 64×64 tomato-clock icon.
// No external asset file needed; runs at startup.

use eframe::egui;

pub fn make_icon() -> egui::IconData {
    const SIZE: usize = 64;
    let mut rgba = vec![0u8; SIZE * SIZE * 4];
    let cx = (SIZE as f32 - 1.0) / 2.0;
    let cy = (SIZE as f32 - 1.0) / 2.0;
    let r_outer = 28.0_f32;
    let r_edge = 27.0_f32; // antialiasing band

    let red = [196u8, 69, 54, 255];
    let rim = [150u8, 45, 35, 255];
    let white = [248u8, 240, 232, 255];
    let ink = [28u8, 27, 24, 255];

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            let i = (y * SIZE + x) * 4;

            if d <= r_outer {
                let mut c = red;
                if d <= 14.0 { c = white; }
                if d > r_edge && d <= r_outer {
                    let t = ((d - r_edge) / (r_outer - r_edge)).clamp(0.0, 1.0);
                    c = blend(red, rim, t);
                }
                rgba[i..i + 4].copy_from_slice(&c);
            } else if d <= r_outer + 1.0 {
                // 1px antialias ring against transparent background
                let alpha = (1.0 - (d - r_outer)).clamp(0.0, 1.0);
                let mut c = rim;
                c[3] = (alpha * 255.0) as u8;
                rgba[i..i + 4].copy_from_slice(&c);
            }
        }
    }

    // Clock hands: hour → 12 o'clock, minute → 3 o'clock
    draw_line(&mut rgba, SIZE, (cx, cy), (cx, cy - 9.0), ink);
    draw_line(&mut rgba, SIZE, (cx, cy), (cx + 11.0, cy), ink);

    // Center dot
    for dy in -1..=1i32 {
        for dx in -1..=1i32 {
            let xi = cx as i32 + dx;
            let yi = cy as i32 + dy;
            if xi >= 0 && yi >= 0 && (xi as usize) < SIZE && (yi as usize) < SIZE {
                let i = ((yi as usize) * SIZE + xi as usize) * 4;
                rgba[i..i + 4].copy_from_slice(&ink);
            }
        }
    }

    // Stem on top (small green wedge)
    let green = [74u8, 124, 89, 255];
    for y in 2..7usize {
        let half = (7 - y) as i32;
        for x in (cx as i32 - half)..=(cx as i32 + half) {
            if x >= 0 && (x as usize) < SIZE {
                let i = (y * SIZE + x as usize) * 4;
                rgba[i..i + 4].copy_from_slice(&green);
            }
        }
    }

    egui::IconData { rgba, width: SIZE as u32, height: SIZE as u32 }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn blend(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);
    [
        (a[0] as f32 * (1.0 - t) + b[0] as f32 * t) as u8,
        (a[1] as f32 * (1.0 - t) + b[1] as f32 * t) as u8,
        (a[2] as f32 * (1.0 - t) + b[2] as f32 * t) as u8,
        (a[3] as f32 * (1.0 - t) + b[3] as f32 * t) as u8,
    ]
}

fn draw_line(rgba: &mut [u8], size: usize, from: (f32, f32), to: (f32, f32), color: [u8; 4]) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let steps = dx.abs().max(dy.abs()).ceil() as i32;
    for s in 0..=steps {
        let t = s as f32 / steps.max(1) as f32;
        let x = (from.0 + dx * t).round() as i32;
        let y = (from.1 + dy * t).round() as i32;
        for oy in -1..=0i32 {
            for ox in -1..=0i32 {
                let xi = x + ox;
                let yi = y + oy;
                if xi >= 0 && yi >= 0 && (xi as usize) < size && (yi as usize) < size {
                    let i = ((yi as usize) * size + xi as usize) * 4;
                    rgba[i..i + 4].copy_from_slice(&color);
                }
            }
        }
    }
}
