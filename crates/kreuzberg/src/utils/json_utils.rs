use serde_json::Value;

/// Recursively convert snake_case keys in a JSON Value to camelCase.
///
/// This is used by language bindings (Node.js, Go, Java, C#, etc.) to provide
/// a consistent camelCase API for consumers even though the Rust core uses snake_case.
pub fn snake_to_camel(val: Value) -> Value {
    match val {
        Value::Object(map) => {
            let mut new_map = serde_json::Map::with_capacity(map.len());
            for (key, value) in map {
                let new_key = to_camel_case(&key);
                new_map.insert(new_key, snake_to_camel(value));
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(snake_to_camel).collect()),
        _ => val,
    }
}

/// snake_case to camelCase converter for keys.
fn to_camel_case(s: &str) -> String {
    let mut camel = String::with_capacity(s.len());
    let mut capitalize_next = false;

    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            camel.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            camel.push(c);
        }
    }
    camel
}

/// Recursively convert camelCase keys in a JSON Value to snake_case.
///
/// This is the inverse of `snake_to_camel`. Used by WASM bindings to accept
/// camelCase config from JavaScript while the Rust core expects snake_case.
pub fn camel_to_snake(val: Value) -> Value {
    match val {
        Value::Object(map) => {
            let mut new_map = serde_json::Map::with_capacity(map.len());
            for (key, value) in map {
                let new_key = to_snake_case(&key);
                new_map.insert(new_key, camel_to_snake(value));
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(camel_to_snake).collect()),
        _ => val,
    }
}

/// camelCase to snake_case converter for keys.
fn to_snake_case(s: &str) -> String {
    let mut snake = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            snake.push('_');
            snake.push(c.to_ascii_lowercase());
        } else {
            snake.push(c);
        }
    }
    snake
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_snake_to_camel_basic() {
        let input = json!({
            "first_name": "John",
            "last_name": "Doe",
            "nested_object": {
                "street_address": "123 Main St"
            },
            "array_of_objects": [
                { "item_id": 1 },
                { "item_name": "item 2" }
            ]
        });

        let expected = json!({
            "firstName": "John",
            "lastName": "Doe",
            "nestedObject": {
                "streetAddress": "123 Main St"
            },
            "arrayOfObjects": [
                { "itemId": 1 },
                { "itemName": "item 2" }
            ]
        });

        assert_eq!(snake_to_camel(input), expected);
    }

    #[test]
    fn test_camel_to_snake_basic() {
        let input = json!({
            "firstName": "John",
            "lastName": "Doe",
            "nestedObject": {
                "streetAddress": "123 Main St"
            },
            "arrayOfObjects": [
                { "itemId": 1 },
                { "itemName": "item 2" }
            ]
        });

        let expected = json!({
            "first_name": "John",
            "last_name": "Doe",
            "nested_object": {
                "street_address": "123 Main St"
            },
            "array_of_objects": [
                { "item_id": 1 },
                { "item_name": "item 2" }
            ]
        });

        assert_eq!(camel_to_snake(input), expected);
    }
}
