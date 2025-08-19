//! RGB LED driver

use embassy_stm32::{
    gpio::{Level, OutputOpenDrain, Pin, Speed},
    Peri,
};

pub struct Led<'d> {
    red: OutputOpenDrain<'d>,
    green: OutputOpenDrain<'d>,
    blue: OutputOpenDrain<'d>,
}

impl<'d> Led<'d> {
    pub fn new(
        red: Peri<'d, impl Pin>,
        green: Peri<'d, impl Pin>,
        blue: Peri<'d, impl Pin>,
    ) -> Self {
        let red = OutputOpenDrain::new(red, Level::High, Speed::Low);
        let green = OutputOpenDrain::new(green, Level::High, Speed::Low);
        let blue = OutputOpenDrain::new(blue, Level::High, Speed::Low);
        Self { red, green, blue }
    }

    pub fn set_color(&mut self, color: Color) {
        let (r, g, b) = color.into_rgb();
        self.red.set_level(r);
        self.green.set_level(g);
        self.blue.set_level(b);
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum Color {
    Off,
    Red,
    Green,
    Blue,
    Cyan,
    Magenta,
    Yellow,
    White,
}

impl Color {
    fn into_rgb(self) -> (Level, Level, Level) {
        use Color::*;
        use Level::*;
        match self {
            Off => (High, High, High),
            Red => (Low, High, High),
            Green => (High, Low, High),
            Blue => (High, High, Low),
            Cyan => (High, Low, Low),
            Magenta => (Low, High, Low),
            Yellow => (Low, Low, High),
            White => (Low, Low, Low),
        }
    }
}
