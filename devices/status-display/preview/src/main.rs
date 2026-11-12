//! Host-side renderer for the status-display dashboard.
//!
//! Compiles the firmware's UI/state modules unchanged and renders them into an
//! RGB565 framebuffer, so layout can be validated pixel-for-pixel without
//! flashing the device:
//!
//!   cargo run --release -- out/        # writes <scenario>.png (1x) and <scenario>@3x.png
//!   cargo test

// The firmware modules expose items only the device build uses.
#![allow(dead_code)]

#[path = "../../firmware/src/aa_fonts.rs"]
mod aa_fonts;
#[path = "../../firmware/src/aa_text.rs"]
mod aa_text;
#[path = "../../firmware/src/canvas.rs"]
mod canvas;
#[path = "../../firmware/src/state.rs"]
mod state;
#[path = "../../firmware/src/ui.rs"]
mod ui;

use std::path::Path;

use anyhow::Result;
use embedded_graphics::prelude::*;

use canvas::Canvas;
use state::{new_shared, SharedState};
use ui::{Dashboard, SCREEN_H, SCREEN_W};

const IP: &str = "192.168.0.88";

fn set(st: &SharedState, entity: &str, attr: &str, value: &str) {
    st.lock().unwrap().update(entity, attr, value);
}

fn thermostat(st: &SharedState, entity: &str, mode: &str, action: &str, cur: &str, lo: &str, hi: &str) {
    set(st, entity, "", mode);
    set(st, entity, "hvac_action", action);
    set(st, entity, "current_temperature", cur);
    set(st, entity, "target_temp_low", lo);
    set(st, entity, "target_temp_high", hi);
}

fn schedule(st: &SharedState, heat: &str, cool: &str) {
    for band in ["early_morning", "morning", "daytime", "overnight"] {
        set(st, &format!("input_number.hvac_{}_heat_f", band), "", heat);
        set(st, &format!("input_number.hvac_{}_cool_f", band), "", cool);
    }
}

const KITCHEN: &str = "climate.minisplit_kitchen_smart_thermostat";
const LIVING: &str = "climate.minisplit_livingroom_smart_thermostat";
const AMOS: &str = "climate.minisplit_amos_smart_thermostat";
const HALL: &str = "climate.smart_thermostat";
const KITCHEN_FAN: &str = "binary_sensor.kitchen_minisplit_kitchen_fan";
const LIVING_FAN: &str = "binary_sensor.living_room_minisplit_livingroom_fan";
const AMOS_FAN: &str = "binary_sensor.amos_bedroom_minisplit_amos_fan";

/// The sample values used in the Stitch design.
fn scenario_design() -> SharedState {
    let st = new_shared();
    thermostat(&st, KITCHEN, "heat_cool", "heating", "70", "66", "73");
    set(&st, KITCHEN_FAN, "", "on");
    thermostat(&st, LIVING, "heat_cool", "cooling", "71.4", "66", "73");
    set(&st, LIVING_FAN, "", "on");
    thermostat(&st, AMOS, "heat_cool", "idle", "68", "64", "72");
    set(&st, AMOS_FAN, "", "off");
    thermostat(&st, HALL, "heat_cool", "fan", "67", "67", "74");
    schedule(&st, "66", "74");
    set(&st, "sensor.atc_a0c6_temperature", "", "71.2");
    set(&st, "sensor.kitchen_temperature", "", "72");
    set(&st, "input_text.bb_status", "", "working");
    st.lock().unwrap().set_ha_clients(1);
    st
}

/// Unusual states: heat warm-up (fan off), defrost, off, no action reported.
fn scenario_edge() -> SharedState {
    let st = new_shared();
    thermostat(&st, KITCHEN, "heat_cool", "heating", "100", "100", "100");
    set(&st, KITCHEN_FAN, "", "off");
    thermostat(&st, LIVING, "heat_cool", "defrosting", "58", "66", "73");
    set(&st, LIVING_FAN, "", "off");
    thermostat(&st, AMOS, "off", "off", "65", "", "");
    set(&st, AMOS_FAN, "", "on");
    thermostat(&st, HALL, "heat_cool", "None", "67", "67", "74");
    schedule(&st, "66", "74");
    set(&st, "sensor.atc_a0c6_temperature", "", "unavailable");
    set(&st, "input_text.bb_status", "", "playing");
    st.lock().unwrap().set_ha_clients(1);
    st
}

/// Freshly booted, Home Assistant has not connected yet.
fn scenario_offline() -> SharedState {
    new_shared()
}

fn render(st: &SharedState) -> Result<Canvas> {
    let mut screen = Canvas::new(Size::new(SCREEN_W, SCREEN_H));
    let mut dash = Dashboard::new(IP.to_string());
    dash.update(&mut screen, st, 12, 0)?;
    Ok(screen)
}

fn write_png(c: &Canvas, path: &Path, scale: u32) -> Result<()> {
    let size = c.size();
    let (w, h) = (size.width * scale, size.height * scale);
    let mut data = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            let p = c.pixel(Point::new((x / scale) as i32, (y / scale) as i32)).unwrap();
            data.push((p.r() << 3) | (p.r() >> 2));
            data.push((p.g() << 2) | (p.g() >> 4));
            data.push((p.b() << 3) | (p.b() >> 2));
        }
    }
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&data)?;
    Ok(())
}

fn main() -> Result<()> {
    let out = std::env::args().nth(1).unwrap_or_else(|| "out".into());
    let out = Path::new(&out);
    std::fs::create_dir_all(out)?;
    let scenarios: [(&str, fn() -> SharedState); 3] = [
        ("design", scenario_design),
        ("edge", scenario_edge),
        ("offline", scenario_offline),
    ];
    for (name, make) in scenarios {
        let screen = render(&make())?;
        write_png(&screen, &out.join(format!("{name}.png")), 1)?;
        write_png(&screen, &out.join(format!("{name}@3x.png")), 3)?;
        println!("wrote {}", out.join(format!("{name}.png")).display());
    }

    let mut boot = Canvas::new(Size::new(SCREEN_W, SCREEN_H));
    let mut dash = Dashboard::new(String::new());
    dash.draw_boot_status(&mut boot, "Connecting to WiFi...", None)?;
    dash.draw_boot_status(&mut boot, "Waiting for HA...", Some(IP))?;
    write_png(&boot, &out.join("boot@3x.png"), 3)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::pixelcolor::Rgb565;
    use embedded_graphics::primitives::Rectangle;
    use ui::{strip_rect, tile_rect, BG};

    #[test]
    fn incremental_updates_match_full_redraw() {
        let st = scenario_design();
        let mut screen = Canvas::new(Size::new(SCREEN_W, SCREEN_H));
        let mut dash = Dashboard::new(IP.to_string());
        dash.update(&mut screen, &st, 12, 0).unwrap();

        thermostat(&st, LIVING, "heat_cool", "idle", "69", "66", "73");
        set(&st, LIVING_FAN, "", "off");
        set(&st, "input_text.bb_status", "", "away");
        st.lock().unwrap().set_ha_clients(0);
        dash.update(&mut screen, &st, 12, 0).unwrap();

        let fresh = render(&st).unwrap();
        assert!(screen.pixels() == fresh.pixels());
    }

    #[test]
    fn unchanged_state_draws_nothing() {
        struct Counting(usize);
        impl OriginDimensions for Counting {
            fn size(&self) -> Size {
                Size::new(SCREEN_W, SCREEN_H)
            }
        }
        impl DrawTarget for Counting {
            type Color = Rgb565;
            type Error = core::convert::Infallible;
            fn draw_iter<I: IntoIterator<Item = Pixel<Rgb565>>>(&mut self, it: I) -> Result<(), Self::Error> {
                self.0 += it.into_iter().count();
                Ok(())
            }
        }
        let st = scenario_design();
        let mut d = Counting(0);
        let mut dash = Dashboard::new(IP.to_string());
        dash.update(&mut d, &st, 12, 0).unwrap();
        assert!(d.0 > 0);
        d.0 = 0;
        dash.update(&mut d, &st, 12, 0).unwrap();
        assert_eq!(d.0, 0);
    }

    #[test]
    fn regions_tile_the_screen_without_overlap() {
        let rects: Vec<_> = (0..4).map(tile_rect).chain([strip_rect()]).collect();
        let screen = Rectangle::new(Point::zero(), Size::new(SCREEN_W, SCREEN_H));
        for (i, a) in rects.iter().enumerate() {
            assert_eq!(a.intersection(&screen), *a, "region {i} off screen");
            for b in &rects[i + 1..] {
                assert!(a.intersection(b).is_zero_sized(), "{a:?} overlaps {b:?}");
            }
        }
    }

    #[test]
    fn content_stays_inside_tiles() {
        // Text is drawn into a tile-sized canvas, so overflow would be clipped
        // silently. The widest realistic content must leave the right padding
        // and the gutters untouched.
        let screen = render(&scenario_edge()).unwrap();
        for i in 0..4 {
            let r = tile_rect(i);
            let x = r.top_left.x + r.size.width as i32 - 4;
            for y in r.top_left.y + 4..r.top_left.y + r.size.height as i32 - 4 {
                let p = screen.pixel(Point::new(x, y)).unwrap();
                assert_eq!(p, screen.pixel(Point::new(x, r.top_left.y + 2)).unwrap(),
                    "tile {i} content reaches right padding at y={y}");
            }
        }
        for y in 0..strip_rect().top_left.y {
            assert_eq!(screen.pixel(Point::new(159, y)).unwrap(), BG, "gutter at y={y}");
        }
    }

    #[test]
    fn action_and_fan_parsing() {
        let st = scenario_edge();
        {
            let s = st.lock().unwrap();
            assert_eq!(s.thermostats[0].action(), state::Action::Heating);
            assert!(!s.thermostats[0].fan_on());
            assert_eq!(s.thermostats[1].action(), state::Action::Defrosting);
            assert_eq!(s.thermostats[2].action(), state::Action::Off);
            assert!(s.thermostats[2].fan_on());
            assert_eq!(s.thermostats[3].action(), state::Action::Auto);
        }
        let st = scenario_design();
        let s = st.lock().unwrap();
        assert!(s.thermostats[3].fan_on(), "fan-only action implies fan running");
    }
}
