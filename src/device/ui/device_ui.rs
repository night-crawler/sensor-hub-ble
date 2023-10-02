use embedded_graphics::text::Text;
use embedded_graphics_core::Drawable;
use embedded_graphics_core::prelude::{DrawTarget, OriginDimensions, Point};
use embedded_layout::layout::linear::FixedMargin;
use embedded_layout::layout::linear::spacing::DistributeFill;
use embedded_layout::prelude::{Align, horizontal, vertical};
use u8g2_fonts::U8g2TextStyle;

use crate::common::device::ui::error::UiError;
use crate::common::device::ui::text_repr::TextRepr;
use crate::common::device::ui::ui_macro::{chain_text_step, h_layout, m_chain, v_layout};

static VERTICAL_MARGIN: FixedMargin = FixedMargin(5);


pub(crate) struct Ui<'a, D> where D: DrawTarget + OriginDimensions {
    display: &'a mut D,
    background_color: D::Color,
    text_style_bat: U8g2TextStyle<<D as DrawTarget>::Color>,
    text_style_small: U8g2TextStyle<<D as DrawTarget>::Color>,
    text_style_med: U8g2TextStyle<<D as DrawTarget>::Color>,
    text_style_large: U8g2TextStyle<<D as DrawTarget>::Color>,
    text_style_large_2: U8g2TextStyle<<D as DrawTarget>::Color>,
    text_style_embedded: U8g2TextStyle<<D as DrawTarget>::Color>,
    text_style_weather: U8g2TextStyle<<D as DrawTarget>::Color>,
    text_style_thing: U8g2TextStyle<<D as DrawTarget>::Color>,
}

impl<'a, D> Ui<'a, D> where D: DrawTarget + OriginDimensions, UiError<<D as DrawTarget>::Error>: From<<D as DrawTarget>::Error> {
    pub(crate) fn new(display: &'a mut D, foreground_color: D::Color, background_color: D::Color) -> Self {
        Self {
            display,
            background_color,
            text_style_bat: U8g2TextStyle::new(u8g2_fonts::fonts::u8g2_font_battery19_tn, foreground_color),
            text_style_small: U8g2TextStyle::new(u8g2_fonts::fonts::u8g2_font_guildenstern_nbp_tr, foreground_color),
            text_style_med: U8g2TextStyle::new(u8g2_fonts::fonts::u8g2_font_logisoso26_tr, foreground_color),
            text_style_large: U8g2TextStyle::new(u8g2_fonts::fonts::u8g2_font_logisoso50_tr, foreground_color),
            text_style_large_2: U8g2TextStyle::new(u8g2_fonts::fonts::u8g2_font_logisoso42_tr, foreground_color),
            text_style_embedded: U8g2TextStyle::new(u8g2_fonts::fonts::u8g2_font_open_iconic_embedded_2x_t, foreground_color),
            text_style_weather: U8g2TextStyle::new(u8g2_fonts::fonts::u8g2_font_open_iconic_weather_2x_t, foreground_color),
            text_style_thing: U8g2TextStyle::new(u8g2_fonts::fonts::u8g2_font_open_iconic_thing_2x_t, foreground_color),
        }
    }

    pub(crate) fn draw(&mut self, text_repr: TextRepr) -> Result<(), UiError<D::Error>> {
        self.display.clear(self.background_color)?;

        let display_area = self.display.bounding_box();
        if display_area.size.width > display_area.size.height {
            self.draw_horizontal(text_repr)
        } else {
            self.draw_vertical(text_repr)
        }
    }

    pub(crate) fn draw_vertical(&mut self, text_repr: TextRepr) -> Result<(), UiError<D::Error>> {
        let display_area = self.display.bounding_box();
        let width = display_area.size.width;
        let height = display_area.size.height;

        let bat = Text::new(&text_repr.bat, Point::zero(), self.text_style_bat.clone());

        let nrf_voltages_chain_1 = chain_text_step!(
            text_repr.nrf_voltages, self.text_style_small, step = 4,
            0, 4, 8
        );
        let nrf_voltages_chain_2 = chain_text_step!(
            text_repr.nrf_voltages, self.text_style_small, step = 4,
             12, 16, 20, 24
        );

        let connections = h_layout!(
            Text::new("\u{0050}", Point::zero(), self.text_style_embedded.clone()),
            Text::new(&text_repr.connections, Point::zero(), self.text_style_small.clone());
            spacing = FixedMargin(1);
            alignment = vertical::Center
        );

        let first_row = h_layout! {
            connections,
            h_layout!(nrf_voltages_chain_1; spacing = DistributeFill(width - 45));
            spacing = DistributeFill(width - 15);
            alignment = vertical::Center
        };

        let second_row = h_layout! {
            nrf_voltages_chain_2;
            spacing = DistributeFill(width - 15)
        };

        let top_rows = v_layout! {
            first_row,
            second_row;
            spacing = VERTICAL_MARGIN
        };

        let nrf_adc_layout = h_layout! {
            top_rows, bat;
            spacing = DistributeFill(width);
            alignment = vertical::Top
        };

        let bme_layout = v_layout! {
            Text::new(&text_repr.temp, Point::zero(), self.text_style_large_2.clone()),
                Text::new(&text_repr.pressure, Point::zero(), self.text_style_med.clone()),
                Text::new(&text_repr.humidity, Point::zero(), self.text_style_med.clone());
            spacing = VERTICAL_MARGIN;
            alignment = horizontal::Center
        };

        let lux = h_layout!(
            Text::new("\u{0045}", Point::zero(), self.text_style_weather.clone()),
            Text::new(&text_repr.lux_text, Point::zero(), self.text_style_small.clone());
            spacing = VERTICAL_MARGIN;
            alignment = vertical::Center
        );

        let cct = h_layout!(
            Text::new("\u{004E}", Point::zero(), self.text_style_thing.clone()),
            Text::new(&text_repr.cct_text, Point::zero(), self.text_style_small.clone());
            spacing = VERTICAL_MARGIN;
            alignment = vertical::Center
        );

        let color_layout = v_layout! {
            Text::new(&text_repr.rgbw_text, Point::zero(), self.text_style_small.clone()),
            h_layout!(lux, cct; spacing = DistributeFill(width));
            spacing = VERTICAL_MARGIN;
            alignment = horizontal::Center
        };

        let adc_layout_1 = chain_text_step!(
            text_repr.adc_voltages, self.text_style_small, step = 4,
            0, 4, 8, 12
        );
        let adc_layout_2 = chain_text_step!(
            text_repr.adc_voltages, self.text_style_small, step = 4,
            16, 20, 24, 28
        );

        let adc_layout = v_layout! {
            h_layout!(adc_layout_1; spacing = DistributeFill(width)),
            h_layout!(adc_layout_2; spacing = DistributeFill(width));
            spacing = VERTICAL_MARGIN
        };

        let main_layout = v_layout! {
            nrf_adc_layout,
            bme_layout,
            color_layout,
            adc_layout
            ;
            spacing = DistributeFill(height);
            alignment = horizontal::Center
        };

        main_layout
            .align_to(&display_area, horizontal::Center, vertical::Center)
            .draw(self.display)?;

        Ok(())
    }

    pub(crate) fn draw_horizontal(&mut self, text_repr: TextRepr) -> Result<(), UiError<D::Error>> {
        let display_area = self.display.bounding_box();
        let width = display_area.size.width;
        let height = display_area.size.height;

        let bat_text = Text::new(&text_repr.bat, Point::zero(), self.text_style_bat.clone());

        let nrf_voltages_chain = chain_text_step!(
            text_repr.nrf_voltages, self.text_style_small, step = 4,
            0, 4, 8, 12, 16, 20, 24
        );

        let adc_voltages_chain = chain_text_step!(
            text_repr.adc_voltages, self.text_style_small, step = 4,
            0, 4, 8, 12, 16, 20, 24, 28
        );

        let bme_layout = h_layout! {
            Text::new(&text_repr.temp, Point::zero(), self.text_style_large.clone()),
            v_layout!(
                Text::new(&text_repr.humidity, Point::zero(), self.text_style_med.clone()),
                Text::new(&text_repr.pressure, Point::zero(), self.text_style_med.clone());
                spacing = VERTICAL_MARGIN
            );
            spacing = DistributeFill(width)
        };

        let connections = h_layout!(
            Text::new("\u{0050}", Point::zero(), self.text_style_embedded.clone()),
            Text::new(&text_repr.connections, Point::zero(), self.text_style_small.clone());
            spacing = FixedMargin(1);
            alignment = vertical::Center
        );

        let lux = h_layout!(
            Text::new("\u{0045}", Point::zero(), self.text_style_weather.clone()),
            Text::new(&text_repr.lux_text, Point::zero(), self.text_style_small.clone());
            spacing = VERTICAL_MARGIN;
            alignment = vertical::Center
        );

        let cct = h_layout!(
            Text::new("\u{004E}", Point::zero(), self.text_style_thing.clone()),
            Text::new(&text_repr.cct_text, Point::zero(), self.text_style_small.clone());
            spacing = VERTICAL_MARGIN;
            alignment = vertical::Center
        );

        let rgbw = Text::new(&text_repr.rgbw_text, Point::zero(), self.text_style_small.clone());
        let xyz = Text::new(&text_repr.xyz_text, Point::zero(), self.text_style_small.clone());

        let first_two_rows = v_layout! {
            h_layout! {
                v_layout! {
                    h_layout!(nrf_voltages_chain; spacing = DistributeFill(width - 15)),
                    h_layout!(connections, xyz, lux, cct; spacing = DistributeFill(width - 15); alignment = vertical::Center);
                    spacing = VERTICAL_MARGIN;
                    alignment = horizontal::Center
                },
                bat_text;
                spacing = DistributeFill(width);
                alignment = vertical::Top
            },
            rgbw;
            spacing = VERTICAL_MARGIN
        };

        let main_layout = v_layout! {
            first_two_rows,
            bme_layout,
            h_layout!(adc_voltages_chain; spacing = DistributeFill(width));

            spacing = DistributeFill(height);
            alignment = horizontal::Center
        };

        main_layout
            .align_to(&display_area, horizontal::Center, vertical::Center)
            .draw(self.display)?;

        Ok(())
    }
}
