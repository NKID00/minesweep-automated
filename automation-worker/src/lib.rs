use futures::{SinkExt, StreamExt};
use gloo_worker::reactor::{reactor, ReactorScope};
use js_sys::global;
use minesweep_core::{GameView, RedrawCells};
use wasm_bindgen::JsCast;
use web_sys::WorkerGlobalScope;

fn timestamp() -> f64 {
    global()
        .dyn_into::<WorkerGlobalScope>()
        .unwrap()
        .performance()
        .unwrap()
        .now() as f64
        / 1000.
}

#[reactor]
pub async fn Automation(mut scope: ReactorScope<GameView, (f64, GameView, Option<RedrawCells>)>) {
    while let Some(mut view) = scope.next().await {
        let begin = timestamp();
        let redraw = view.automation_step();
        let result = (timestamp() - begin, view, redraw);
        if scope.send(result).await.is_err() {
            break;
        }
    }
}
