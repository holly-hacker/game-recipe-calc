mod evaluation;
mod parsing;

use std::fmt::Display;

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
        let context = evaluation::evaluate(self);

        let context = match context {
            Ok(c) => c,
            Err(e) => return format!("Error during evaluation: {e:?}"),
        };

        let mut result = String::new();

        // TODO: show which items will actually be used?

        let missing_items = context.get_missing_items();
        if missing_items.is_empty() {
            result.push_str("You have all the required items!\n");
        } else {
            result.push_str("Missing items:\n");
            for stack in missing_items {
                result.push_str(&format!("- {} {}\n", stack.count, stack.item.0));
            }
        }
        result.push('\n');

        let leftover_items = context.get_available_items();
        if leftover_items.is_empty() {
            result.push_str("No items are left over after crafting.\n");
        } else {
            result.push_str("Leftover items after crafting:\n");
            for stack in leftover_items {
                result.push_str(&format!("- {} {}\n", stack.count, stack.item.0));
            }
        }
        result.push('\n');

        result.push_str("Executed recipes:\n");
        for recipe in context.get_executed_recipes() {
            result.push_str(&format!("- {recipe}\n"));
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

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Recipe {
    pub output: ItemStack,
    pub inputs: Vec<ItemStack>,
}

impl Recipe {
    pub fn multiplied_by(&self, count: u64) -> Self {
        let mut cloned = self.clone();
        cloned.multiply(count);
        cloned
    }

    pub fn multiply(&mut self, count: u64) {
        self.output.count *= count;

        for input in &mut self.inputs {
            input.count *= count;
        }
    }
}

impl Display for Recipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut is_first = true;
        for input in &self.inputs {
            if !is_first {
                write!(f, "+ ")?;
            }

            write!(f, "{} {} ", input.count, &input.item.0)?;

            is_first = false;
        }

        write!(f, "-> {} {}", self.output.count, &self.output.item.0)?;

        Ok(())
    }
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
