mod parsing;

use log::{debug, error};

#[derive(Debug)]
pub struct Program {
    pub need_section: NeedSection,
    pub have_section: HaveSection,
    pub recipe_section: RecipeSection,
}

impl Program {
    pub fn parse_from_string(input: &str) -> Result<Self, String> {
        debug!("Parsing input with length {}", input.len());
        match parsing::program(input) {
            Ok(("", output)) => {
                info!("Parsed input");
                Ok(output)
            }
            Ok((remaining, _)) => {
                error!("Parsed input but {} chars were remaining", remaining.len());
                Err(format!("Remaining: {remaining}"))
            }
            Err(e) => {
                error!("Error while parsing input: {}", e);
                Err(format!("{}", e))
            }
        }
    }
}

#[derive(Debug)]
pub struct NeedSection(Vec<ItemWithCount>);

#[derive(Debug)]
pub struct HaveSection(Vec<ItemWithCount>);

#[derive(Debug)]
pub struct RecipeSection(Vec<Recipe>);

#[derive(Debug, PartialEq, Eq)]
pub struct Recipe(ItemWithCount, Vec<ItemWithCount>);

#[derive(Debug, PartialEq, Eq)]
pub struct ItemWithCount(u64, Item);

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Item(String);

impl Item {
    fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}
