use automation_worker::Automation;
use ev::{mousemove, mouseup};
use futures::{SinkExt, StreamExt};
use gloo_worker::Spawnable;
use html::Canvas;
use js_sys::{Object, Reflect};
use leptos::logging::log;
use leptos::*;
use leptos_dom::helpers::set_property;
use leptos_meta::*;
use leptos_use::{
    use_event_listener, use_interval, use_mouse, use_mouse_in_element, use_window_size,
    UseIntervalReturn, UseMouseInElementReturn, UseMouseReturn, UseWindowSizeReturn,
};
use serde::{Deserialize, Serialize};
use stylers::style_str;
use wasm_bindgen::{prelude::*, JsValue};
use web_sys::{CanvasRenderingContext2d, HtmlDivElement, HtmlImageElement};

use minesweep_core::{
    CellView, Difficulty, GameOptions, GameResult, GameView, Gesture, RedrawCells,
};

const INITIAL_SCALE: f64 = 1.;
const SCALE_FACTOR: f64 = 1.1;
const PADDING: f64 = 20.;
const CELL_SIZE: f64 = 50.;
const CELL_GAP: f64 = 2.;

fn timestamp() -> f64 {
    window().performance().unwrap().now() as f64 / 1000.
}

#[derive(Debug, Clone)]
struct Transform {
    origin_x: f64,
    origin_y: f64,
    scale: f64,
}

impl Transform {
    fn wheel(&mut self, scale_origin_x: f64, scale_origin_y: f64, wheel: f64) {
        self.scale(
            scale_origin_x,
            scale_origin_y,
            match wheel.signum() {
                1. => 1. / SCALE_FACTOR,
                -1. => SCALE_FACTOR,
                _ => unreachable!(),
            },
        );
    }

    fn scale(&mut self, scale_origin_x: f64, scale_origin_y: f64, scale: f64) {
        let before = self.scale;
        self.scale *= scale;
        self.scale = self.scale.clamp(0.1, 1.);
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

fn map_pixel_size(view: &MaybeUninitGameView) -> (f64, f64) {
    (
        (view.width() as f64 * (CELL_SIZE + CELL_GAP)) - CELL_GAP,
        (view.height() as f64 * (CELL_SIZE + CELL_GAP)) - CELL_GAP,
    )
}

fn map_pixel_size_with_padding(view: &MaybeUninitGameView) -> (f64, f64) {
    (
        (view.width() as f64 * (CELL_SIZE + CELL_GAP)) - CELL_GAP + PADDING * 2.,
        (view.height() as f64 * (CELL_SIZE + CELL_GAP)) - CELL_GAP + PADDING * 2.,
    )
}

fn init_view(ctx: &CanvasRenderingContext2d, images: &Images, view: &MaybeUninitGameView) {
    let (w_pixels, h_pixels) = map_pixel_size(view);
    ctx.set_stroke_style(&"#777".into());
    ctx.set_line_width(2.);
    ctx.stroke_rect(
        PADDING / 2.,
        PADDING / 2.,
        w_pixels + PADDING,
        h_pixels + PADDING,
    );
    for (x, y) in RedrawCells::redraw_all(view.width(), view.height()).iter() {
        redraw_cell(ctx, images, view.cell(*x, *y), *x, *y);
    }
}

fn redraw_view(
    ctx: &CanvasRenderingContext2d,
    images: &Images,
    view: &MaybeUninitGameView,
    redraw: &RedrawCells,
) {
    for (x, y) in redraw.iter() {
        redraw_cell(ctx, images, view.cell(*x, *y), *x, *y);
    }
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

fn redraw_cell(
    ctx: &CanvasRenderingContext2d,
    images: &Images,
    cell: CellView,
    x: usize,
    y: usize,
) {
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
        CellView::Unopened | CellView::Hovered | CellView::Pushed => {
            match cell {
                CellView::Unopened => ctx.set_fill_style(&"#f0f0f0".into()),
                CellView::Hovered => ctx.set_fill_style(&"#f3f3f3".into()),
                CellView::Pushed => ctx.set_fill_style(&"#e0e0e0".into()),
                _ => unreachable!(),
            }
            ctx.begin_path();
            ctx.round_rect_with_f64(x, y, w, h, 3.).unwrap();
            ctx.fill();
        }
        _ => {
            match cell {
                CellView::Flagged => ctx.set_fill_style(&"#f0f0f0".into()),
                CellView::Questioned => ctx.set_fill_style(&"#f0f0f0".into()),
                CellView::Opened(_) => ctx.set_fill_style(&"white".into()),
                CellView::Mine => ctx.set_fill_style(&"white".into()),
                CellView::WrongMine => ctx.set_fill_style(&"white".into()),
                CellView::Exploded => ctx.set_fill_style(&"white".into()),
                _ => unreachable!(),
            }
            ctx.begin_path();
            ctx.round_rect_with_f64(x, y, w, h, 3.).unwrap();
            ctx.fill();
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

fn ray_cast(
    t: &Transform,
    view: &MaybeUninitGameView,
    mouse_x: f64,
    mouse_y: f64,
) -> Option<(usize, usize)> {
    let w = view.width();
    let h = view.height();
    let (w_pixels, h_pixels) = map_pixel_size(view);
    let x = (mouse_x - t.origin_x) / t.scale - PADDING;
    let y = (mouse_y - t.origin_y) / t.scale - PADDING;
    let x = x + CELL_GAP / 2.;
    let y = y + CELL_GAP / 2.;
    // inside map && inside cell
    if 0. <= x && x <= (w_pixels + CELL_GAP) && 0. <= y && y <= (h_pixels + CELL_GAP) {
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
}

#[component]
fn Map(view: RwSignal<MaybeUninitGameView>, redraw: RwSignal<RedrawCells>) -> impl IntoView {
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
    create_effect({
        let images = images.clone();
        move |previous_map_size| {
            redraw.track();
            let map_size = view.with_untracked(|view| (view.width(), view.height()));
            if previous_map_size == Some(map_size) {
                return map_size;
            }
            let begin = timestamp();
            let canvas = canvas().unwrap();
            let (w_pixels, h_pixels) = view.with_untracked(map_pixel_size_with_padding);
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
                transform.origin_x = (width.get_untracked() / 2. - w_pixels / 2.) * INITIAL_SCALE;
                transform.origin_y = (height.get_untracked() / 2. - h_pixels / 2.) * INITIAL_SCALE;
                transform.scale = INITIAL_SCALE;
            });
            view.with_untracked(|view| init_view(&ctx, &images, view));
            log!("init {:.3}s", timestamp() - begin);
            map_size
        }
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
        match (mouse_down(), hover()) {
            (Some(0), Some((x, y))) => {
                let mut next_redraw = Default::default();
                update!(|view| next_redraw = view.left_click(x, y));
                redraw.set(next_redraw);
            }
            (Some(1), Some((x, y))) => {
                let mut next_redraw = Default::default();
                update!(|view| next_redraw = view.middle_click(x, y));
                redraw.set(next_redraw);
            }
            (Some(2), Some((x, y))) => {
                let mut next_redraw = Default::default();
                update!(|view| next_redraw = view.right_click(x, y));
                redraw.set(next_redraw);
            }
            _ => {}
        }
        set_offset_x(None);
        set_offset_y(None);
        set_mouse_down(None);
    });
    let _ = use_event_listener(document(), mousemove, move |_| {
        let ray_cast_result =
            with!(|transform, view| ray_cast(transform, view, mouse_x(), mouse_y()));
        if let Some((x, y)) = ray_cast_result {
            if hover() != Some((x, y)) {
                set_hover(Some((x, y)));
            }
        } else if hover().is_some() {
            set_hover(None);
        }
    });

    // update hover
    create_effect(move |_| match (mouse_down(), hover()) {
        (_, None) => {
            let mut next_redraw = Default::default();
            update!(|view| next_redraw = view.gesture(Gesture::None));
            redraw.set(next_redraw);
        }
        (None, Some((x, y))) => {
            let mut next_redraw = Default::default();
            update!(|view| next_redraw = view.gesture(Gesture::Hover(x, y)));
            redraw.set(next_redraw);
        }
        (Some(0 | 2), Some((x, y))) => {
            let mut redraw_1 = Default::default();
            update!(|view| redraw_1 = view.gesture(Gesture::LeftOrRightPush(x, y)));
            redraw.set(redraw_1);
        }
        (Some(1), Some((x, y))) => {
            let mut redraw_1 = Default::default();
            update!(|view| redraw_1 = view.gesture(Gesture::MidPush(x, y)));
            redraw.set(redraw_1);
        }

        _ => {}
    });

    // transform
    create_effect(move |_| {
        let canvas = canvas().unwrap();
        let t = transform();
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
        with!(|redraw| if !redraw.is_empty() {
            let begin = timestamp();
            let canvas = canvas().unwrap();
            let options = Object::new();
            Reflect::set(&options, &"alpha".into(), &JsValue::FALSE).unwrap();
            let ctx = canvas
                .get_context_with_context_options("2d", &options)
                .unwrap()
                .unwrap()
                .dyn_into::<CanvasRenderingContext2d>()
                .unwrap();
            view.with_untracked(|view| redraw_view(&ctx, &images, view, redraw));
            log!("redraw {:.3}s", timestamp() - begin);
        });
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
            update!(|transform| transform.wheel(mouse_x(), mouse_y(), ev.delta_y()));
        }>
            <canvas on:contextmenu=move |ev| {
                ev.prevent_default();
            } ref=canvas> "Canvas required." </canvas>
        </div>
    }
}

#[wasm_bindgen(inline_js = "export function drawer_show_ffi(drawer) { drawer.show(); }")]
extern "C" {
    fn drawer_show_ffi(drawer: &JsValue);
}

fn drawer_show(drawer: NodeRef<html::Custom>) {
    drawer_show_ffi(&(drawer.get_untracked().unwrap().into_any()));
}

#[wasm_bindgen(inline_js = "export function drawer_hide_ffi(drawer) { drawer.hide(); }")]
extern "C" {
    fn drawer_hide_ffi(drawer: &JsValue);
}

fn drawer_hide(drawer: NodeRef<html::Custom>) {
    drawer_hide_ffi(&(drawer.get_untracked().unwrap().into_any()));
}

#[wasm_bindgen(inline_js = "export function alert_toast_ffi(alert) { alert.toast(); }")]
extern "C" {
    fn alert_toast_ffi(alert: &JsValue);
}

fn alert_toast(alert: NodeRef<html::Custom>) {
    alert_toast_ffi(&(alert.get_untracked().unwrap().into_any()));
}

fn into_html_element_untracked(ref_: NodeRef<html::Custom>) -> web_sys::HtmlElement {
    (*ref_.get_untracked().unwrap().into_any()).clone()
}

fn read_input_untracked(ref_: NodeRef<html::Custom>) -> Option<i64> {
    Reflect::get(&into_html_element_untracked(ref_), &"value".into())
        .ok()?
        .as_string()?
        .parse()
        .ok()
}

#[component]
fn Controls(
    view: RwSignal<MaybeUninitGameView>,
    redraw: RwSignal<RedrawCells>,
    new_game: WriteSignal<GameOptions>,
    restart: Trigger,
) -> impl IntoView {
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
    let seed_ref: NodeRef<html::Custom> = create_node_ref();
    let (difficulty, set_difficulty) = create_signal(Difficulty::Easy);
    let width_ref: NodeRef<html::Custom> = create_node_ref();
    let height_ref: NodeRef<html::Custom> = create_node_ref();
    let mines_ref: NodeRef<html::Custom> = create_node_ref();
    let new_game_drawer_ref: NodeRef<html::Custom> = create_node_ref();
    let invalid_config_alert_ref: NodeRef<html::Custom> = create_node_ref();
    let restart_dialog_ref: NodeRef<html::Custom> = create_node_ref();
    let UseIntervalReturn {
        counter,
        reset,
        is_active,
        pause,
        resume,
        ..
    } = use_interval(1000);
    create_effect({
        let reset = reset.clone();
        let pause = pause.clone();
        move |_| {
            with!(|view| match view {
                MaybeUninitGameView::Uninit { .. } => {
                    reset();
                    pause();
                }
                MaybeUninitGameView::GameView(view) =>
                    if view.result != GameResult::Playing {
                        pause();
                    } else if !is_active.get_untracked() {
                        resume()
                    },
            });
        }
    });
    create_effect(move |_| {
        restart.track();
        reset();
        pause();
    });
    let (automation, set_automation) = create_signal(false);
    let automation_switch_ref: NodeRef<html::Custom> = create_node_ref();
    let automation_fail_ref: NodeRef<html::Custom> = create_node_ref();
    let bridge = store_value(Automation::spawner().spawn("./automation-worker.js"));
    let automation_result = create_resource(
        move || (),
        move |_| async move {
            let view = view.get_untracked();
            if !view.is_playing() {
                return None;
            }
            match view {
                MaybeUninitGameView::Uninit { .. } => None,
                MaybeUninitGameView::GameView(view) => {
                    let mut bridge = with!(|bridge| bridge.fork());
                    bridge.send(view).await.unwrap();
                    bridge.next().await
                }
            }
        },
    );
    let automation_in_progress = automation_result.loading();
    // redraw after automation step
    create_effect(move |_| {
        if automation_in_progress() {
            return;
        }
        let Some(Some((duration, new_view, new_result))) = automation_result() else {
            return;
        };
        if let Some(new_result) = new_result {
            log!("automation {duration:.3}s, success");
            update!(move |view| *view = MaybeUninitGameView::GameView(new_view));
            update!(move |redraw| *redraw = new_result);
        } else {
            log!("automation {duration:.3}s, fail");
            set_property(
                &into_html_element_untracked(automation_switch_ref),
                "checked",
                &Some(JsValue::FALSE),
            );
            alert_toast(automation_fail_ref);
        }
    });
    // chain automation step
    create_effect(move |_| {
        if automation_in_progress()
            || !view.with_untracked(|view| view.is_playing())
            || !automation()
        {
            return;
        }
        let Some(Some((_, _, Some(_)))) = automation_result() else {
            return;
        };
        automation_result.refetch();
    });
    let (class_name, style_val) = style_str! {
        .non-draggable {
            cursor: auto;
        }
        #controls {
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
            z-index: 10;
            background-color: white;
            cursor: move;
            user-select: none;
        }
        #controls > h1 {
            font-size: 1.25rem;
            line-height: 1.75rem;
            font-weight: 700;
            margin: 0 0 1rem 0;
        }
        #new-game-drawer {
            --size: 60vw;
        }
        #random-seed {
            margin-right: 30vw;
        }
        #automation,
        #new-game-or-restart {
            display: flex;
            flex-direction: row;
            align-items: center;
            gap: 1rem;
        }
        #custom-difficulty-options {
            display: flex;
            flex-direction: row;
            align-items: center;
            gap: 2rem;
        }
    };
    view! {
        class = class_name,
        <Style> { style_val } </Style>
        <div id="controls" style="left: 5vw;" ref=div_ref on:mousedown=move |ev| {
            if ev.button() != 0 {
                return;
            }
            set_offset_x(mouse_x() - element_position_x());
            set_offset_y(mouse_y() - element_position_y());
            set_mouse_down(true);
        }>
            <h1>"Minesweep Automated"</h1>
            { move || with!(|view| match view {
                MaybeUninitGameView::Uninit { options, .. } => view! {
                    <p> "Tap to start" </p>
                    <p> { format!("Mines: 0/{}", options.difficulty.mines()) } </p>
                    <p> "Time: 00:00" </p>
                },
                MaybeUninitGameView::GameView(view) => view! {
                    <p> { match view.result {
                        GameResult::Playing => "Playing 😊",
                        GameResult::Win => "Win 😎",
                        GameResult::Lose => "Lose 😵",
                    } } </p>
                    <p> { format!("Mines: {}/{}", view.flags, view.mines) } </p>
                    <p> { move || with!(|counter| format!("Time: {:02}:{:02}", counter / 60, counter % 60)) } </p>
                },
            }) } <br />
            <div id="automation" class="non-draggable" on:mousedown=move |ev| ev.stop_propagation()>
                <sl-switch disabled={
                    move || with!(|view| matches!(view, MaybeUninitGameView::Uninit { .. }))
                } on:sl-change=move |ev: JsValue| {
                    let target = Reflect::get(&ev, &"target".into()).unwrap();
                    let checked = Reflect::get(&target, &"checked".into()).unwrap().as_bool().unwrap();
                    set_automation(checked);
                    if checked {
                        automation_result.refetch()
                    }
                } ref=automation_switch_ref> "Automation" </sl-switch>
                <sl-button disabled={
                    move || with!(|view| matches!(view, MaybeUninitGameView::Uninit { .. }))
                } on:click=move |_| automation_result.refetch()> "Step" </sl-button>
            </div>
            <sl-alert variant="danger" duration="2000" countdown="ltr" closable ref=automation_fail_ref>
                <sl-icon slot="icon" name="exclamation-octagon"></sl-icon>
                "No possible move found"
            </sl-alert>
            <div id="new-game-or-restart" class="non-draggable" on:mousedown=move |ev| ev.stop_propagation()>
                <sl-button on:click=move |_| drawer_show(new_game_drawer_ref)> "New Game" </sl-button>
                <sl-button disabled={ move || with!(|view| matches!(view, MaybeUninitGameView::Uninit { .. })) } on:click=move |_| drawer_show(restart_dialog_ref)> "Restart" </sl-button>
            </div>
            <sl-drawer label="New Game" id="new-game-drawer" class="non-draggable" ref=new_game_drawer_ref on:mousedown=move |ev| ev.stop_propagation()>
                <sl-input label="Random Seed" id="random-seed" pattern="[0-9]*" ref=seed_ref> "0" </sl-input> <br />
                <sl-radio-group label="Difficulty" name="difficulty" value="easy">
                    <sl-radio-button value="easy" on:click=move |_| set_difficulty(Difficulty::Easy)> "Easy" </sl-radio-button>
                    <sl-radio-button value="medium" on:click=move |_| set_difficulty(Difficulty::Medium)> "Medium" </sl-radio-button>
                    <sl-radio-button value="hard" on:click=move |_| set_difficulty(Difficulty::Hard)> "Hard" </sl-radio-button>
                    <sl-radio-button value="custom" on:click=move |_| {
                        set_difficulty(Difficulty::Custom {
                            width: 0,
                            height: 0,
                            mines: 0,
                        });
                    }> "Custom" </sl-radio-button>
                </sl-radio-group> <br />
                <div id="custom-difficulty-options">
                    <sl-input label="Width" pattern="[0-9]*" ref=width_ref disabled={
                        move || !matches!(difficulty(), Difficulty::Custom { .. })
                    }> "30" </sl-input>
                    <sl-input label="Height" pattern="[0-9]*" ref=height_ref disabled={
                        move || !matches!(difficulty(), Difficulty::Custom { .. })
                    }> "16" </sl-input>
                    <sl-input label="Mines" pattern="[0-9]*" ref=mines_ref disabled={
                        move || !matches!(difficulty(), Difficulty::Custom { .. })
                    }> "99" </sl-input>
                </div>
                <sl-button slot="footer" variant="primary" on:click=move |_| {
                    let seed = read_input_untracked(seed_ref).map(|seed| seed as u64);
                    let difficulty = match difficulty() {
                        Difficulty::Custom {..} => {
                            let Some(width) = read_input_untracked(width_ref) else {
                                alert_toast(invalid_config_alert_ref);
                                return;
                            };
                            let Some(height) = read_input_untracked(height_ref) else {
                                alert_toast(invalid_config_alert_ref);
                                return;
                            };
                            let Some(mines) = read_input_untracked(mines_ref) else {
                                alert_toast(invalid_config_alert_ref);
                                return;
                            };
                            if width <= 0 || height <= 0 || mines <= 0 || width * height <= mines {
                                alert_toast(invalid_config_alert_ref);
                                return;
                            }
                            Difficulty::Custom {
                                width: width as usize,
                                height: height as usize,
                                mines: mines as usize,
                            }
                        }
                        difficulty => difficulty,
                    };
                    drawer_hide(new_game_drawer_ref);
                    new_game(GameOptions { difficulty, safe_pos: None, seed });
                }> "New Game" </sl-button>
                <sl-button slot="footer" on:click=move |_| drawer_hide(new_game_drawer_ref)> "Cancel" </sl-button>
            </sl-drawer>
            <sl-alert variant="danger" duration="2000" countdown="ltr" closable ref=invalid_config_alert_ref>
                <sl-icon slot="icon" name="exclamation-octagon"></sl-icon>
                "Invalid configuration"
            </sl-alert>
            <sl-dialog label="Restart Confirm" class="non-draggable" ref=restart_dialog_ref on:mousedown=move |ev| ev.stop_propagation()>
                "Do you want to restart the game?"
                <sl-button slot="footer" variant="primary" on:click=move |_| {
                    drawer_hide(restart_dialog_ref);
                    restart.notify();
                }> "Restart" </sl-button>
                <sl-button slot="footer" on:click=move |_| drawer_hide(restart_dialog_ref)> "Cancel" </sl-button>
            </sl-dialog>
            { move || with!(|view| match view {
                MaybeUninitGameView::Uninit { options, .. } =>
                    if options.seed.is_some() {
                        view! { <p> { format!("Seed: {}", options.seed.unwrap()) } </p> }.into_view()
                    } else {
                        ().into_view()
                    }
                MaybeUninitGameView::GameView(view) => view! {
                    <p> { format!("Seed: {}", view.options().seed.unwrap()) } </p>
                }.into_view(),
            }) } <br />
            <a href="https://github.com/NKID00" target="_blank" id="footer" class="link non-draggable" on:mousedown=move |ev| ev.stop_propagation()>
                <p> "© 2024 NKID00, under AGPL-3.0-or-later" </p>
            </a>
        </div>
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum MaybeUninitGameView {
    Uninit {
        gesture: Gesture,
        options: GameOptions,
    },
    GameView(GameView),
}

impl MaybeUninitGameView {
    fn width(&self) -> usize {
        match self {
            MaybeUninitGameView::Uninit { options, .. } => options.difficulty.width(),
            MaybeUninitGameView::GameView(view) => view.width(),
        }
    }

    fn height(&self) -> usize {
        match self {
            MaybeUninitGameView::Uninit { options, .. } => options.difficulty.height(),
            MaybeUninitGameView::GameView(view) => view.height(),
        }
    }

    fn restart(&mut self) {
        if let MaybeUninitGameView::GameView(view) = self {
            *self = MaybeUninitGameView::Uninit {
                gesture: view.gesture,
                options: view.options(),
            };
            self.init();
        }
    }

    fn init(&mut self) {
        if let MaybeUninitGameView::Uninit { gesture, options } = self {
            let mut view = GameView::from(options.clone().build());
            view.gesture(*gesture);
            *self = MaybeUninitGameView::GameView(view);
        }
    }

    fn cell(&self, x: usize, y: usize) -> CellView {
        use CellView::*;
        match self {
            MaybeUninitGameView::Uninit {
                gesture,
                options: _,
            } => match *gesture {
                Gesture::Hover(x0, y0) if x == x0 && y == y0 => Hovered,
                Gesture::LeftOrRightPush(x0, y0) if x == x0 && y == y0 => Pushed,
                Gesture::MidPush(x0, y0) if x == x0 && y == y0 => Hovered,
                Gesture::MidPush(x0, y0)
                    if x as i32 - 1 <= x0 as i32
                        && x0 <= x + 1
                        && y as i32 - 1 <= y0 as i32
                        && y0 <= y + 1 =>
                {
                    Pushed
                }
                _ => Unopened,
            },
            MaybeUninitGameView::GameView(view) => view.cell(x, y),
        }
    }

    fn left_click(&mut self, x: usize, y: usize) -> RedrawCells {
        match self {
            MaybeUninitGameView::Uninit {
                gesture: _,
                options,
            } => {
                options.safe_pos = Some((x, y));
                self.init();
                self.left_click(x, y)
            }
            MaybeUninitGameView::GameView(view) => view.left_click(x, y),
        }
    }

    fn right_click(&mut self, x: usize, y: usize) -> RedrawCells {
        match self {
            MaybeUninitGameView::Uninit { .. } => RedrawCells::default(),
            MaybeUninitGameView::GameView(view) => view.right_click(x, y),
        }
    }

    fn middle_click(&mut self, x: usize, y: usize) -> RedrawCells {
        match self {
            MaybeUninitGameView::Uninit { .. } => RedrawCells::default(),
            MaybeUninitGameView::GameView(view) => view.middle_click(x, y),
        }
    }

    fn gesture(&mut self, gesture: Gesture) -> RedrawCells {
        match self {
            MaybeUninitGameView::Uninit {
                gesture: previous_gesture,
                options,
            } => {
                let mut redraw = Vec::new();
                match previous_gesture {
                    Gesture::Hover(x, y) | Gesture::LeftOrRightPush(x, y) => redraw.push((*x, *y)),
                    Gesture::MidPush(x, y) => {
                        let x = *x as i32;
                        let y = *y as i32;
                        for y1 in [y - 1, y, y + 1] {
                            if y1 < 0 || y1 >= options.difficulty.height() as i32 {
                                continue;
                            }
                            for x1 in [x - 1, x, x + 1] {
                                if x1 < 0 || x1 >= options.difficulty.width() as i32 {
                                    continue;
                                }
                                redraw.push((x1 as usize, y1 as usize));
                            }
                        }
                    }
                    Gesture::None => Default::default(),
                }
                match gesture {
                    Gesture::Hover(x, y) | Gesture::LeftOrRightPush(x, y) => redraw.push((x, y)),
                    Gesture::MidPush(x, y) => {
                        let x = x as i32;
                        let y = y as i32;
                        for y1 in [y - 1, y, y + 1] {
                            if y1 < 0 || y1 >= options.difficulty.height() as i32 {
                                continue;
                            }
                            for x1 in [x - 1, x, x + 1] {
                                if x1 < 0 || x1 >= options.difficulty.width() as i32 {
                                    continue;
                                }
                                redraw.push((x1 as usize, y1 as usize));
                            }
                        }
                    }
                    Gesture::None => Default::default(),
                }
                *self = MaybeUninitGameView::Uninit {
                    gesture,
                    options: options.clone(),
                };
                RedrawCells(redraw)
            }
            MaybeUninitGameView::GameView(view) => view.gesture(gesture),
        }
    }

    fn is_draggable(&self, x: usize, y: usize) -> bool {
        match self {
            MaybeUninitGameView::Uninit { .. } => false,
            MaybeUninitGameView::GameView(view) => view.is_draggable(x, y),
        }
    }

    fn is_playing(&self) -> bool {
        match self {
            MaybeUninitGameView::Uninit { .. } => false,
            MaybeUninitGameView::GameView(view) => view.result == GameResult::Playing,
        }
    }
}

impl From<GameOptions> for MaybeUninitGameView {
    fn from(value: GameOptions) -> Self {
        MaybeUninitGameView::Uninit {
            gesture: Gesture::None,
            options: value,
        }
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    let view = create_rw_signal(MaybeUninitGameView::Uninit {
        gesture: Gesture::None,
        options: GameOptions {
            difficulty: Difficulty::Easy,
            safe_pos: None,
            seed: Some(1),
        },
    });
    let redraw: RwSignal<RedrawCells> = create_rw_signal(Default::default());
    let (get_new_game, new_game) = create_signal(GameOptions::default());
    let restart = create_trigger();
    create_effect(move |_| {
        update!(|view| *view = get_new_game().into());
        let (w, h) = view.with_untracked(|view| (view.width(), view.height()));
        update!(|redraw| *redraw = RedrawCells::redraw_all(w, h));
    });
    create_effect(move |_| {
        restart.track();
        update!(|view| view.restart());
        let (w, h) = view.with_untracked(|view| (view.width(), view.height()));
        update!(|redraw| *redraw = RedrawCells::redraw_all(w, h));
    });
    let (_class_name, style_val) = style_str! {};
    view! {
        class = class_name,
        <Style> { style_val } </Style>
        <Map view redraw />
        <Controls view redraw new_game restart />
    }
}
