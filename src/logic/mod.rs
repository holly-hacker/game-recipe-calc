mod evaluation;
mod parsing;

use log::{debug, error, info};

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

    pub fn evaluate(&self) -> String {
        let needed = evaluation::calculate_stuff(self);

        let mut result = String::new();

        for stack in needed {
            result.push_str(&format!("- {} {}\n", stack.count, stack.item.0));
        }

        result
    }
}

#[derive(Debug)]
pub struct NeedSection(Vec<ItemStack>);

#[derive(Debug)]
pub struct HaveSection(Vec<ItemStack>);

#[derive(Debug)]
pub struct RecipeSection(Vec<Recipe>);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Recipe {
    pub output: ItemStack,
    pub inputs: Vec<ItemStack>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ItemStack {
    count: u64,
    item: Item,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Item(String);

impl Item {
    fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}
