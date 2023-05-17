use std::collections::HashMap;

use super::{Item, ItemStack, Program, Recipe};

#[derive(Debug, Default)]
struct Context {
    items_available: HashMap<Item, u64>,
    items_needed: HashMap<Item, u64>,

    recipes: HashMap<Item, Recipe>,
}

impl Context {
    pub fn new(program: &Program) -> Self {
        let mut ctx: Self = Default::default();

        for have in &program.have_section.0 {
            ctx.items_available.insert(have.item.clone(), have.count);
        }

        for recipe in &program.recipe_section.0 {
            let already_existed = ctx
                .recipes
                .insert(recipe.output.item.clone(), recipe.clone());

            if already_existed.is_some() {
                log::error!("tried to add recipe for {:?} but there already was one. old one gets overwritten.", recipe.output);
            }
        }

        ctx
    }

    fn add_items(&mut self, items: &ItemStack) {
        *self.items_available.entry(items.item.clone()).or_default() += items.count;
    }

    fn try_use_item(&mut self, items: &ItemStack) -> u64 {
        if let Some(stored_count) = self.items_available.get_mut(&items.item) {
            let old_count = *stored_count;
            *stored_count = old_count.saturating_sub(items.count);
            let used = old_count - *stored_count;
            items.count - used
        } else {
            items.count
        }
    }

    fn evaluate_recipe(&mut self, recipe: &Recipe) {
        log::debug!("Evaluating recipe {recipe:?}");
        for input in &recipe.inputs {
            // consume the existing items we have
            let count_needed = self.try_use_item(input);

            if count_needed == 0 {
                log::debug!("Satisfied requirement for {input:?} using available items");
            } else {
                log::debug!("Missing {count_needed} {} for {input:?}", &input.item.0);
                if let Some(recipe_to_use) = self.recipes.get(&input.item) {
                    log::debug!("Found recipe {recipe_to_use:?}");
                    let recipe = recipe_to_use.clone(); // TODO: annoying lifetime hack

                    // TODO: this should be a while, until we have enough items
                    self.evaluate_recipe(&recipe);
                } else {
                    log::info!(
                        "No recipe found for {:?}, registering {count_needed} items as 'needed'",
                        input.item
                    );
                    self.register_need_item(ItemStack {
                        count: count_needed,
                        item: input.item.clone(),
                    });
                }
            }
        }

        self.add_items(&recipe.output);
    }

    fn register_need_item(&mut self, item: ItemStack) {
        let count_remaining = self.try_use_item(&item);

        *self.items_needed.entry(item.item.clone()).or_default() += count_remaining;
    }
}

pub fn calculate_stuff(program: &Program) -> Vec<ItemStack> {
    let mut ctx = Context::new(program);

    for need in &program.need_section.0 {
        let recipe = ctx.recipes.get(&need.item).cloned();
        if let Some(recipe) = recipe {
            ctx.evaluate_recipe(&recipe);
        }
    }
    log::debug!("context: {ctx:#?}");

    ctx.items_needed.retain(|_, v| *v != 0);

    let mut items = ctx
        .items_needed
        .into_iter()
        .map(|(item, count)| ItemStack { item, count })
        .collect::<Vec<_>>();

    items.sort_by(|a, b| a.item.0.cmp(&b.item.0));

    items
}
