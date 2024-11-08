use automation_worker::Automation;
use gloo_worker::Registrable;

fn main() {
    console_error_panic_hook::set_once();
    Automation::registrar().register();
}
