use gridvana_core::color::{hsv_to_rgba, rgb_to_hsv};
use gridvana_core::model::Rgba;
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Event, Frame, Geometry, Path, Stroke};
use iced::widget::image;
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme};
use std::cell::RefCell;

use crate::Message;

const WHEEL_SIZE: f32 = 118.0;
const WHEEL_PADDING: f32 = 4.0;
const MARKER_RADIUS: f32 = 5.0;
const WHEEL_SUPERSAMPLE: u32 = 2;

pub fn view<'a>(color: Rgba) -> Element<'a, Message> {
    Canvas::new(ColorWheel { color })
        .width(Length::Fixed(WHEEL_SIZE))
        .height(Length::Fixed(WHEEL_SIZE))
        .into()
}

struct ColorWheel {
    color: Rgba,
}

#[derive(Clone)]
struct WheelImage {
    width: u32,
    height: u32,
    handle: image::Handle,
}

#[derive(Default)]
pub struct State {
    dragging: bool,
    wheel_image: RefCell<Option<WheelImage>>,
}

impl canvas::Program<Message> for ColorWheel {
    type State = State;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let center = wheel_center(bounds.size());
        let radius = wheel_radius(bounds.size());

        let handle = wheel_image_handle(&state.wheel_image, bounds.size());
        frame.draw_image(Rectangle::with_size(bounds.size()), &handle);

        let wheel_outline = Path::circle(center, radius);
        frame.stroke(
            &wheel_outline,
            Stroke::default().with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.18)),
        );

        let (hue, saturation, _) = rgb_to_hsv(self.color);
        let angle = hue.to_radians();
        let marker_distance = saturation * radius;
        let marker_center = Point::new(
            center.x + marker_distance * angle.cos(),
            center.y + marker_distance * angle.sin(),
        );
        let marker = Path::circle(marker_center, MARKER_RADIUS);

        frame.fill(
            &marker,
            Color::from_rgba(self.color.r, self.color.g, self.color.b, 1.0),
        );
        frame.stroke(
            &marker,
            Stroke::default()
                .with_color(Color::WHITE)
                .with_width(if state.dragging { 2.5 } else { 2.0 }),
        );
        frame.stroke(
            &marker,
            Stroke::default()
                .with_color(Color::from_rgba(0.0, 0.0, 0.0, 0.55))
                .with_width(1.0),
        );

        if cursor
            .position_in(bounds)
            .is_some_and(|position| point_in_wheel(position, center, radius))
        {
            let hover_ring = Path::circle(center, radius + 1.0);
            frame.stroke(
                &hover_ring,
                Stroke::default().with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.3)),
            );
        }

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let cursor_position = cursor.position_in(bounds);
        let center = wheel_center(bounds.size());
        let radius = wheel_radius(bounds.size());

        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let cursor_position = cursor_position?;

                if point_in_wheel(cursor_position, center, radius) {
                    state.dragging = true;
                    return Some(canvas::Action::publish(color_message_from_point(
                        cursor_position,
                        center,
                        radius,
                        self.color,
                    )));
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if state.dragging => {
                let cursor_position = cursor_position?;

                return Some(canvas::Action::publish(color_message_from_point(
                    cursor_position,
                    center,
                    radius,
                    self.color,
                )));
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.dragging = false;
            }
            _ => {}
        }

        None
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let Some(position) = cursor.position_in(bounds) else {
            return mouse::Interaction::default();
        };

        let center = wheel_center(bounds.size());
        let radius = wheel_radius(bounds.size());

        if state.dragging || point_in_wheel(position, center, radius) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

fn wheel_center(size: Size) -> Point {
    Point::new(size.width * 0.5, size.height * 0.5)
}

fn wheel_radius(size: Size) -> f32 {
    ((size.width.min(size.height)) * 0.5 - WHEEL_PADDING).max(1.0)
}

fn point_in_wheel(point: Point, center: Point, radius: f32) -> bool {
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    (dx * dx + dy * dy) <= radius * radius
}

fn color_message_from_point(point: Point, center: Point, radius: f32, current: Rgba) -> Message {
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    let distance = (dx * dx + dy * dy).sqrt();
    let hue = dy.atan2(dx).to_degrees().rem_euclid(360.0);
    let saturation = (distance / radius).clamp(0.0, 1.0);
    let (_, _, value) = rgb_to_hsv(current);

    Message::SelectColor(hsv_to_rgba(hue, saturation, value.max(0.0), current.a))
}

fn wheel_image_handle(cache: &RefCell<Option<WheelImage>>, size: Size) -> image::Handle {
    let width = (size.width.max(1.0) * WHEEL_SUPERSAMPLE as f32).round() as u32;
    let height = (size.height.max(1.0) * WHEEL_SUPERSAMPLE as f32).round() as u32;

    if let Some(image) = cache.borrow().as_ref()
        && image.width == width
        && image.height == height
    {
        return image.handle.clone();
    }

    let handle = render_wheel_handle(size, width, height);

    *cache.borrow_mut() = Some(WheelImage {
        width,
        height,
        handle: handle.clone(),
    });

    handle
}

fn render_wheel_handle(size: Size, width: u32, height: u32) -> image::Handle {
    let center = Point::new(width as f32 * 0.5, height as f32 * 0.5);
    let radius = wheel_radius(size) * WHEEL_SUPERSAMPLE as f32;
    let edge_softness = 1.25 * WHEEL_SUPERSAMPLE as f32;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            let sample = Point::new(x as f32 + 0.5, y as f32 + 0.5);
            let dx = sample.x - center.x;
            let dy = sample.y - center.y;
            let distance = (dx * dx + dy * dy).sqrt();
            let alpha = ((radius + edge_softness - distance) / edge_softness).clamp(0.0, 1.0);

            if alpha <= 0.0 {
                pixels.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }

            let hue = dy.atan2(dx).to_degrees().rem_euclid(360.0);
            let saturation = (distance / radius).clamp(0.0, 1.0);
            let color = hsv_to_rgba(hue, saturation, 1.0, alpha);

            pixels.push((color.r.clamp(0.0, 1.0) * 255.0).round() as u8);
            pixels.push((color.g.clamp(0.0, 1.0) * 255.0).round() as u8);
            pixels.push((color.b.clamp(0.0, 1.0) * 255.0).round() as u8);
            pixels.push((color.a.clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }

    image::Handle::from_rgba(width, height, pixels)
}
