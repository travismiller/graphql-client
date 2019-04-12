use graphql_client::*;

const RESPONSE: &'static str = include_str!("unions/union_query_response.json");

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "tests/unions/union_query.graphql",
    schema_path = "tests/unions/union_schema.graphql",
    response_derives = "PartialEq, Debug"
)]
pub struct UnionQuery;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "tests/unions/union_query.graphql",
    schema_path = "tests/unions/union_schema.graphql",
    response_derives = "PartialEq, Debug"
)]
pub struct FragmentOnUnion;

#[test]
fn union_query_deserialization() {
    let response_data: union_query::ResponseData = serde_json::from_str(RESPONSE).unwrap();

    let expected = union_query::ResponseData {
        names: Some(vec![
            union_query::UnionQueryNames::Person(union_query::UnionQueryNamesOnPerson {
                first_name: "Audrey".to_string(),
                last_name: Some("Lorde".to_string()),
            }),
            union_query::UnionQueryNames::Dog(union_query::UnionQueryNamesOnDog {
                name: "Laïka".to_string(),
            }),
            union_query::UnionQueryNames::Organization(
                union_query::UnionQueryNamesOnOrganization {
                    title: "Mozilla".to_string(),
                },
            ),
            union_query::UnionQueryNames::Dog(union_query::UnionQueryNamesOnDog {
                name: "Norbert".to_string(),
            }),
        ]),
    };

    assert_eq!(response_data, expected);

    assert_eq!(response_data.names.map(|names| names.len()), Some(4));
}

#[test]
fn fragment_on_union() {
    let response_data: fragment_on_union::ResponseData = serde_json::from_str(RESPONSE).unwrap();

    let expected = fragment_on_union::ResponseData {
        names: Some(vec![
            fragment_on_union::FragmentOnUnionNames::Person(
                fragment_on_union::FragmentOnUnionNamesOnPerson {
                    first_name: "Audrey".to_string(),
                },
            ),
            fragment_on_union::FragmentOnUnionNames::Dog(
                fragment_on_union::FragmentOnUnionNamesOnDog {
                    name: "Laïka".to_string(),
                },
            ),
            fragment_on_union::FragmentOnUnionNames::Organization(
                fragment_on_union::FragmentOnUnionNamesOnOrganization {
                    title: "Mozilla".to_string(),
                },
            ),
            fragment_on_union::FragmentOnUnionNames::Dog(
                fragment_on_union::FragmentOnUnionNamesOnDog {
                    name: "Norbert".to_string(),
                },
            ),
        ]),
    };

    assert_eq!(response_data, expected);
}
