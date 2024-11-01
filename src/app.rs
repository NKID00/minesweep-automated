use leptos::logging::log;
use leptos::*;
use leptos_meta::*;
use leptos_use::{use_mouse_in_element, UseMouseInElementReturn};
use stylers::style_str;
use web_sys::HtmlDivElement;

use minesweep_core::{GameOptions, GameView};

#[component]
pub fn Map(view: RwSignal<GameView>) -> impl IntoView {
    view! {
        <canvas />
    }
}

#[component]
pub fn Controls() -> impl IntoView {
    let div_ref = create_node_ref();
    let (mouse_down, set_mouse_down) = create_signal(false);
    let UseMouseInElementReturn {
        x: mouse_x,
        y: mouse_y,
        element_position_x,
        element_position_y,
        ..
    } = use_mouse_in_element(div_ref);
    create_effect(move |offset_when_mouse_down| {
        let Some(offset_when_mouse_down) = offset_when_mouse_down else {
            mouse_down.track();
            return None;
        };
        if !mouse_down() {
            return None;
        }
        match offset_when_mouse_down {
            Some((offset_x, offset_y)) => {
                let element: &HtmlDivElement = &*div_ref().unwrap();
                element
                    .style()
                    .set_property("left", &format!("{}px", mouse_x() - offset_x))
                    .unwrap();
                element
                    .style()
                    .set_property("top", &format!("{}px", mouse_y() - offset_y))
                    .unwrap();
                Some((offset_x, offset_y))
            }
            None => {
                // reached at first time after mouse down
                Some((
                    mouse_x() - element_position_x(),
                    mouse_y() - element_position_y(),
                ))
            }
        }
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
            padding: 2rem;
            border-radius: 0.5rem;
            border-width: 2px;
            border-color: rgba(118, 210, 255, 0.3);
            border-style: solid;
            box-shadow: 0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1);
            position: absolute;
            cursor: move;
            user-select: none;
            z-index: 10;
        }
    };
    view! {
        class = class_name,
        <Style> { style_val } </Style>
        <div ref=div_ref on:mousedown=move |_| {
            log!("mouse down");
            set_mouse_down(true);
        } on:mouseup=move |_| {
            log!("mouse up");
            set_mouse_down(false);
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
    let (class_name, style_val) = style_str! {};
    view! {
        class = class_name,
        <Style> { style_val } </Style>
        <Map view />
        <Controls />
    }
}
