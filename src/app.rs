use leptos::*;
use leptos_meta::*;
use stylers::style_str;
use wasm_bindgen::prelude::*;
use web_sys::HtmlDivElement;

#[component]
pub fn Map() -> impl IntoView {
    view! {
        <canvas />
    }
}

#[component]
pub fn Controls() -> impl IntoView {
    let div_ref = create_node_ref();
    let (mouse_down, set_mouse_down) = create_signal(false);
    let (previous_pos_x, set_previous_pos_x) = create_signal(0);
    let (previous_pos_y, set_previous_pos_y) = create_signal(0);
    let closure: Box<dyn FnMut(_)> = Box::new(move |ev: web_sys::MouseEvent| {
        if !mouse_down.get_untracked() {
            return;
        }
        let offset_x = ev.client_x() - previous_pos_x.get_untracked();
        let offset_y = ev.client_y() - previous_pos_y.get_untracked();
        set_previous_pos_x.set_untracked(ev.client_x());
        set_previous_pos_y.set_untracked(ev.client_y());
        let element: &HtmlDivElement = &*div_ref.get_untracked().unwrap();
        element
            .style()
            .set_property("left", &format!("{}px", element.offset_left() + offset_x))
            .unwrap();
        element
            .style()
            .set_property("top", &format!("{}px", element.offset_top() + offset_y))
            .unwrap();
    });
    let closure = Closure::wrap(closure);
    document().add_event_listener_with_callback("mousemove", closure.into_js_value().unchecked_ref()).unwrap();
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
        <div ref=div_ref on:mousedown=move |ev| {
            set_mouse_down(true);
            set_previous_pos_x(ev.client_x());
            set_previous_pos_y(ev.client_y());
        } on:mouseup=move |_ev| {
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
    let (class_name, style_val) = style_str! {};
    view! {
        class = class_name,
        <Style> { style_val } </Style>
        <Map />
        <Controls />
    }
}
