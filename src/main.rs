mod app;
mod logic;

use app::App;

fn main() {
    yew::Renderer::<App>::new().render();
}
