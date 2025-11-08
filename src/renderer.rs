use macroquad::prelude::*;

use std::sync::mpsc::{Receiver};
use webboy::device::{ImageData};
use webboy::tlu::TLUData;
use webboy::palette;

const SCALE_FACTOR: f32 = 2.0;

pub fn window_conf() -> Conf {
    // Configuration for the screen
    Conf {
        window_title: "Web boy".to_owned(),
        window_width: (192) * (SCALE_FACTOR as i32) + 8,
        window_height: (242) * (SCALE_FACTOR as i32) + 8,
        window_resizable: true,
        ..Default::default()
    }
}

pub async fn handle(rx: &Receiver<ImageData>) {
    next_frame().await;

    // Drain all pending messages, keep only the latest
    let mut latest_data = None;
    while let Ok(data) = rx.try_recv() {
        latest_data = Some(data);
    }

    if let Some(data) = latest_data {
        render_tlu_data(&data.tlu_data).await;
    }
}

pub async fn render_tlu_data(tlu_data: &TLUData) {
    let scale_factor = 1.5;

    let width = tlu_data.tile_data[0].len() as f32;
    let height = tlu_data.tile_data.len() as f32;

    let texture = Texture2D::from_rgba8(
        width as u16,
        height as u16,
        &tlu_data
            .tile_data
            .iter()
            .flat_map(|row| row.iter().flat_map(palette::Color::to_rgba).collect::<Vec<u8>>())
        	.collect::<Vec<u8>>(),
    );
    texture.set_filter(FilterMode::Nearest);

    let tile_map_texture = Texture2D::from_rgba8(
        tlu_data.background_data[0].len() as u16,
        tlu_data.background_data.len() as u16,
        &tlu_data
            .background_data
            .iter()
            .flat_map(|row| row.iter().flat_map(palette::Color::to_rgba).collect::<Vec<u8>>())
            .collect::<Vec<u8>>(),
    );
    tile_map_texture.set_filter(FilterMode::Nearest);

    // Same color as an actual game boy
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(189.0 / 255.0, 192.0 / 255.0, 202.0 / 255.0, 1.0),
    );

    draw_texture_ex(
        &texture,
        4.0, 4.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(width * scale_factor, height * scale_factor)),
            ..Default::default()
        },
    );

    draw_texture_ex(
        &tile_map_texture,
        4.0, (64.0 * scale_factor) + 8.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(tlu_data.background_data[0].len() as f32 * scale_factor, tlu_data.background_data.len() as f32 * scale_factor)),
            ..Default::default()
        },
    );
}