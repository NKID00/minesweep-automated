use futures::{SinkExt, StreamExt};
use gloo_worker::reactor::{ReactorScope, reactor};
use js_sys::global;
use minesweep_core::{GameView, RedrawCells, SatSolver};
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
pub async fn Automation(
    mut scope: ReactorScope<(GameView, SatSolver), (f64, GameView, Option<RedrawCells>)>,
) {
    while let Some((mut view, solver)) = scope.next().await {
        let begin = timestamp();
        let redraw = view.automation_step(solver);
        let result = (timestamp() - begin, view, redraw);
        if scope.send(result).await.is_err() {
            break;
        }
    }
}
