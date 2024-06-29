use super::{VDLNode, VDLPipeline};
use anyhow::{bail, Context, Result};
use nom::{
	branch::alt,
	bytes::complete::{escaped_transform, tag, take_while, take_while1},
	character::complete::{alphanumeric1, char, multispace0, multispace1, none_of, one_of},
	combinator::{opt, recognize, value},
	error::{context, ContextError},
	multi::{many1, separated_list0},
	sequence::{delimited, pair, separated_pair, tuple},
	IResult, Parser,
};
use std::{collections::HashMap, fmt::Debug};

#[allow(dead_code)]
fn debug<I: Clone + Debug, E: ContextError<I>, F, O: Debug>(
	context: &'static str,
	mut f: F,
) -> impl FnMut(I) -> IResult<I, O, E>
where
	F: Parser<I, O, E>,
{
	move |i: I| {
		let result = f.parse(i.clone());
		println!("CONTEXT: {context}");
		println!("  INPUT: {i:?}");
		if let Ok(v) = &result {
			println!("  \x1b[0;32mRESULT: {v:?}\x1b[0m");
		} else {
			println!("  \x1b[0;31mERROR!!!\x1b[0m");
		}
		result
	}
}

fn parse_unquoted_value(input: &str) -> IResult<&str, String> {
	context(
		"parse_unquoted_value",
		recognize(many1(alt((alphanumeric1, recognize(one_of(".-_")))))),
	)(input)
	.map(|(a, b)| (a, b.to_string()))
}

fn parse_string(input: &str) -> IResult<&str, String> {
	context(
		"parse_string",
		escaped_transform(
			none_of("\\\""),
			'\\',
			alt((
				value("\\", tag("\\")),
				value("\"", tag("\"")),
				value("\n", tag("n")),
				value("\t", tag("t")),
			)),
		),
	)(input)
}

fn parse_bare_identifier(input: &str) -> IResult<&str, String> {
	fn is_initial_identifier_char(c: char) -> bool {
		!c.is_ascii_digit() && is_identifier_char(c)
	}

	fn is_identifier_char(c: char) -> bool {
		c.is_ascii_alphanumeric() || "_-".contains(c)
	}

	context(
		"parse_bare_identifier",
		recognize(pair(
			take_while1(|c: char| is_initial_identifier_char(c)),
			take_while(|c: char| is_identifier_char(c)),
		)),
	)(input)
	.map(|(a, b)| (a, b.to_string()))
}

fn parse_quoted_string(input: &str) -> IResult<&str, String> {
	context(
		"parse_quoted_string",
		delimited(char('\"'), parse_string, char('\"')),
	)(input)
}

fn parse_array(input: &str) -> IResult<&str, Vec<String>> {
	delimited(
		tuple((char('['), multispace0)),
		separated_list0(
			tuple((multispace0, char(','), multispace0)),
			alt((parse_quoted_string, parse_unquoted_value)),
		),
		tuple((multispace0, char(']'))),
	)(input)
}

fn parse_value(input: &str) -> IResult<&str, Vec<String>> {
	context(
		"parse_value",
		alt((
			parse_quoted_string.map(|v| vec![v]),
			parse_unquoted_value.map(|v| vec![v]),
			parse_array,
		)),
	)(input)
}

fn parse_prop(input: &str) -> IResult<&str, (String, Vec<String>)> {
	context(
		"parse_prop",
		separated_pair(
			parse_identifier,
			tuple((multispace0, char('='), multispace0)),
			parse_value,
		),
	)(input)
}

fn parse_identifier(input: &str) -> IResult<&str, String> {
	context(
		"parse_identifier",
		alt((parse_quoted_string, parse_bare_identifier)),
	)(input)
}

fn parse_children(input: &str) -> IResult<&str, Vec<VDLPipeline>> {
	context(
		"parse_children",
		opt(delimited(
			tuple((char('['), multispace0)),
			separated_list0(char(','), parse_pipeline),
			tuple((multispace0, char(']'))),
		))
		.map(|r| r.unwrap_or(vec![])),
	)(input)
}

fn parse_node<'a>(input: &'a str) -> IResult<&str, VDLNode> {
	context("parse_node", |input: &'a str| {
		let (input, _) = multispace0(input)?;
		let (input, name) = parse_identifier(input)?;
		let (input, _) = multispace0(input)?;
		let (input, property_list) = separated_list0(multispace1, parse_prop)(input)?;
		let (input, _) = multispace0(input)?;
		let (input, children) = parse_children(input)?;
		let (input, _) = multispace0(input)?;

		let mut properties = HashMap::new();
		for (key, mut values) in property_list {
			properties
				.entry(key)
				.and_modify(|list: &mut Vec<String>| list.append(&mut values))
				.or_insert(values);
		}

		Ok((
			input,
			VDLNode {
				name,
				properties,
				children,
			},
		))
	})(input)
}

fn parse_pipeline(input: &str) -> IResult<&str, VDLPipeline> {
	context(
		"parse_pipeline",
		delimited(
			multispace0,
			separated_list0(char('|'), parse_node).map(|pipeline| VDLPipeline { pipeline }),
			multispace0,
		),
	)(input)
}

pub fn parse_vdl(input: &str) -> Result<VDLPipeline> {
	match parse_pipeline(input) {
		Ok((leftover, nodes)) => {
			if leftover.trim().is_empty() {
				Ok(nodes)
			} else {
				bail!("VDL didn't parse till the end: '{leftover}'")
			}
		}
		Err(e) => {
			Err(anyhow::anyhow!("Error parsing VDL: {:?}", e)).context("Failed to parse VDL input")
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_bare_identifier() {
		for (input, output) in [
			("foo", "foo"),
			("foo123", "foo123"),
			("-foo", "-foo"),
			("foo-bar", "foo-bar"),
			("foo_bar", "foo_bar"),
			("foo!bar", "foo"),
		] {
			assert_eq!(parse_bare_identifier(input).unwrap().1, output)
		}

		for input in ["123foo", "=a"] {
			let r = parse_bare_identifier(input);
			assert!(r.is_err(), "input did not fail: {input}");
		}
	}

	#[test]
	fn test_parse_identifier() {
		assert_eq!(parse_identifier("foo"), Ok(("", "foo".to_string())));
		assert_eq!(parse_identifier("\"foo\""), Ok(("", "foo".to_string())));
		assert!(parse_identifier("123foo").is_err());
		assert!(parse_identifier("\"foo").is_err());
	}

	#[test]
	fn test_parse_quoted_string() {
		assert_eq!(parse_quoted_string("\"foo\""), Ok(("", "foo".to_string())));
		assert_eq!(
			parse_quoted_string("\"foo bar\""),
			Ok(("", "foo bar".to_string()))
		);
		assert_eq!(
			parse_quoted_string("\"foo\\\"bar\\\"\""),
			Ok(("", "foo\"bar\"".to_string()))
		);
		assert!(parse_quoted_string("\"foo").is_err());
		assert!(parse_quoted_string("foo\"").is_err());
	}

	#[test]
	fn test_parse_prop() {
		let check = |a, b: &str, c: &str| {
			assert_eq!(
				parse_prop(a),
				Ok(("", (b.to_string(), vec![c.to_string()]))),
				"error on: {a}"
			)
		};
		check("key=value", "key", "value");
		check("key=\"value\"", "key", "value");
		check("key=-2.0", "key", "-2.0");
	}

	#[test]
	fn test_parse_node() {
		let input = "node key1=value1 key2=\"value2\" key3=\"a=\\\"b\\\"\" [ child ]";
		let expected = VDLNode::from((
			"node",
			vec![("key1", "value1"), ("key2", "value2"), ("key3", "a=\"b\"")],
			VDLPipeline::from(VDLNode::from("child")),
		));
		assert_eq!(parse_node(input), Ok(("", expected)));
	}

	#[test]
	fn test_parse_nodes1() {
		let input = "node1 key1=value1|\nnode2 key2=\"value2\"";
		let expected = VDLPipeline::from(vec![
			VDLNode::from(("node1", ("key1", "value1"))),
			VDLNode::from(("node2", ("key2", "value2"))),
		]);
		assert_eq!(parse_vdl(input).unwrap(), expected);
	}

	#[test]
	fn test_parse_nodes2() {
		let input = "node1 key1=value1|node2 key2=\"value2\"| node3 key3=value3 |\nnode4 key4=value4";
		let expected = VDLPipeline::from(vec![
			VDLNode::from(("node1", ("key1", "value1"))),
			VDLNode::from(("node2", ("key2", "value2"))),
			VDLNode::from(("node3", ("key3", "value3"))),
			VDLNode::from(("node4", ("key4", "value4"))),
		]);
		assert_eq!(parse_vdl(input).unwrap(), expected);
	}

	#[test]
	fn test_parse_nodes3() {
		let input = "node1 key1=value1 [ child1 key2=value2 | child2 key3=\"value3\" ] | node2";
		let expected = VDLPipeline::from(vec![
			VDLNode::from((
				"node1",
				vec![("key1", "value1")],
				vec![
					VDLPipeline::from(VDLNode::from(("child1", ("key2", "value2")))),
					VDLPipeline::from(VDLNode::from(("child2", ("key3", "value3")))),
				],
			)),
			VDLNode::from("node2"),
		]);
		assert_eq!(parse_vdl(input).unwrap(), expected);
	}

	#[test]
	fn test_parse_nodes4() {
		pub const INPUT: &str = include_str!("../../../../testdata/berlin.vdl");

		let expected = VDLPipeline::from(vec![
			VDLNode::from(("read", ("filename", "berlin.mbtiles"))),
			VDLNode::from((
				"vectortiles_update_properties",
				vec![
					("data_source_path", "cities.csv"),
					("id_field_tiles", "id"),
					("id_field_values", "city_id"),
				],
			)),
		]);
		assert_eq!(parse_vdl(INPUT).unwrap(), expected);
	}

	#[test]
	fn test_parse_unquoted_value() {
		let inputs = ["value1", "value.1", "value-1", "value_1"];

		for input in inputs.iter() {
			assert_eq!(parse_unquoted_value(input).unwrap().1, *input);
		}
	}

	#[test]
	fn test_parse_value() {
		assert_eq!(parse_value("value1"), Ok(("", vec!["value1".to_string()])));
		assert_eq!(
			parse_value("\"value1\""),
			Ok(("", vec!["value1".to_string()]))
		);
		assert_eq!(
			parse_value("value 1"),
			Ok((" 1", vec!["value".to_string()]))
		);
		assert_eq!(
			parse_value("value\""),
			Ok(("\"", vec!["value".to_string()]))
		);
		assert!(parse_value("\"value").is_err());
	}

	#[test]
	fn test_parse_vdl_with_error() {
		let input = "node1 key1=value1 [ child1 key2=value2 | child2 key3=\"value3\" ] node2";
		let result = parse_vdl(input);
		assert!(result.is_err());
	}
}