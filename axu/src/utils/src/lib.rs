use serde_json::Value;


pub fn update_resp(json_obj: &mut Value, field_name: &str, new_value: Value) {
    if let Value::Object(obj) = json_obj {
        if let Some(field) = obj.get_mut(field_name) {
            *field = new_value;
        }
    }
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
