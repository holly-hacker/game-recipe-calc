use web_sys::HtmlTextAreaElement;
use yew::prelude::*;

#[function_component(App)]
pub fn app() -> Html {
    let textarea_ref = use_node_ref();
    let text = use_state(|| String::from("Initial text"));

    // onkeypress would be delayed by 1 character
    let onkeyup = {
        let text = text.clone();
        let textarea_ref = textarea_ref.clone();
        move |_| {
            let new_value = textarea_ref.cast::<HtmlTextAreaElement>().unwrap().value();
            let new_value = transform_text(&new_value);
            text.set(new_value);
        }
    };

    html! {
        <div>
            <textarea ref={textarea_ref} {onkeyup} />
            <pre>{ format!("{}", *text) }</pre>
        </div>
    }
}

fn transform_text(text: &str) -> String {
    match super::logic::Program::parse_from_string(text) {
        Ok(v) => format!("{v:#?}"),
        Err(e) => format!("Error: {e}"),
    }
}
