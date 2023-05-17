use nom::{
    branch::{alt, permutation},
    bytes::complete::{is_not, tag, take_while1},
    character::complete::{char, line_ending, multispace0, space0},
    combinator::eof,
    multi::{many0, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple, Tuple},
    IResult, Parser,
};

use super::{HaveSection, Item, ItemStack, NeedSection, Program, Recipe, RecipeSection};

/// Parses a full program.
pub fn program(input: &str) -> IResult<&str, Program> {
    let need_section = section("need", item_with_count);
    let have_section = section("have", item_with_count);
    let recipe_section = section("recipes", recipe);

    terminated(
        permutation((
            preceded(multispace0, need_section),
            preceded(multispace0, have_section),
            preceded(multispace0, recipe_section),
        )),
        multispace0,
    )
    .map(|(n, h, r)| Program {
        need_section: NeedSection(n),
        have_section: HaveSection(h),
        recipe_section: RecipeSection(r),
    })
    .parse(input)
}

/// Parses a headered section, such as `section:\n-test1\ntest2\n`.
fn section<'i, O, F>(head: &'i str, body: F) -> impl FnMut(&'i str) -> IResult<&'i str, Vec<O>>
where
    F: Parser<&'i str, O, nom::error::Error<&'i str>>,
{
    preceded(
        tuple((tag(head), char(':'), fuzzy_line_ending)),
        many0(list_item(body)),
    )
}

/// An object inside a line, such as `- wooper!\n` (where `wooper!` is matched).
fn list_item<'i, O, F>(f: F) -> impl FnMut(&'i str) -> IResult<&'i str, O>
where
    F: Parser<&'i str, O, nom::error::Error<&'i str>>,
{
    delimited(pair(char('-'), space0), f, alt((fuzzy_line_ending, eof)))
}

/// A recipe, such as `1 diamond shovel = 2 stick + 1 diamond`.
fn recipe(input: &str) -> IResult<&str, Recipe> {
    let equal = delimited(space0, char('='), space0);
    let plus = delimited(space0, char('+'), space0);

    let mut recipe = separated_pair(
        item_with_count,
        equal,
        separated_list1(plus, item_with_count),
    );

    let (input, (output, inputs)) = recipe.parse(input)?;

    Ok((input, Recipe { output, inputs }))
}

/// An item with a count, such as `1 wood` or `10 diamond shovel`.
fn item_with_count(input: &str) -> IResult<&str, ItemStack> {
    let count = nom::character::complete::u64;
    let space = take_while1(|c| c == ' ');

    let (input, (count, _, item)) = (count, space, item).parse(input)?;

    Ok((input, ItemStack { count, item }))
}

/// An item name, such as `wood` or `diamond shovel`.
fn item(input: &str) -> IResult<&str, Item> {
    is_not("+=\r\n")
        .map(|item: &str| item.trim()) // this trim is somewhat hacky
        .map(Item::new)
        .parse(input)
}

/// A line ending that may be preceeded by spaces.
///
/// This type does not return the entire matched `&str` because it's
/// impractical to do so. It would either require stitching 2 separate `&str`s
/// together or custom matching logic.
fn fuzzy_line_ending(input: &str) -> IResult<&str, &str> {
    preceded(space0, line_ending).parse(input)
}

#[cfg(test)]
mod tests {
    use nom::character::complete::{alpha1, alphanumeric1};

    use crate::logic::{parsing::*, Item, ItemStack, Recipe};

    #[test]
    fn smoke_test_example_input() {
        let input = include_str!("example_input.txt");

        let (remaining, program) = program(input).unwrap();
        assert_eq!(remaining, "");
        assert_eq!(program.need_section.0.len(), 1);
        assert_eq!(program.have_section.0.len(), 2);
        assert_eq!(program.recipe_section.0.len(), 3);
    }

    #[test]
    fn test_section() {
        assert_eq!(
            section("section", alphanumeric1).parse("section:\n-line1\n-line2\n"),
            Ok(("", vec!["line1", "line2"]))
        );
        assert_eq!(
            section("section", alphanumeric1).parse("section:\n-line1\n-line2"),
            Ok(("", vec!["line1", "line2"]))
        );
        assert_eq!(
            section("section", alphanumeric1).parse("section:\n"),
            Ok(("", vec![]))
        );
    }

    #[test]
    fn test_line() {
        assert_eq!(list_item(tag("a")).parse("- a\n"), Ok(("", "a")));
        assert_eq!(list_item(tag("a")).parse("- a\r\n"), Ok(("", "a")));
        assert_eq!(list_item(tag("a")).parse("- a"), Ok(("", "a")));
        assert_eq!(list_item(tag("a")).parse("-a"), Ok(("", "a")));
        assert_eq!(list_item(tag("a")).parse("-a\n-b"), Ok(("-b", "a")));
    }

    #[test]
    fn test_recipe() {
        assert_eq!(
            recipe("1 output = 1 input"),
            Ok((
                "",
                Recipe {
                    output: ItemStack {
                        count: 1,
                        item: Item::new("output")
                    },
                    inputs: vec![ItemStack {
                        count: 1,
                        item: Item::new("input")
                    }]
                }
            ))
        );
        assert_eq!(
            recipe("1 output = 2 input1 + 1 input2"),
            Ok((
                "",
                Recipe {
                    output: ItemStack {
                        count: 1,
                        item: Item::new("output")
                    },
                    inputs: vec![
                        ItemStack {
                            count: 2,
                            item: Item::new("input1")
                        },
                        ItemStack {
                            count: 1,
                            item: Item::new("input2")
                        },
                    ]
                }
            ))
        );
        assert_eq!(
            recipe("1 output=2 input1+1 input2"),
            Ok((
                "",
                Recipe {
                    output: ItemStack {
                        count: 1,
                        item: Item::new("output")
                    },
                    inputs: vec![
                        ItemStack {
                            count: 2,
                            item: Item::new("input1")
                        },
                        ItemStack {
                            count: 1,
                            item: Item::new("input2")
                        },
                    ]
                }
            ))
        );
        assert_eq!(
            recipe("1 output thing = 1 input thing + 2 input thing"),
            Ok((
                "",
                Recipe {
                    output: ItemStack {
                        count: 1,
                        item: Item::new("output thing")
                    },
                    inputs: vec![
                        ItemStack {
                            count: 1,
                            item: Item::new("input thing")
                        },
                        ItemStack {
                            count: 2,
                            item: Item::new("input thing")
                        },
                    ]
                }
            ))
        );
    }

    #[test]
    fn test_item_with_count() {
        assert_eq!(
            item_with_count("1 item"),
            Ok((
                "",
                ItemStack {
                    count: 1,
                    item: Item::new("item")
                }
            ))
        );
        assert_eq!(
            item_with_count("1 item thing"),
            Ok((
                "",
                ItemStack {
                    count: 1,
                    item: Item::new("item thing")
                }
            ))
        );
    }

    #[test]
    fn test_fuzzy_line_ending() {
        assert_eq!(
            pair(alpha1, fuzzy_line_ending).parse("test\n"),
            Ok(("", ("test", "\n")))
        );
        assert_eq!(
            pair(alpha1, fuzzy_line_ending).parse("test \n"),
            Ok(("", ("test", "\n")))
        );
        assert_eq!(
            pair(alpha1, fuzzy_line_ending).parse("test \n "),
            Ok((" ", ("test", "\n")))
        );
    }
}
