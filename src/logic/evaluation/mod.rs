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
    /// Items that are required to craft the item but are missing
    items_missing: HashMap<Item, u64>,

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

    fn create_items(&mut self, item_needed: &ItemStack) -> Result<(), EvaluationError> {
        let mut item_count_needed = item_needed.count;
        log::debug!("Need {item_count_needed} of {}", &item_needed.item.0);

        // try to take items from our existing stash
        {
            let count_available = self
                .items_available
                .entry(item_needed.item.clone())
                .or_default();
            let count_available_to_use = item_count_needed.min(*count_available);
            log::debug!(
                "{count_available} of {} is already available, will use {count_available_to_use}",
                &item_needed.item.0
            );
            *count_available -= count_available_to_use;
            item_count_needed -= count_available_to_use;
        }

        // early return if we already have everything
        if item_count_needed == 0 {
            return Ok(());
        }

        // find a recipe to craft the remaining items needed
        // this currently only supports recipes that return 1 item kind
        let Some(recipe) = self.recipes.get(&item_needed.item).cloned() else {
            // if no recipe is found, add these items to the missing items pile
            log::info!("Could not find recipe to create {}, adding it to items required", item_needed.item.0);
            *self.items_missing.entry(item_needed.item.clone()).or_default() += item_count_needed;

            return Ok(());
        };

        // we have a known recipe, now execute it until we have all the items we need
        // this is suboptimal, the loop will be executed many times for a large amount of items
        let mut item_count_created = 0;
        while item_count_created < item_count_needed {
            self.depth += 1;

            if self.depth > Self::MAX_DEPTH {
                return Err(EvaluationError::MaxDepthExceeded);
            }

            for input in &recipe.inputs {
                self.create_items(input)?;
            }

            // TODO: not actually creating the result?

            self.depth -= 1;
            item_count_created += recipe.output.count;
        }

        let items_created_too_many = item_count_created - item_count_needed;

        *self
            .items_available
            .entry(item_needed.item.clone())
            .or_default() += items_created_too_many;

        Ok(())
    }

    fn cleanup(&mut self) {
        self.items_available.retain(|_, v| *v != 0);
        self.items_missing.retain(|_, v| *v != 0);
    }

    pub fn get_missing_items(&self) -> Vec<ItemStack> {
        self.items_missing
            .iter()
            .map(|(item, count)| ItemStack {
                item: item.clone(),
                count: *count,
            })
            .collect()
    }

    pub fn get_available_items(&self) -> Vec<ItemStack> {
        self.items_available
            .iter()
            .map(|(item, count)| ItemStack {
                item: item.clone(),
                count: *count,
            })
            .collect()
    }
}

/// Calculate the crafting path for the current program.
pub fn evaluate(program: &Program) -> Result<Context, EvaluationError> {
    let mut ctx = Context::new(program);

    for need in &program.need_section.0 {
        ctx.create_items(need)?;
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
        assert_eq!(context.get_missing_items(), vec![]);
        assert_eq!(context.get_available_items(), vec![]);
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
            context.get_missing_items(),
            vec![ItemStack {
                count: 1,
                item: Item("input".into()),
            }]
        );
        assert_eq!(context.get_available_items(), vec![]);
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
        assert_eq!(context.get_missing_items(), vec![]);
        assert_eq!(context.get_available_items(), vec![]);
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
            context.get_missing_items(),
            vec![ItemStack {
                count: 1,
                item: Item("input".into()),
            }]
        );
        assert_eq!(context.get_available_items(), vec![]);
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
            context.get_missing_items(),
            vec![ItemStack {
                count: 10,
                item: Item("input".into()),
            }]
        );
        assert_eq!(context.get_available_items(), vec![]);
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
            context.get_missing_items(),
            vec![ItemStack {
                count: 2,
                item: Item("input".into()),
            }]
        );
        assert_eq!(context.get_available_items(), vec![]);
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
            context.get_missing_items(),
            vec![ItemStack {
                count: 2,
                item: Item("input".into()),
            }]
        );
        assert_eq!(context.get_available_items(), vec![]);
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
        assert_eq!(context.get_missing_items(), vec![]);
        assert_eq!(context.get_available_items(), vec![]);
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
