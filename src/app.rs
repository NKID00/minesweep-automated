use ev::{mousemove, mouseup};
use html::Canvas;
use js_sys::{Object, Reflect};
use leptos::logging::log;
use leptos::*;
use leptos_meta::*;
use leptos_use::{
    use_event_listener, use_mouse, use_mouse_in_element, use_window_size, UseMouseInElementReturn,
    UseMouseReturn, UseWindowSizeReturn,
};
use stylers::style_str;
use wasm_bindgen::{prelude::*, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlDivElement, HtmlImageElement};

use minesweep_core::{CellView, GameOptions, GameView, Gesture};

#[derive(Debug, Clone)]
struct Transform {
    origin_x: f64,
    origin_y: f64,
    scale: f64,
}

const SCALE_FACTOR: f64 = 1.1;

impl Transform {
    fn scale(&mut self, scale_origin_x: f64, scale_origin_y: f64, wheel: f64) {
        let before = self.scale;
        match wheel.signum() {
            1. => self.scale /= SCALE_FACTOR,
            -1. => self.scale *= SCALE_FACTOR,
            _ => unreachable!(),
        }
        self.scale = self.scale.clamp(0.02, 2.);
        let scale = self.scale / before;
        self.origin_x += (self.origin_x - scale_origin_x) * (scale - 1.);
        self.origin_y += (self.origin_y - scale_origin_y) * (scale - 1.);
    }

    fn cell_size(&self) -> f64 {
        CELL_SIZE * self.scale
    }

    fn cell_gap(&self) -> f64 {
        CELL_GAP * self.scale
    }
}

const CELL_SIZE: f64 = 50.;
const CELL_GAP: f64 = 10.;

fn clear(ctx: &CanvasRenderingContext2d, canvas: &HtmlElement<Canvas>) {
    ctx.save();
    ctx.set_fill_style(&"white".into());
    ctx.fill_rect(0., 0., canvas.width() as f64, canvas.height() as f64);
    ctx.restore();
}

fn draw_game_view(ctx: &CanvasRenderingContext2d, images: &Images, t: &Transform, view: &GameView) {
    ctx.save();
    let w = view.width();
    let h = view.height();
    let w_pixels = (w as f64 * (t.cell_size() + t.cell_gap())) - t.cell_gap();
    let h_pixels = (h as f64 * (t.cell_size() + t.cell_gap())) - t.cell_gap();
    ctx.translate(t.origin_x - (w_pixels / 2.), t.origin_y - (h_pixels / 2.))
        .unwrap();
    for (x, y, cell) in view.iter() {
        draw_cell(ctx, t, images, cell, x, y);
    }
    ctx.restore();
}

#[derive(Debug, Clone)]
struct Images {
    numbers: Vec<HtmlImageElement>,
    flag: HtmlImageElement,
    question: HtmlImageElement,
    mine: HtmlImageElement,
    wrong_mine: HtmlImageElement,
    explosion: HtmlImageElement,
}

fn draw_cell(
    ctx: &CanvasRenderingContext2d,
    t: &Transform,
    images: &Images,
    cell: CellView,
    x: usize,
    y: usize,
) {
    let x = x as f64 * (t.cell_size() + t.cell_gap());
    let y = y as f64 * (t.cell_size() + t.cell_gap());
    let w = t.cell_size();
    let h = t.cell_size();
    match cell {
        CellView::Unopened => {
            ctx.set_fill_style(&"#ccc".into());
            ctx.fill_rect(x, y, w, h);
        }
        CellView::Hovered => {
            ctx.set_fill_style(&"#ddd".into());
            ctx.fill_rect(x, y, w, h);
        }
        CellView::Pushed => {
            ctx.set_fill_style(&"#aaa".into());
            ctx.fill_rect(x, y, w, h);
        }
        _ => {
            let image = match cell {
                CellView::Flagged => &images.flag,
                CellView::Questioned => &images.question,
                CellView::Opened(n) => &images.numbers[n as usize],
                CellView::Mine => &images.mine,
                CellView::WrongMine => &images.wrong_mine,
                CellView::Exploded => &images.explosion,
                _ => unreachable!(),
            };
            ctx.draw_image_with_html_image_element_and_dw_and_dh(image, x, y, w, h)
                .unwrap();
        }
    }
}

fn ray_cast(t: &Transform, view: &GameView, mouse_x: f64, mouse_y: f64) -> Option<(usize, usize)> {
    let w = view.width();
    let h = view.height();
    let w_pixels = (w as f64 * (t.cell_size() + t.cell_gap())) - t.cell_gap();
    let h_pixels = (h as f64 * (t.cell_size() + t.cell_gap())) - t.cell_gap();
    let x = mouse_x - (t.origin_x - (w_pixels / 2.));
    let y = mouse_y - (t.origin_y - (h_pixels / 2.));
    // inside map && inside cell
    if (0. <= x && x <= w_pixels && 0. <= y && y <= h_pixels)
        && (x % (t.cell_size() + t.cell_gap()) <= t.cell_size()
            && y % (t.cell_size() + t.cell_gap()) <= t.cell_size())
    {
        Some((
            (x / (t.cell_size() + t.cell_gap()))
                .floor()
                .clamp(0., (w - 1) as f64) as usize,
            (y / (t.cell_size() + t.cell_gap()))
                .floor()
                .clamp(0., (h - 1) as f64) as usize,
        ))
    } else {
        None
    }
}

#[component]
pub fn Map(view: RwSignal<GameView>) -> impl IntoView {
    let canvas: NodeRef<Canvas> = create_node_ref();
    let (transform, set_transform) = create_signal(Transform {
        origin_x: 0.,
        origin_y: 0.,
        scale: 1.,
    });
    let images: Images = {
        let mut numbers = Vec::new();
        numbers.push(HtmlImageElement::new().unwrap());
        for n in 1..9 {
            let number = HtmlImageElement::new().unwrap();
            number.set_src(&format!("/public/{n}.svg"));
            numbers.push(number)
        }
        let flag = HtmlImageElement::new().unwrap();
        flag.set_src("/public/flag.svg");
        let question = HtmlImageElement::new().unwrap();
        question.set_src("/public/question.svg");
        let mine = HtmlImageElement::new().unwrap();
        mine.set_src("/public/mine.svg");
        let wrong_mine = HtmlImageElement::new().unwrap();
        wrong_mine.set_src("/public/wrong_mine.svg");
        let explosion = HtmlImageElement::new().unwrap();
        explosion.set_src("/public/explosion.svg");
        Images {
            numbers,
            flag,
            question,
            mine,
            wrong_mine,
            explosion,
        }
    };
    let UseWindowSizeReturn { width, height } = use_window_size();
    create_effect(move |_| {
        let canvas = canvas().unwrap();
        canvas.set_width(width() as u32);
        canvas.set_height(height() as u32);
        set_transform.update(|transform| {
            transform.origin_x = width() / 2.;
            transform.origin_y = height() / 2.;
        });
    });
    let UseMouseReturn {
        x: mouse_x,
        y: mouse_y,
        ..
    } = use_mouse();
    let (mouse_down, set_mouse_down) = create_signal(None);
    let (hover, set_hover) = create_signal(None::<(usize, usize)>);
    let (offset_x, set_offset_x) = create_signal(0f64);
    let (offset_y, set_offset_y) = create_signal(0f64);
    create_effect(move |_| {
        if mouse_down() != Some(0) {
            return;
        }
        set_transform.update(|t| {
            t.origin_x = mouse_x() - offset_x();
            t.origin_y = mouse_y() - offset_y();
        });
    });
    let _ = use_event_listener(document(), mouseup, move |_| {
        log!("up: {:?}", mouse_down());
        match (mouse_down(), hover()) {
            (Some(0), Some((x, y))) => {
                view.update(|view| view.left_click(x, y));
            }
            (Some(1), Some((x, y))) => {
                view.update(|view| view.middle_click(x, y));
            }
            (Some(2), Some((x, y))) => {
                view.update(|view| view.right_click(x, y));
            }
            _ => {}
        }
        set_mouse_down(None);
    });
    let _ = use_event_listener(document(), mousemove, move |ev| {
        log!("move: {}, {}", mouse_x(), mouse_y());
        if let Some((x, y)) = ray_cast(&transform(), &view(), mouse_x(), mouse_y()) {
            if hover() != Some((x, y)) {
                log!("hover: {}, {}", x, y);
                set_hover(Some((x, y)));
                view.update(|view| view.gesture(Gesture::Hover(x, y)));
            }
        } else {
            set_hover(None);
            view.update(|view| view.gesture(Gesture::None));
        }
    });
    create_effect(move |_| match (mouse_down(), hover()) {
        (None, Some((x, y))) => {
            view.update(|view| view.gesture(Gesture::Hover(x, y)));
        }
        (Some(0 | 2), Some((x, y))) => {
            view.update(|view| view.gesture(Gesture::LeftOrRightPush(x, y)));
        }
        (Some(1), Some((x, y))) => {
            view.update(|view| view.gesture(Gesture::MidPush(x, y)));
        }
        _ => {}
    });
    create_effect(move |_| {
        width.track();
        let canvas = canvas().unwrap();
        let options = Object::new();
        Reflect::set(&options, &"alpha".into(), &JsValue::FALSE).unwrap();
        let ctx = canvas
            .get_context_with_context_options("2d", &options)
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();
        clear(&ctx, &canvas);
        draw_game_view(&ctx, &images, &transform(), &view());
    });
    let (class_name, style_val) = style_str! {
        canvas {
            position: absolute;
            left: 0;
            top: 0;
        }
    };
    view! {
        class = class_name,
        <Style> { style_val } </Style>
        <canvas ref=canvas on:contextmenu=move |ev| {
            ev.prevent_default();
        } on:mousedown=move |ev| {
            log!("down: {}", ev.button());
            if ev.button() == 0 && hover().is_none() {
                set_offset_x(mouse_x() - transform().origin_x);
                set_offset_y(mouse_y() - transform().origin_y);
            }
            set_mouse_down(Some(ev.button()));
        } on:wheel=move |ev| {
            set_transform.update(|t| t.scale(mouse_x(), mouse_y(), ev.delta_y()));
        }> "Canvas required." </canvas>
    }
}

#[component]
pub fn Controls() -> impl IntoView {
    let div_ref = create_node_ref();
    let UseMouseInElementReturn {
        x: mouse_x,
        y: mouse_y,
        element_position_x,
        element_position_y,
        ..
    } = use_mouse_in_element(div_ref);
    let (mouse_down, set_mouse_down) = create_signal(false);
    let (offset_x, set_offset_x) = create_signal(0f64);
    let (offset_y, set_offset_y) = create_signal(0f64);
    create_effect(move |_| {
        if !mouse_down() {
            return;
        }
        let element: &HtmlDivElement = &div_ref().unwrap();
        element
            .style()
            .set_property("left", &format!("{}px", mouse_x() - offset_x()))
            .unwrap();
        element
            .style()
            .set_property("top", &format!("{}px", mouse_y() - offset_y()))
            .unwrap();
    });
    let _ = use_event_listener(document(), mouseup, move |_| {
        set_mouse_down(false);
    });
    let (class_name, style_val) = style_str! {
        h1 {
            font-size: 1.25rem;
            line-height: 1.75rem;
            font-weight: 700;
            margin: 0 0 1rem 0;
        }
        div {
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 1rem;
            padding: 2rem 2rem 1rem 2rem;
            border-radius: 0.75rem;
            border-width: 2px;
            border-color: rgba(118, 210, 255, 0.3);
            border-style: solid;
            box-shadow: 0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1);
            position: absolute;
            cursor: move;
            user-select: none;
            z-index: 10;
            background-color: white;
        }
    };
    view! {
        class = class_name,
        <Style> { style_val } </Style>
        <div ref=div_ref on:mousedown=move |ev| {
            if ev.button() != 0 {
                return;
            }
            set_offset_x(mouse_x() - element_position_x());
            set_offset_y(mouse_y() - element_position_y());
            set_mouse_down(true);
        }>
            <h1>"Minesweep Automated"</h1>
            <sl-dropdown>
                <sl-button slot="trigger" caret> "New Game" </sl-button>
                <sl-menu>
                    <sl-menu-item> "Easy" </sl-menu-item>
                    <sl-menu-item> "Medium" </sl-menu-item>
                    <sl-menu-item> "Hard" </sl-menu-item>
                    <sl-menu-item> "Expert" </sl-menu-item>
                </sl-menu>
            </sl-dropdown>
            <sl-switch> "Automation" </sl-switch>
            <p> "Mines: 0/10" </p>
            <p> "Time: 00:04" </p>
            <a href="https://github.com/NKID00" target="_blank" id="footer" class="link">
                <p> "© 2024 NKID00, under AGPL-3.0-or-later" </p>
            </a>
        </div>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    let options = GameOptions {
        size: (3, 3),
        safe_pos: None,
        mines: 3,
        seed: Some(1),
    };
    let state = options.build();
    let view = create_rw_signal(state.into());
    let (_class_name, style_val) = style_str! {};
    view! {
        class = class_name,
        <Style> { style_val } </Style>
        <Map view />
        <Controls />
    }
}
