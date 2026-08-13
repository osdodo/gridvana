use gridvana_core::color::{hsv_to_rgba, rgb_to_hsv};
use gridvana_core::model::Rgba;
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Event, Frame, Geometry, Path, Stroke};
use iced::widget::image;
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme};
use std::cell::RefCell;

use crate::Message;

const PICKER_HEIGHT: f32 = 150.0;
const HUE_BAR_HEIGHT: f32 = 14.0;
const HUE_BAR_GAP: f32 = 6.0;
const MARKER_RADIUS: f32 = 5.0;
const SUPERSAMPLE: u32 = 2;

pub fn view<'a>(color: Rgba) -> Element<'a, Message> {
    Canvas::new(ColorPicker { color })
        .width(Length::Fill)
        .height(Length::Fixed(PICKER_HEIGHT))
        .into()
}

struct ColorPicker {
    color: Rgba,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Region {
    Square,
    HueBar,
}

#[derive(Clone)]
struct CachedImage {
    width: u32,
    height: u32,
    hue_key: u32,
    handle: image::Handle,
}

#[derive(Default)]
pub struct State {
    dragging: Option<Region>,
    hue: RefCell<f32>,
    image: RefCell<Option<CachedImage>>,
}

impl canvas::Program<Message> for ColorPicker {
    type State = State;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let square = square_rect(bounds.size());
        let hue_bar = hue_bar_rect(bounds.size());
        let (hue, saturation, value) = self.hsv(state);

        let handle = image_handle(&state.image, bounds.size(), hue);
        frame.draw_image(Rectangle::with_size(bounds.size()), &handle);

        frame.stroke(
            &Path::rectangle(square.position(), square.size()),
            Stroke::default().with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.35)),
        );
        frame.stroke(
            &Path::rectangle(hue_bar.position(), hue_bar.size()),
            Stroke::default().with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.35)),
        );

        let marker = Path::circle(
            Point::new(
                square.x + saturation * square.width,
                square.y + (1.0 - value) * square.height,
            ),
            MARKER_RADIUS,
        );
        frame.stroke(
            &marker,
            Stroke::default()
                .with_color(Color::WHITE)
                .with_width(if state.dragging == Some(Region::Square) {
                    2.5
                } else {
                    2.0
                }),
        );
        frame.stroke(
            &marker,
            Stroke::default()
                .with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.55))
                .with_width(1.0),
        );

        let hue_x = hue_bar.x + (hue / 360.0) * hue_bar.width;
        let hue_marker = Path::rectangle(
            Point::new(hue_x - 2.0, hue_bar.y - 1.0),
            Size::new(4.0, hue_bar.height + 2.0),
        );
        frame.stroke(
            &hue_marker,
            Stroke::default().with_color(Color::WHITE).with_width(2.0),
        );
        frame.stroke(
            &hue_marker,
            Stroke::default()
                .with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.55))
                .with_width(1.0),
        );

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let position = cursor.position_in(bounds)?;
                let region = region_at(position, bounds.size())?;
                state.dragging = Some(region);

                Some(canvas::Action::publish(self.color_message(
                    state, region, position, bounds,
                )))
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let region = state.dragging?;
                let position = cursor.position_in(bounds).or_else(|| {
                    cursor
                        .position()
                        .map(|point| Point::new(point.x - bounds.x, point.y - bounds.y))
                })?;

                Some(canvas::Action::publish(self.color_message(
                    state, region, position, bounds,
                )))
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.dragging = None;
                None
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let hovering = cursor
            .position_in(bounds)
            .and_then(|position| region_at(position, bounds.size()))
            .is_some();

        if state.dragging.is_some() || hovering {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

impl ColorPicker {
    /// Hue is undefined for greys and blacks, so the last picked hue is kept in
    /// the canvas state to stop the picker from snapping back to red.
    fn hsv(&self, state: &State) -> (f32, f32, f32) {
        let (hue, saturation, value) = rgb_to_hsv(self.color);

        if saturation <= f32::EPSILON || value <= f32::EPSILON {
            (*state.hue.borrow(), saturation, value)
        } else {
            *state.hue.borrow_mut() = hue;
            (hue, saturation, value)
        }
    }

    fn color_message(
        &self,
        state: &State,
        region: Region,
        position: Point,
        bounds: Rectangle,
    ) -> Message {
        let (hue, saturation, value) = self.hsv(state);

        let color = match region {
            Region::Square => {
                let square = square_rect(bounds.size());
                let saturation = ((position.x - square.x) / square.width).clamp(0.0, 1.0);
                let value = 1.0 - ((position.y - square.y) / square.height).clamp(0.0, 1.0);

                hsv_to_rgba(hue, saturation, value, self.color.a)
            }
            Region::HueBar => {
                let hue_bar = hue_bar_rect(bounds.size());
                let hue = ((position.x - hue_bar.x) / hue_bar.width).clamp(0.0, 1.0) * 360.0;
                *state.hue.borrow_mut() = hue;

                hsv_to_rgba(hue, saturation, value.max(f32::EPSILON), self.color.a)
            }
        };

        Message::SelectColor(color)
    }
}

fn square_rect(size: Size) -> Rectangle {
    Rectangle::new(
        Point::ORIGIN,
        Size::new(
            size.width.max(1.0),
            (size.height - HUE_BAR_HEIGHT - HUE_BAR_GAP).max(1.0),
        ),
    )
}

fn hue_bar_rect(size: Size) -> Rectangle {
    Rectangle::new(
        Point::new(0.0, (size.height - HUE_BAR_HEIGHT).max(0.0)),
        Size::new(size.width.max(1.0), HUE_BAR_HEIGHT),
    )
}

fn region_at(position: Point, size: Size) -> Option<Region> {
    if square_rect(size).contains(position) {
        Some(Region::Square)
    } else if hue_bar_rect(size).contains(position) {
        Some(Region::HueBar)
    } else {
        None
    }
}

fn image_handle(cache: &RefCell<Option<CachedImage>>, size: Size, hue: f32) -> image::Handle {
    let width = (size.width.max(1.0) * SUPERSAMPLE as f32).round() as u32;
    let height = (size.height.max(1.0) * SUPERSAMPLE as f32).round() as u32;
    let hue_key = hue.rem_euclid(360.0).round() as u32;

    if let Some(cached) = cache.borrow().as_ref()
        && cached.width == width
        && cached.height == height
        && cached.hue_key == hue_key
    {
        return cached.handle.clone();
    }

    let handle = render_handle(size, width, height, hue);

    *cache.borrow_mut() = Some(CachedImage {
        width,
        height,
        hue_key,
        handle: handle.clone(),
    });

    handle
}

fn render_handle(size: Size, width: u32, height: u32, hue: f32) -> image::Handle {
    let scale = SUPERSAMPLE as f32;
    let square = square_rect(size);
    let hue_bar = hue_bar_rect(size);
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            let sample = Point::new((x as f32 + 0.5) / scale, (y as f32 + 0.5) / scale);

            let color = if square.contains(sample) {
                let saturation = ((sample.x - square.x) / square.width).clamp(0.0, 1.0);
                let value = 1.0 - ((sample.y - square.y) / square.height).clamp(0.0, 1.0);

                hsv_to_rgba(hue, saturation, value, 1.0)
            } else if hue_bar.contains(sample) {
                let bar_hue = ((sample.x - hue_bar.x) / hue_bar.width).clamp(0.0, 1.0) * 360.0;

                hsv_to_rgba(bar_hue, 1.0, 1.0, 1.0)
            } else {
                pixels.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            };

            pixels.push((color.r.clamp(0.0, 1.0) * 255.0).round() as u8);
            pixels.push((color.g.clamp(0.0, 1.0) * 255.0).round() as u8);
            pixels.push((color.b.clamp(0.0, 1.0) * 255.0).round() as u8);
            pixels.push(255);
        }
    }

    image::Handle::from_rgba(width, height, pixels)
}
