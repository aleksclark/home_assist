use std::fmt::Write as FmtWrite;

use anyhow::Result;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use ha_display_kit::{
    capitalize, clear_screen, fill_rect, style_small, style_small_bg, txt, txt_center, BootScreen,
    Region, Theme,
};

use crate::state::{BbStatus, SharedState, NUM_SENSORS, NUM_THERMOSTATS};

const SCREEN_W: u32 = 320;
const SCREEN_H: u32 = 240;
const SCREEN_CX: i32 = 160;

const COL_NAME: i32 = 6;
const COL_STATUS: i32 = 80;
const COL_CURR: i32 = 160;
const COL_SET: i32 = 230;

const COL_NAME_CHARS: usize = 12;
const COL_STATUS_CHARS: usize = 13;
const COL_CURR_CHARS: usize = 11;
const COL_SET_CHARS: usize = 15;

const HEADER_Y: i32 = 14;
const TABLE_Y: i32 = 22;
const ROW_H: i32 = 20;
const HEADER_H: u32 = 20;

const FOOTER: Region = Region::new(0, SCREEN_H as i32 - 22, SCREEN_W, 22);

fn row_y(idx: usize) -> i32 {
    TABLE_Y + ROW_H * (idx as i32 + 1) + 12
}

fn row_region(idx: usize) -> Region {
    let y = TABLE_Y + ROW_H * (idx as i32 + 1);
    Region::new(0, y, SCREEN_W, ROW_H as u32)
}

fn band_y() -> i32 {
    TABLE_Y + ROW_H * (NUM_THERMOSTATS as i32 + 1) + 8
}

fn band_region() -> Region {
    Region::new(0, band_y() - 6, SCREEN_W, ROW_H as u32)
}

fn sensor_y() -> i32 {
    band_y() + ROW_H + 4
}

fn sensor_region() -> Region {
    Region::new(0, sensor_y() - 6, SCREEN_W, ROW_H as u32)
}

fn bb_status_y() -> i32 {
    sensor_y() + ROW_H + 4
}

fn bb_status_region() -> Region {
    Region::new(0, bb_status_y() - 6, SCREEN_W, ROW_H as u32)
}

fn bb_status_color(status: &BbStatus) -> Rgb565 {
    match status {
        BbStatus::Unknown => Rgb565::new(8, 16, 8),     // dim green (same as off)
        BbStatus::Away => Rgb565::new(31, 48, 0),       // yellow
        BbStatus::Working => Rgb565::new(0, 63, 0),     // green
        BbStatus::Playing => Rgb565::new(20, 12, 31),   // purple
    }
}

fn format_temp(raw: &str) -> String {
    if raw.is_empty() {
        return "--".into();
    }
    match raw.parse::<f32>() {
        Ok(v) => {
            let mut s = String::new();
            write!(s, "{:.0}F", v).unwrap();
            s
        }
        Err(_) => raw.to_string(),
    }
}

fn pad_right(s: &str, width: usize) -> String {
    let mut out = String::with_capacity(width);
    out.push_str(s);
    while out.len() < width {
        out.push(' ');
    }
    out
}

fn local_hm() -> (u8, u8) {
    unsafe {
        let mut now: esp_idf_sys::time_t = 0;
        esp_idf_sys::time(&mut now);
        let mut tm: esp_idf_sys::tm = core::mem::zeroed();
        esp_idf_sys::localtime_r(&now, &mut tm);
        (tm.tm_hour as u8, tm.tm_min as u8)
    }
}

fn status_color(status: &str, theme: &Theme) -> Rgb565 {
    match status {
        "heating" | "heat" => theme.heat,
        "cooling" | "cool" => theme.cool,
        "drying" | "dry" => theme.dry,
        "fan" | "fan_only" => theme.fan_only,
        "idle" | "auto" | "heat_cool" => theme.label,
        "off" | "unavailable" => theme.off,
        _ => theme.unavail,
    }
}

fn status_label(status: &str) -> String {
    match status {
        "heating" => "HEAT".into(),
        "cooling" => "COOL".into(),
        "drying" => "DRY".into(),
        "fan" | "fan_only" => "FAN".into(),
        "idle" => "IDLE".into(),
        "off" => "OFF".into(),
        "heat" => "HEAT".into(),
        "cool" => "COOL".into(),
        "dry" => "DRY".into(),
        "auto" | "heat_cool" => "AUTO".into(),
        "" => "---".into(),
        other => {
            let mut s = capitalize(other);
            s.make_ascii_uppercase();
            s
        }
    }
}

#[derive(Clone, PartialEq)]
struct RowSnapshot {
    status: String,
    current: String,
    setpoint: String,
    heating: bool,
    cooling: bool,
}

impl Default for RowSnapshot {
    fn default() -> Self {
        Self {
            status: String::new(),
            current: String::new(),
            setpoint: String::new(),
            heating: false,
            cooling: false,
        }
    }
}

#[derive(Clone, PartialEq, Default)]
struct SensorSnapshot {
    value: String,
}

pub struct Dashboard {
    ip: String,
    theme: Theme,
    boot: BootScreen,
    prev_rows: [RowSnapshot; NUM_THERMOSTATS],
    prev_sensors: [SensorSnapshot; NUM_SENSORS],
    prev_band: String,
    prev_bb_status: BbStatus,
    prev_generation: u32,
    first_draw: bool,
}

impl Dashboard {
    pub fn new(ip: String) -> Self {
        Self {
            ip,
            theme: Theme::GREEN,
            boot: BootScreen::new("Status Display", SCREEN_CX),
            prev_rows: Default::default(),
            prev_sensors: Default::default(),
            prev_band: String::new(),
            prev_bb_status: BbStatus::Unknown,
            prev_generation: 0,
            first_draw: true,
        }
    }

    pub fn draw_boot_status<D: DrawTarget<Color = Rgb565>>(
        &self, d: &mut D, status: &str, ip: Option<&str>,
    ) -> Result<()> {
        self.boot.draw_status(d, &self.theme, status, ip)
    }

    pub fn update<D: DrawTarget<Color = Rgb565>>(
        &mut self, d: &mut D, state: &SharedState,
    ) -> Result<()> {
        let st = state.lock().unwrap();
        let gen = st.generation();
        if gen == self.prev_generation && !self.first_draw {
            return Ok(());
        }

        let force = self.first_draw;
        if self.first_draw {
            clear_screen(d, &self.theme)?;
            self.draw_column_headers(d)?;
            self.draw_footer(d)?;
            self.first_draw = false;
        }

        for i in 0..NUM_THERMOSTATS {
            let t = &st.thermostats[i];
            let snap = RowSnapshot {
                status: t.display_status().to_string(),
                current: t.current_temp.clone(),
                setpoint: t.setpoint().to_string(),
                heating: t.is_heating_mode(),
                cooling: t.is_cooling_mode(),
            };
            if force || snap != self.prev_rows[i] || gen != self.prev_generation {
                self.draw_row(d, i, t.name, &snap, force)?;
                self.prev_rows[i] = snap;
            }
        }

        let (h, m) = local_hm();
        let (heat, cool) = st.schedule.current(h, m);
        let band_text = format!("Band:  {} min  /  {} max", format_temp(heat), format_temp(cool));
        if force || band_text != self.prev_band {
            self.draw_band(d, &band_text, force)?;
            self.prev_band = band_text;
        }

        let mut sensor_changed = false;
        for i in 0..NUM_SENSORS {
            let snap = SensorSnapshot { value: st.sensors[i].value.clone() };
            if snap != self.prev_sensors[i] {
                self.prev_sensors[i] = snap;
                sensor_changed = true;
            }
        }
        if force || sensor_changed || gen != self.prev_generation {
            self.draw_sensors(d, &st.sensors, force)?;
        }

        if force || st.bb_status != self.prev_bb_status {
            self.draw_bb_status(d, &st.bb_status, force)?;
            self.prev_bb_status = st.bb_status.clone();
        }

        self.prev_generation = gen;
        Ok(())
    }

    fn draw_column_headers<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) -> Result<()> {
        let t = &self.theme;
        let header = Region::new(0, 0, SCREEN_W, HEADER_H);
        fill_rect(d, &header, t.accent_bg)?;
        let s = style_small(t.header);
        txt(d, "Name", Point::new(COL_NAME, HEADER_Y), s)?;
        txt(d, "Status", Point::new(COL_STATUS, HEADER_Y), s)?;
        txt(d, "Temp", Point::new(COL_CURR, HEADER_Y), s)?;
        txt(d, "Set", Point::new(COL_SET, HEADER_Y), s)?;
        Ok(())
    }

    fn draw_footer<D: DrawTarget<Color = Rgb565>>(&self, d: &mut D) -> Result<()> {
        let t = &self.theme;
        fill_rect(d, &FOOTER, t.accent_bg)?;
        let text = format!("WiFi OK  |  {}", self.ip);
        txt_center(d, &text, Point::new(SCREEN_CX, SCREEN_H as i32 - 7), style_small(t.footer))
    }

    fn draw_row<D: DrawTarget<Color = Rgb565>>(
        &self, d: &mut D, idx: usize, name: &str, snap: &RowSnapshot, force: bool,
    ) -> Result<()> {
        let t = &self.theme;
        let bg = t.bg;

        if force {
            fill_rect(d, &row_region(idx), bg)?;
        }

        let y = row_y(idx);

        let name_padded = pad_right(name, COL_NAME_CHARS);
        txt(d, &name_padded, Point::new(COL_NAME, y), style_small_bg(t.label, bg))?;

        let st_text = pad_right(&status_label(&snap.status), COL_STATUS_CHARS);
        let st_color = status_color(&snap.status, t);
        txt(d, &st_text, Point::new(COL_STATUS, y), style_small_bg(st_color, bg))?;

        let curr = pad_right(&format_temp(&snap.current), COL_CURR_CHARS);
        txt(d, &curr, Point::new(COL_CURR, y), style_small_bg(t.value, bg))?;

        let set_text = if !snap.setpoint.is_empty() {
            pad_right(&format_temp(&snap.setpoint), COL_SET_CHARS)
        } else {
            pad_right("", COL_SET_CHARS)
        };
        let set_color = if snap.cooling {
            t.cool
        } else if snap.heating {
            t.heat
        } else {
            t.label
        };
        txt(d, &set_text, Point::new(COL_SET, y), style_small_bg(set_color, bg))?;

        Ok(())
    }

    fn draw_band<D: DrawTarget<Color = Rgb565>>(
        &self, d: &mut D, text: &str, force: bool,
    ) -> Result<()> {
        let t = &self.theme;
        if force {
            fill_rect(d, &band_region(), t.bg)?;
        }
        let padded = pad_right(text, 50);
        txt_center(d, &padded, Point::new(SCREEN_CX, band_y() + 6), style_small_bg(t.label, t.bg))
    }

    fn draw_sensors<D: DrawTarget<Color = Rgb565>>(
        &self, d: &mut D, sensors: &[crate::state::ExtraSensor; NUM_SENSORS], force: bool,
    ) -> Result<()> {
        let t = &self.theme;
        if force {
            fill_rect(d, &sensor_region(), t.bg)?;
        }

        let y = sensor_y() + 6;
        let half = SCREEN_W as i32 / 2;

        for (i, sensor) in sensors.iter().enumerate() {
            let x = COL_NAME + half * i as i32;
            let temp = format_temp(&sensor.value);
            let label = pad_right(&format!("{}: {}", sensor.name, temp), 25);
            txt(d, &label, Point::new(x, y), style_small_bg(t.value, t.bg))?;
        }

        Ok(())
    }

    fn draw_bb_status<D: DrawTarget<Color = Rgb565>>(
        &self, d: &mut D, status: &BbStatus, force: bool,
    ) -> Result<()> {
        let t = &self.theme;
        let bg = t.bg;
        if force {
            fill_rect(d, &bb_status_region(), bg)?;
        }
        let y = bb_status_y() + 6;
        let color = bb_status_color(status);
        let label = pad_right(&format!("BB Status: {}", status.label()), 40);
        txt(d, &label, Point::new(COL_NAME, y), style_small_bg(color, bg))
    }
}
