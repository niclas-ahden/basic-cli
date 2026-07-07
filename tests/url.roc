app [main!] { pf: platform "../platform/main.roc" }

import pf.Stdout
import pf.Url

main! : List(Str) => Try({}, [Exit(I32), ..])
main! = |_args|
    match run_tests!() {
        Ok({}) => Ok({})
        Err(err) => {
            Stdout.line!("Test run failed: ${Str.inspect(err)}") ? |_| Exit(1)
            Err(Exit(1))
        }
    }

run_tests! : () => Try({}, _)
run_tests! = || {
    Stdout.line!("Testing Url module functions...")?

    test_part_1!()?
    test_part_2!()?

    Stdout.line!("\nAll tests executed.")?
    Ok({})
}

test_part_1! : () => Try({}, _)
test_part_1! = || {
    url = Url.from_str("https://example.com")
    Stdout.line!("Created URL: ${Url.to_str(url)}")?

    Stdout.line!("Testing Url.append:")?

    url_with_path = Url.append(url, "some stuff")
    Stdout.line!("URL with append: ${Url.to_str(url_with_path)}")?

    url_search = Url.from_str("https://example.com?search=blah#fragment")
    url_search_append = Url.append(url_search, "stuff")
    Stdout.line!("URL with query and fragment, then appended path: ${Url.to_str(url_search_append)}")?

    url_things = Url.from_str("https://example.com/things/")
    url_things_append = Url.append(url_things, "/stuff/")
    url_things_append_more = Url.append(url_things_append, "/more/etc/")
    Stdout.line!("URL with multiple appended paths: ${Url.to_str(url_things_append_more)}")?

    Stdout.line!("Testing Url.append_param:")?

    url_example = Url.from_str("https://example.com")
    url_example_param = Url.append_param(url_example, "email", "someone@example.com")
    Stdout.line!("URL with appended param: ${Url.to_str(url_example_param)}")?

    url_example_2 = Url.from_str("https://example.com")
    url_example_2_cafe = Url.append_param(url_example_2, "café", "du Monde")
    url_example_2_cafe_email = Url.append_param(url_example_2_cafe, "email", "hi@example.com")
    Stdout.line!("URL with multiple appended params: ${Url.to_str(url_example_2_cafe_email)}")?

    Stdout.line!("\nTesting Url.has_query:")?

    url_with_query = Url.from_str("https://example.com?key=value#stuff")
    has_query_1 = Url.has_query(url_with_query)
    Stdout.line!("URL with query has_query: ${bool_to_old_str(has_query_1)}")?

    url_hashtag = Url.from_str("https://example.com#stuff")
    has_query_2 = Url.has_query(url_hashtag)
    Stdout.line!("URL without query has_query: ${bool_to_old_str(has_query_2)}")?

    Stdout.line!("\nTesting Url.has_fragment:")?

    url_key_val_hashtag = Url.from_str("https://example.com?key=value#stuff")
    has_fragment = Url.has_fragment(url_key_val_hashtag)
    Stdout.line!("URL with fragment has_fragment: ${bool_to_old_str(has_fragment)}")?

    url_key_val = Url.from_str("https://example.com?key=value")
    has_fragment_2 = Url.has_fragment(url_key_val)
    Stdout.line!("URL without fragment has_fragment: ${bool_to_old_str(has_fragment_2)}")?

    Stdout.line!("\nTesting Url.query:")?

    url_key_val_multi = Url.from_str("https://example.com?key1=val1&key2=val2&key3=val3#stuff")
    query = Url.query(url_key_val_multi)
    Stdout.line!("Query from URL: ${query}")?

    url_no_query = Url.from_str("https://example.com#stuff")
    query_empty = Url.query(url_no_query)
    Stdout.line!("Query from URL without query:${query_empty}")?

    Ok({})
}

test_part_2! : () => Try({}, _)
test_part_2! = || {
    Stdout.line!("\nTesting Url.fragment:")?

    url_with_fragment = Url.from_str("https://example.com#stuff")
    fragment = Url.fragment(url_with_fragment)
    Stdout.line!("Fragment from URL: ${fragment}")?

    url_no_fragment = Url.from_str("https://example.com")
    fragment_empty = Url.fragment(url_no_fragment)
    Stdout.line!("Fragment from URL without fragment:${fragment_empty}")?

    Stdout.line!("\nTesting Url.reserve:")?

    url_to_reserve = Url.from_str("https://example.com")
    url_reserved = Url.reserve(url_to_reserve, 50)
    url_reserved_path = Url.append(url_reserved, "stuff")
    url_reserved_cafe = Url.append_param(url_reserved_path, "café", "du Monde")
    url_with_params = Url.append_param(url_reserved_cafe, "email", "hi@example.com")

    Stdout.line!("URL with reserved capacity and params: ${Url.to_str(url_with_params)}")?

    Stdout.line!("\nTesting Url.with_query:")?

    url_replace_query = Url.from_str("https://example.com?key1=val1&key2=val2#stuff")
    url_with_new_query = Url.with_query(url_replace_query, "newQuery=thisRightHere")
    Stdout.line!("URL with replaced query: ${Url.to_str(url_with_new_query)}")?

    url_remove_query = Url.from_str("https://example.com?key1=val1&key2=val2#stuff")
    url_with_empty_query = Url.with_query(url_remove_query, "")
    Stdout.line!("URL with removed query: ${Url.to_str(url_with_empty_query)}")?

    Stdout.line!("\nTesting Url.with_fragment:")?

    url_replace_fragment = Url.from_str("https://example.com#stuff")
    url_with_new_fragment = Url.with_fragment(url_replace_fragment, "things")
    Stdout.line!("URL with replaced fragment: ${Url.to_str(url_with_new_fragment)}")?

    url_add_fragment = Url.from_str("https://example.com")
    url_with_added_fragment = Url.with_fragment(url_add_fragment, "things")
    Stdout.line!("URL with added fragment: ${Url.to_str(url_with_added_fragment)}")?

    url_remove_fragment = Url.from_str("https://example.com#stuff")
    url_with_empty_fragment = Url.with_fragment(url_remove_fragment, "")
    Stdout.line!("URL with removed fragment: ${Url.to_str(url_with_empty_fragment)}")?

    Stdout.line!("\nTesting Url.query_params:")?

    url_with_many_params = Url.from_str("https://example.com?key1=val1&key2=val2&key3=val3")
    params_dict = Url.query_params(url_with_many_params)
    expect_param!(params_dict, "key1", "val1")?
    expect_param!(params_dict, "key2", "val2")?
    expect_param!(params_dict, "key3", "val3")?
    Stdout.line!("params_dict: {\"key1\": \"val1\", \"key2\": \"val2\", \"key3\": \"val3\"}")?

    Stdout.line!("\nTesting Url.path:")?

    url_with_path = Url.from_str("https://example.com/foo/bar?key1=val1&key2=val2#stuff")
    path = Url.path(url_with_path)
    Stdout.line!("Path from URL: ${path}")?

    url_relative = Url.from_str("/foo/bar?key1=val1&key2=val2#stuff")
    path_relative = Url.path(url_relative)
    Stdout.line!("Path from relative URL: ${path_relative}")?

    Ok({})
}

bool_to_old_str : Bool -> Str
bool_to_old_str = |value|
    if value {
        "Bool.true"
    } else {
        "Bool.false"
    }

expect_param! : Dict(Str, Str), Str, Str => Try({}, [TestFailed(Str), ..])
expect_param! = |dict, key, expected|
    match Dict.get(dict, key) {
        Ok(actual) =>
            if actual == expected {
                Ok({})
            } else {
                Err(TestFailed("Expected query param ${key}=${expected}, got ${actual}"))
            }
        Err(_) => Err(TestFailed("Missing query param ${key}"))
    }
