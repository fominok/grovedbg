//! Module of useful components

use std::fmt::Write;

use eframe::egui;

const MAX_BYTES: usize = 10;
const MAX_HEX_LENGTH: usize = 20;
const HEX_PARTS_LENGTH: usize = 8;

fn bytes_as_slice(bytes: &[u8]) -> String {
    if bytes.len() <= MAX_BYTES {
        format!("{:?}", bytes)
    } else {
        let mut buf = String::from("[");
        bytes.iter().for_each(|b| {
            let _ = write!(buf, "{b},");
        });
        buf.push_str("...");
        buf
    }
}

pub(crate) fn bytes_as_hex(bytes: &[u8]) -> String {
    let hex_str = hex::encode(bytes);
    if hex_str.len() <= MAX_HEX_LENGTH {
        hex_str
    } else {
        let mut buf = String::from(&hex_str[0..HEX_PARTS_LENGTH]);
        buf.push_str("..");
        buf.push_str(&hex_str[(hex_str.len() - HEX_PARTS_LENGTH)..]);
        buf
    }
}

pub(crate) fn bytes_by_display_variant(bytes: &[u8], display_variant: &DisplayVariant) -> String {
    match display_variant {
        DisplayVariant::U8 => bytes_as_slice(bytes),
        DisplayVariant::String => String::from_utf8_lossy(bytes).to_string(),
        DisplayVariant::Hex => bytes_as_hex(bytes),
    }
}

/// Represent binary data different ways and to choose from
pub(crate) fn binary_label<'a>(
    ui: &mut egui::Ui,
    bytes: &[u8],
    display_variant: &mut DisplayVariant,
) {
    let text = bytes_by_display_variant(bytes, &display_variant);
    ui.collapsing(text, |ui| {
        ui.radio_value(display_variant, DisplayVariant::U8, "Integers");
        ui.radio_value(display_variant, DisplayVariant::String, "UTF-8 String");
        ui.radio_value(display_variant, DisplayVariant::Hex, "Hex String");
    });
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum DisplayVariant {
    U8,
    String,
    Hex,
}
