use ev::mouseup;
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
use wasm_bindgen::{JsCast as _, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlDivElement};

use minesweep_core::{CellView, GameOptions, GameView};

#[derive(Debug, Clone)]
struct Transform {
    origin_x: f64,
    origin_y: f64,
    scale: f64,
}

const SCALE_FACTOR: f64 = 1.1;

impl Transform {
    fn scale(&mut self, wheel: f64) {
        match wheel.signum() {
            1. => self.scale /= SCALE_FACTOR,
            -1. => self.scale *= SCALE_FACTOR,
            _ => unreachable!(),
        }
        self.scale = self.scale.clamp(0.02, 2.);
    }

    fn cell_size(&self) -> f64 {
        CELL_SIZE * self.scale
    }

    fn cell_gap(&self) -> f64 {
        CELL_GAP * self.scale
    }
}

const CELL_SIZE: f64 = 50.;
const CELL_GAP: f64 = 25.;

fn clear(ctx: &CanvasRenderingContext2d, canvas: &HtmlElement<Canvas>) {
    ctx.save();
    ctx.set_fill_style(&"white".into());
    ctx.fill_rect(0., 0., canvas.width() as f64, canvas.height() as f64);
    ctx.restore();
}

fn draw_game_view(ctx: &CanvasRenderingContext2d, t: &Transform, view: &GameView) {
    ctx.save();
    let w = view.width();
    let h = view.height();
    let w_pixels = (w as f64 * (t.cell_size() + t.cell_gap())) - t.cell_gap();
    let h_pixels = (h as f64 * (t.cell_size() + t.cell_gap())) - t.cell_gap();
    ctx.translate(t.origin_x - (w_pixels / 2.), t.origin_y - (h_pixels / 2.))
        .unwrap();
    for (x, y, cell) in view.iter() {
        draw_cell(ctx, t, cell, x, y);
    }
    ctx.restore();
}

fn draw_cell(ctx: &CanvasRenderingContext2d, t: &Transform, cell: CellView, x: usize, y: usize) {
    match cell {
        minesweep_core::CellView::Unopened => ctx.set_fill_style(&"gray".into()),
        minesweep_core::CellView::Flagged => ctx.set_fill_style(&"blue".into()),
        minesweep_core::CellView::Questioned => ctx.set_fill_style(&"magenta".into()),
        minesweep_core::CellView::Opened(_) => ctx.set_fill_style(&"green".into()),
        minesweep_core::CellView::Mine => ctx.set_fill_style(&"orange".into()),
        minesweep_core::CellView::WrongMine => ctx.set_fill_style(&"yellow".into()),
        minesweep_core::CellView::Exploded => ctx.set_fill_style(&"red".into()),
    }
    ctx.fill_rect(
        x as f64 * (t.cell_size() + t.cell_gap()),
        y as f64 * (t.cell_size() + t.cell_gap()),
        t.cell_size(),
        t.cell_size(),
    );
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
    let (mouse_down, set_mouse_down) = create_signal(false);
    let (offset_x, set_offset_x) = create_signal(0f64);
    let (offset_y, set_offset_y) = create_signal(0f64);
    create_effect(move |_| {
        if !mouse_down() {
            return;
        }
        set_transform.update(|t| {
            t.origin_x = mouse_x() - offset_x();
            t.origin_y = mouse_y() - offset_y();
        });
    });
    let _ = use_event_listener(document(), mouseup, move |_| {
        set_mouse_down(false);
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
        draw_game_view(&ctx, &transform(), &view());
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
        <canvas ref=canvas on:click=move |ev| {
            if let Some((x, y)) = ray_cast(
                &transform(),
                &view(),
                mouse_x(),
                mouse_y(),
            ) {
                view.update(|view| view.left_click(x, y));
                ev.prevent_default();
            }
        } on:auxclick=move |ev| {
            log!("button: {}", ev.button());
            if let Some((x, y)) = ray_cast(
                &transform(),
                &view(),
                mouse_x(),
                mouse_y(),
            ) {
                match ev.button() {
                    1 => view.update(|view| view.middle_click(x, y)),
                    2 => view.update(|view| view.right_click(x, y)),
                    _ => {}
                }
                ev.prevent_default();
            }
        } on:contextmenu=move |ev| {
            if ray_cast(
                &transform(),
                &view(),
                mouse_x(),
                mouse_y(),
            ).is_some() {
                ev.prevent_default();
            }
        } on:mousedown=move |ev| {
            if ev.button() != 0 {
                return;
            }
            let None = ray_cast(
                &transform(),
                &view(),
                mouse_x(),
                mouse_y(),
            ) else {
                return;
            };
            set_offset_x(mouse_x() - transform().origin_x);
            set_offset_y(mouse_y() - transform().origin_y);
            set_mouse_down(true);
        } on:wheel=move |ev| {
            set_transform.update(|t| t.scale(ev.delta_y()));
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
