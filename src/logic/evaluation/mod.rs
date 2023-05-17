use std::collections::HashMap;

use super::{Item, ItemStack, Program, Recipe};

#[derive(Debug, PartialEq, Eq)]
pub enum EvaluationError {
    MaxDepthExceeded,
}

#[derive(Debug, Default)]
pub struct Context {
    /// Items that can be used for crafting
    items_available: HashMap<Item, u64>,
    /// Items that were requested and have been crafted. These items cannot be used for crafting.
    items_output: HashMap<Item, u64>,
    /// Items that we are missing, but are required to craft the requested items.
    items_needed: HashMap<Item, u64>,

    /// A map with a recipe for each item we can craft.
    recipes: HashMap<Item, Recipe>,

    /// The current recursion depth. Limited to [Context::MAX_DEPTH].
    depth: usize,
}

impl Context {
    pub const MAX_DEPTH: usize = 128;

    /// Create a new context for a given program
    pub fn new(program: &Program) -> Self {
        let mut ctx: Self = Default::default();

        for have in &program.have_section.0 {
            *ctx.items_available.entry(have.item.clone()).or_default() += have.count;
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

    fn have_or_craft_item(
        &mut self,
        item_needed: &ItemStack,
        top_level: bool,
    ) -> Result<(), EvaluationError> {
        // consume the existing items we have
        let count_needed = self.try_use_item(item_needed);

        if count_needed == 0 {
            log::debug!("Satisfied requirement for {item_needed:?} using available items");
        } else {
            log::debug!(
                "Missing {count_needed} {} for {item_needed:?}",
                &item_needed.item.0
            );
            if let Some(recipe_to_use) = self.recipes.get(&item_needed.item) {
                log::debug!("Found recipe {recipe_to_use:?}");
                let recipe = recipe_to_use.clone(); // TODO: annoying lifetime hack

                while self
                    .items_available
                    .get(&item_needed.item)
                    .cloned()
                    .unwrap_or_default()
                    < count_needed
                {
                    self.evaluate_recipe(&recipe)?;
                }
            } else {
                log::info!(
                    "No recipe found for {:?}, registering {count_needed} items as 'needed'",
                    item_needed.item
                );
                self.register_need_item(ItemStack {
                    count: count_needed,
                    item: item_needed.item.clone(),
                });
            }
        }

        if top_level {
            // transfer this item to the output
            *self
                .items_available
                .entry(item_needed.item.clone())
                .or_default() -= item_needed.count;
            *self
                .items_output
                .entry(item_needed.item.clone())
                .or_default() += item_needed.count;
        }

        Ok(())
    }

    fn evaluate_recipe(&mut self, recipe: &Recipe) -> Result<(), EvaluationError> {
        // ensure we don't enter an infinite loop
        if self.depth > Self::MAX_DEPTH {
            return Err(EvaluationError::MaxDepthExceeded);
        }
        self.depth += 1;

        log::debug!("Evaluating recipe {recipe:?}");
        for input in &recipe.inputs {
            self.have_or_craft_item(input, false)?;
        }

        self.add_items(&recipe.output);

        self.depth -= 1;

        Ok(())
    }

    fn register_need_item(&mut self, item: ItemStack) {
        let count_remaining = self.try_use_item(&item);

        *self.items_needed.entry(item.item.clone()).or_default() += count_remaining;
    }

    /// Get the items that we need to craft the items currently in the context.
    pub fn get_needed_items(&self) -> Vec<ItemStack> {
        let mut items = self
            .items_needed
            .iter()
            .map(|(item, count)| ItemStack {
                item: item.clone(),
                count: *count,
            })
            .collect::<Vec<_>>();

        items.sort_by(|a, b| a.item.0.cmp(&b.item.0));

        items
    }

    fn cleanup(&mut self) {
        self.items_needed.retain(|_, v| *v != 0);
        self.items_available.retain(|_, v| *v != 0);

        debug_assert!(!self.items_output.iter().any(|(_, v)| *v == 0));
    }
}

/// Calculate the crafting path for the current program.
pub fn evaluate(program: &Program) -> Result<Context, EvaluationError> {
    let mut ctx = Context::new(program);

    for need in &program.need_section.0 {
        ctx.have_or_craft_item(need, true)?;
    }
    ctx.cleanup();
    log::debug!("context after calculations: {ctx:#?}");

    Ok(ctx)
}

#[cfg(test)]
mod tests {
    use crate::logic::{evaluation::EvaluationError, *};

    use super::evaluate;

    #[test]
    fn test_single_recipe_has_everything() {
        let program = Program {
            need_section: NeedSection(vec![ItemStack {
                count: 1,
                item: Item("output".into()),
            }]),
            have_section: HaveSection(vec![ItemStack {
                count: 1,
                item: Item("input".into()),
            }]),
            recipe_section: RecipeSection(vec![Recipe {
                output: ItemStack {
                    count: 1,
                    item: Item("output".into()),
                },
                inputs: vec![ItemStack {
                    count: 1,
                    item: Item("input".into()),
                }],
            }]),
        };

        let context = evaluate(&program).unwrap();
        assert_eq!(context.get_needed_items(), vec![]);
    }

    #[test]
    fn test_single_recipe_has_nothing() {
        let program = Program {
            need_section: NeedSection(vec![ItemStack {
                count: 1,
                item: Item("output".into()),
            }]),
            have_section: HaveSection(vec![]),
            recipe_section: RecipeSection(vec![Recipe {
                output: ItemStack {
                    count: 1,
                    item: Item("output".into()),
                },
                inputs: vec![ItemStack {
                    count: 1,
                    item: Item("input".into()),
                }],
            }]),
        };

        let context = evaluate(&program).unwrap();
        assert_eq!(
            context.get_needed_items(),
            vec![ItemStack {
                count: 1,
                item: Item("input".into()),
            }]
        );
    }

    #[test]
    fn test_double_recipe_has_everything() {
        let program = Program {
            need_section: NeedSection(vec![ItemStack {
                count: 1,
                item: Item("output".into()),
            }]),
            have_section: HaveSection(vec![ItemStack {
                count: 1,
                item: Item("input".into()),
            }]),
            recipe_section: RecipeSection(vec![
                Recipe {
                    output: ItemStack {
                        count: 1,
                        item: Item("output".into()),
                    },
                    inputs: vec![ItemStack {
                        count: 1,
                        item: Item("middle".into()),
                    }],
                },
                Recipe {
                    output: ItemStack {
                        count: 1,
                        item: Item("middle".into()),
                    },
                    inputs: vec![ItemStack {
                        count: 1,
                        item: Item("input".into()),
                    }],
                },
            ]),
        };

        let context = evaluate(&program).unwrap();
        assert_eq!(context.get_needed_items(), vec![]);
    }

    #[test]
    fn test_double_recipe_has_nothing() {
        let program = Program {
            need_section: NeedSection(vec![ItemStack {
                count: 1,
                item: Item("output".into()),
            }]),
            have_section: HaveSection(vec![]),
            recipe_section: RecipeSection(vec![
                Recipe {
                    output: ItemStack {
                        count: 1,
                        item: Item("output".into()),
                    },
                    inputs: vec![ItemStack {
                        count: 1,
                        item: Item("middle".into()),
                    }],
                },
                Recipe {
                    output: ItemStack {
                        count: 1,
                        item: Item("middle".into()),
                    },
                    inputs: vec![ItemStack {
                        count: 1,
                        item: Item("input".into()),
                    }],
                },
            ]),
        };

        let context = evaluate(&program).unwrap();
        assert_eq!(
            context.get_needed_items(),
            vec![ItemStack {
                count: 1,
                item: Item("input".into()),
            }]
        );
    }

    #[test]
    fn test_run_recipe_multiple_times() {
        let program = Program {
            need_section: NeedSection(vec![ItemStack {
                count: 10,
                item: Item("output".into()),
            }]),
            have_section: HaveSection(vec![]),
            recipe_section: RecipeSection(vec![Recipe {
                output: ItemStack {
                    count: 1,
                    item: Item("output".into()),
                },
                inputs: vec![ItemStack {
                    count: 1,
                    item: Item("input".into()),
                }],
            }]),
        };

        let context = evaluate(&program).unwrap();
        assert_eq!(
            context.get_needed_items(),
            vec![ItemStack {
                count: 10,
                item: Item("input".into()),
            }]
        );
    }

    #[test]
    fn test_need_items_dont_get_used_by_other_need_items() {
        let program = Program {
            need_section: NeedSection(vec![
                ItemStack {
                    count: 1,
                    item: Item("middle".into()),
                },
                ItemStack {
                    count: 1,
                    item: Item("output".into()),
                },
            ]),
            have_section: HaveSection(vec![]),
            recipe_section: RecipeSection(vec![
                Recipe {
                    output: ItemStack {
                        count: 1,
                        item: Item("output".into()),
                    },
                    inputs: vec![ItemStack {
                        count: 1,
                        item: Item("middle".into()),
                    }],
                },
                Recipe {
                    output: ItemStack {
                        count: 1,
                        item: Item("middle".into()),
                    },
                    inputs: vec![ItemStack {
                        count: 1,
                        item: Item("input".into()),
                    }],
                },
            ]),
        };

        let context = evaluate(&program).unwrap();
        assert_eq!(
            context.get_needed_items(),
            vec![ItemStack {
                count: 2,
                item: Item("input".into()),
            }]
        );
    }

    #[test]
    fn test_can_have_duplicate_need_items() {
        let program = Program {
            need_section: NeedSection(vec![
                ItemStack {
                    count: 1,
                    item: Item("output".into()),
                },
                ItemStack {
                    count: 1,
                    item: Item("output".into()),
                },
            ]),
            have_section: HaveSection(vec![]),
            recipe_section: RecipeSection(vec![Recipe {
                output: ItemStack {
                    count: 1,
                    item: Item("output".into()),
                },
                inputs: vec![ItemStack {
                    count: 1,
                    item: Item("input".into()),
                }],
            }]),
        };

        let context = evaluate(&program).unwrap();
        assert_eq!(
            context.get_needed_items(),
            vec![ItemStack {
                count: 2,
                item: Item("input".into()),
            }]
        );
    }

    #[test]
    fn test_can_have_duplicate_have_items() {
        let program = Program {
            need_section: NeedSection(vec![ItemStack {
                count: 1,
                item: Item("output".into()),
            }]),
            have_section: HaveSection(vec![
                ItemStack {
                    count: 1,
                    item: Item("input".into()),
                },
                ItemStack {
                    count: 1,
                    item: Item("input".into()),
                },
            ]),
            recipe_section: RecipeSection(vec![Recipe {
                output: ItemStack {
                    count: 1,
                    item: Item("output".into()),
                },
                inputs: vec![ItemStack {
                    count: 2,
                    item: Item("input".into()),
                }],
            }]),
        };

        let context = evaluate(&program).unwrap();
        assert_eq!(context.get_needed_items(), vec![]);
    }

    #[test]
    #[ntest::timeout(100)]
    fn test_prevent_infinite_loop() {
        let program = Program {
            need_section: NeedSection(vec![ItemStack {
                count: 1,
                item: Item("item".into()),
            }]),
            have_section: HaveSection(vec![]),
            recipe_section: RecipeSection(vec![Recipe {
                output: ItemStack {
                    count: 1,
                    item: Item("item".into()),
                },
                inputs: vec![ItemStack {
                    count: 1,
                    item: Item("item".into()),
                }],
            }]),
        };

        let result = evaluate(&program);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), EvaluationError::MaxDepthExceeded);
    }
}
