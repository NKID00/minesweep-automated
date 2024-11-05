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

const SCALE_FACTOR: f64 = 1.1;
const PADDING: f64 = 20.;
const CELL_SIZE: f64 = 50.;
const CELL_GAP: f64 = 10.;

#[derive(Debug, Clone)]
struct Transform {
    origin_x: f64,
    origin_y: f64,
    scale: f64,
}

impl Transform {
    fn scale(&mut self, scale_origin_x: f64, scale_origin_y: f64, wheel: f64) {
        let before = self.scale;
        match wheel.signum() {
            1. => self.scale /= SCALE_FACTOR,
            -1. => self.scale *= SCALE_FACTOR,
            _ => unreachable!(),
        }
        self.scale = self.scale.clamp(0.015, 1.5);
        let scale = self.scale / before;
        self.origin_x += (self.origin_x - scale_origin_x) * (scale - 1.);
        self.origin_y += (self.origin_y - scale_origin_y) * (scale - 1.);
    }
}

fn clear(ctx: &CanvasRenderingContext2d, canvas: &HtmlElement<Canvas>) {
    ctx.save();
    ctx.set_fill_style(&"white".into());
    ctx.fill_rect(0., 0., canvas.width() as f64, canvas.height() as f64);
    ctx.restore();
}

fn map_pixel_size(view: &GameView) -> (f64, f64) {
    (
        (view.width() as f64 * (CELL_SIZE + CELL_GAP)) - CELL_GAP,
        (view.height() as f64 * (CELL_SIZE + CELL_GAP)) - CELL_GAP,
    )
}

fn map_pixel_size_with_padding(view: &GameView) -> (f64, f64) {
    (
        (view.width() as f64 * (CELL_SIZE + CELL_GAP)) - CELL_GAP + PADDING * 2.,
        (view.height() as f64 * (CELL_SIZE + CELL_GAP)) - CELL_GAP + PADDING * 2.,
    )
}

fn draw_game_view(ctx: &CanvasRenderingContext2d, images: &Images, view: &GameView) {
    ctx.save();
    let (w_pixels, h_pixels) = map_pixel_size(view);
    ctx.set_stroke_style(&"#777".into());
    ctx.set_line_width(2.);
    ctx.stroke_rect(
        PADDING / 2.,
        PADDING / 2.,
        w_pixels + PADDING,
        h_pixels + PADDING,
    );
    for (x, y, cell) in view.iter() {
        draw_cell(ctx, images, cell, x, y);
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

fn draw_cell(ctx: &CanvasRenderingContext2d, images: &Images, cell: CellView, x: usize, y: usize) {
    let x = x as f64 * (CELL_SIZE + CELL_GAP) + PADDING;
    let y = y as f64 * (CELL_SIZE + CELL_GAP) + PADDING;
    let w = CELL_SIZE;
    let h = CELL_SIZE;
    ctx.set_fill_style(&"white".into());
    ctx.fill_rect(
        x - CELL_GAP / 2.,
        y - CELL_GAP / 2.,
        w + CELL_GAP,
        h + CELL_GAP,
    );
    match cell {
        CellView::Unopened => {
            ctx.set_fill_style(&"#ddd".into());
            ctx.fill_rect(x, y, w, h);
        }
        CellView::Hovered => {
            ctx.set_fill_style(&"#eee".into());
            ctx.fill_rect(x, y, w, h);
        }
        CellView::Pushed => {
            ctx.set_fill_style(&"#ccc".into());
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
    let (w_pixels, h_pixels) = map_pixel_size_with_padding(view);
    let x = (mouse_x - t.origin_x) / t.scale;
    let y = (mouse_y - t.origin_y) / t.scale;
    // inside map && inside cell
    if 0. <= x && x <= w_pixels && 0. <= y && y <= h_pixels {
        let x = x - PADDING;
        let y = y - PADDING;
        if x % (CELL_SIZE + CELL_GAP) <= CELL_SIZE && y % (CELL_SIZE + CELL_GAP) <= CELL_SIZE {
            Some((
                (x / (CELL_SIZE + CELL_GAP))
                    .floor()
                    .clamp(0., (w - 1) as f64) as usize,
                (y / (CELL_SIZE + CELL_GAP))
                    .floor()
                    .clamp(0., (h - 1) as f64) as usize,
            ))
        } else {
            None
        }
    } else {
        None
    }
}

#[component]
pub fn Map(view: RwSignal<GameView>) -> impl IntoView {
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

    let canvas: NodeRef<Canvas> = create_node_ref();
    let transform = create_rw_signal(Transform {
        origin_x: 0.,
        origin_y: 0.,
        scale: 1.,
    });

    let UseWindowSizeReturn { width, height } = use_window_size();
    // initialize canvas and transform
    create_effect(move |_| {
        let canvas = canvas().unwrap();
        let view = view.get_untracked();
        let (w_pixels, h_pixels) = map_pixel_size_with_padding(&view);
        canvas.set_width(w_pixels as u32);
        canvas.set_height(h_pixels as u32);
        let options = Object::new();
        Reflect::set(&options, &"alpha".into(), &JsValue::FALSE).unwrap();
        let ctx = canvas
            .get_context_with_context_options("2d", &options)
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();
        clear(&ctx, &canvas);
        update!(|transform| {
            transform.origin_x = width.get_untracked() / 2. - w_pixels / 2.;
            transform.origin_y = height.get_untracked() / 2. - h_pixels / 2.;
        })
    });

    let UseMouseReturn {
        x: mouse_x,
        y: mouse_y,
        ..
    } = use_mouse();
    let (mouse_down, set_mouse_down) = create_signal(None);
    let (hover, set_hover) = create_signal(None::<(usize, usize)>);
    let (offset_x, set_offset_x) = create_signal(None::<f64>);
    let (offset_y, set_offset_y) = create_signal(None::<f64>);

    // update transform according to mouse state
    create_effect(move |_| {
        if mouse_down() != Some(0) || offset_x().is_none() || offset_y().is_none() {
            return;
        }
        update!(|transform| {
            transform.origin_x = mouse_x() - offset_x().unwrap();
            transform.origin_y = mouse_y() - offset_y().unwrap();
        });
    });

    // mouse event listener
    let _ = use_event_listener(document(), mouseup, move |_| {
        log!("up: {:?}", mouse_down());
        match (mouse_down(), hover()) {
            (Some(0), Some((x, y))) => {
                update!(|view| view.left_click(x, y));
            }
            (Some(1), Some((x, y))) => {
                update!(|view| view.middle_click(x, y));
            }
            (Some(2), Some((x, y))) => {
                update!(|view| view.right_click(x, y));
            }
            _ => {}
        }
        set_offset_x(None);
        set_offset_y(None);
        set_mouse_down(None);
    });
    let _ = use_event_listener(document(), mousemove, move |_| {
        log!("move: {}, {}", mouse_x(), mouse_y());
        let ray_cast_result =
            with!(|transform, view| ray_cast(transform, view, mouse_x(), mouse_y()));
        if let Some((x, y)) = ray_cast_result {
            if hover() != Some((x, y)) {
                log!("hover: {}, {}", x, y);
                set_hover(Some((x, y)));
                update!(|view| view.gesture(Gesture::Hover(x, y)));
            }
        } else if hover().is_some() {
            set_hover(None);
            update!(|view| view.gesture(Gesture::None));
        }
    });

    // update hover
    create_effect(move |_| match (mouse_down(), hover()) {
        (None, Some((x, y))) => {
            update!(|view| view.gesture(Gesture::Hover(x, y)));
        }
        (Some(0 | 2), Some((x, y))) => {
            update!(|view| view.gesture(Gesture::LeftOrRightPush(x, y)));
        }
        (Some(1), Some((x, y))) => {
            update!(|view| view.gesture(Gesture::MidPush(x, y)));
        }
        _ => {}
    });

    // transform
    create_effect(move |_| {
        let canvas = canvas().unwrap();
        let t = transform();
        log!("transform: {t:?}");
        (*canvas)
            .style()
            .set_property(
                "transform",
                &format!(
                    "translate({}px, {}px) scale({})",
                    t.origin_x, t.origin_y, t.scale
                ),
            )
            .unwrap();
    });

    // redraw
    create_effect(move |_| {
        log!("redraw, bad");
        let canvas = canvas().unwrap();
        let options = Object::new();
        Reflect::set(&options, &"alpha".into(), &JsValue::FALSE).unwrap();
        let ctx = canvas
            .get_context_with_context_options("2d", &options)
            .unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .unwrap();
        with!(|view| draw_game_view(&ctx, &images, view));
    });

    let (class_name, style_val) = style_str! {
        div {
            position:absolute;
            top:0;
            left:0;
            bottom:0;
            right:0;
            height:100%;
            width:100%;
        }
        canvas {
            position: absolute;
            left: 0;
            top: 0;
            transform-origin: top left;
        }
    };
    view! {
        class = class_name,
        <Style> { style_val } </Style>
        <div on:contextmenu=move |ev| {
            ev.prevent_default();
        } on:mousedown=move |ev| {
            log!("down: {}", ev.button());
            let hover = hover();
            if ev.button() == 0
                && (hover.is_none()
                    || with!(|view| view.is_draggable(hover.unwrap().0, hover.unwrap().1)))
            {
                with!(|transform| set_offset_x(Some(mouse_x() - transform.origin_x)));
                with!(|transform| set_offset_y(Some(mouse_y() - transform.origin_y)));
            }
            set_mouse_down(Some(ev.button()));
        } on:wheel=move |ev| {
            update!(|transform| transform.scale(mouse_x(), mouse_y(), ev.delta_y()));
        }>
            <canvas on:contextmenu=move |ev| {
                ev.prevent_default();
            } ref=canvas> "Canvas required." </canvas>
        </div>
    }
}

#[component]
pub fn Controls(view: RwSignal<GameView>) -> impl IntoView {
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
            <p> { move || with!(|view| format!("Mines: {}/{}", view.flags, view.mines)) } </p>
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
        size: (200, 200),
        safe_pos: None,
        mines: 10,
        seed: Some(1),
    };
    let state = options.build();
    let view = create_rw_signal(state.into());
    let (_class_name, style_val) = style_str! {};
    view! {
        class = class_name,
        <Style> { style_val } </Style>
        <Map view />
        <Controls view />
    }
}
