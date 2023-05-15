use monaco::{
    api::TextModel,
    sys::editor::{
        IDimension, IEditorMinimapOptions, IModelContentChangedEvent,
        IStandaloneEditorConstructionOptions,
    },
    yew::CodeEditor,
};
use yew::prelude::*;

#[function_component(App)]
pub fn app() -> Html {
    let text = use_state(|| String::from(include_str!("logic/parsing/example_input.txt")));

    let text_model = use_state_eq(|| {
        let model = TextModel::create(&text, None, None).unwrap();

        let text = text.clone();

        let model_clone = model.clone();
        let closure = model.on_did_change_content(move |_: IModelContentChangedEvent| {
            text.set(model_clone.get_value());
        });

        // TODO: I can't figure out how to keep it in memory otherwise
        // perhaps see https://github.com/siku2/rust-monaco/issues/19
        Box::leak(Box::new(closure));

        model
    });

    let options = use_state(|| {
        let minimap_options = IEditorMinimapOptions::default();
        minimap_options.set_enabled(Some(false));

        let dimensions = IDimension::new(512, 384);

        let options = IStandaloneEditorConstructionOptions::default();

        options.set_dimension(Some(&dimensions));
        options.set_folding(Some(false));
        options.set_line_numbers_min_chars(Some(3.));
        options.set_minimap(Some(&minimap_options));
        options.set_scroll_beyond_last_line(Some(false));
        options.set_theme(Some("vs-dark"));
        // options.set_value(Some(include_str!("logic/parsing/example_input.txt")));
        options
    });

    html! {
        <div style={"padding: 1rem"}>
            <CodeEditor options={(*options).clone()} model={(*text_model).clone()} />
            <pre>{ transform_text(&text) }</pre>
        </div>
    }
}

fn transform_text(text: &str) -> String {
    match super::logic::Program::parse_from_string(text) {
        Ok(v) => format!("{v:#?}"),
        Err(e) => format!("Error: {e}"),
    }
}
