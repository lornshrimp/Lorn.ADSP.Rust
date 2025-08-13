//! 宏工具函数

use proc_macro2::Span;
use syn::{Expr, Field, Fields, Ident, Lit, Meta, Result, Type};

/// 从结构体中提取名称
pub fn extract_struct_name(ident: &Ident) -> String {
    ident.to_string()
}

/// 解析属性参数的通用函数
pub fn parse_attribute_args(args: Vec<Meta>) -> Result<Vec<(String, Option<String>)>> {
    let mut parsed_args = Vec::new();

    for arg in args {
        match arg {
            Meta::Path(path) => {
                if let Some(ident) = path.get_ident() {
                    parsed_args.push((ident.to_string(), None));
                }
            }
            Meta::NameValue(nv) => {
                if let Some(ident) = nv.path.get_ident() {
                    let value = match nv.value {
                        Expr::Lit(expr_lit) => match expr_lit.lit {
                            Lit::Str(lit_str) => Some(lit_str.value()),
                            Lit::Int(lit_int) => Some(lit_int.to_string()),
                            Lit::Bool(lit_bool) => Some(lit_bool.value.to_string()),
                            _ => None,
                        },
                        _ => None,
                    };
                    parsed_args.push((ident.to_string(), value));
                }
            }
            _ => {}
        }
    }

    Ok(parsed_args)
}

/// 查找配置字段
pub fn find_config_field<'a>(fields: &'a Fields, field_name: &str) -> Option<&'a Field> {
    match fields {
        Fields::Named(fields_named) => fields_named.named.iter().find(|field| {
            field
                .ident
                .as_ref()
                .map(|ident| ident == field_name)
                .unwrap_or(false)
        }),
        _ => None,
    }
}

/// 查找第一个配置相关的字段
pub fn find_first_config_field<'a>(fields: &'a Fields) -> Option<&'a Field> {
    match fields {
        Fields::Named(fields_named) => {
            // 优先查找名为 "config" 的字段
            if let Some(field) = fields_named.named.iter().find(|field| {
                field
                    .ident
                    .as_ref()
                    .map(|ident| ident == "config")
                    .unwrap_or(false)
            }) {
                return Some(field);
            }

            // 查找以 "config" 或 "Config" 结尾的字段
            fields_named.named.iter().find(|field| {
                field
                    .ident
                    .as_ref()
                    .map(|ident| {
                        let name = ident.to_string();
                        name.ends_with("config") || name.ends_with("Config")
                    })
                    .unwrap_or(false)
            })
        }
        _ => None,
    }
}

/// 从类型中提取泛型参数
pub fn extract_generic_type(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                match &segment.arguments {
                    syn::PathArguments::AngleBracketed(args) => {
                        if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                            return Some(inner_type);
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    None
}

/// 检查类型是否为 Option<T>
pub fn is_option_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                segment.ident == "Option"
            } else {
                false
            }
        }
        _ => false,
    }
}

/// 检查类型是否为 Result<T, E>
pub fn is_result_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                segment.ident == "Result"
            } else {
                false
            }
        }
        _ => false,
    }
}

/// 生成字段访问器方法名
pub fn generate_accessor_name(field_name: &str, prefix: &str) -> Ident {
    let accessor_name = format!("{}_{}", prefix, field_name);
    Ident::new(&accessor_name, Span::call_site())
}

/// 生成设置器方法名
pub fn generate_setter_name(field_name: &str) -> Ident {
    let setter_name = format!("set_{}", field_name);
    Ident::new(&setter_name, Span::call_site())
}

/// 生成获取器方法名
pub fn generate_getter_name(field_name: &str) -> Ident {
    let getter_name = format!("get_{}", field_name);
    Ident::new(&getter_name, Span::call_site())
}

/// 将驼峰命名转换为蛇形命名
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_uppercase() && i > 0 {
            // 检查前一个字符是否为小写，或者下一个字符是否为小写
            let prev_is_lower = chars.get(i - 1).map_or(false, |c| c.is_lowercase());
            let next_is_lower = chars.get(i + 1).map_or(false, |c| c.is_lowercase());

            if prev_is_lower || next_is_lower {
                result.push('_');
            }
        }
        result.push(ch.to_lowercase().next().unwrap_or(ch));
    }

    result
}

/// 将蛇形命名转换为驼峰命名
pub fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_uppercase().next().unwrap_or(ch));
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

/// 将蛇形命名转换为帕斯卡命名
pub fn to_pascal_case(s: &str) -> String {
    let camel_case = to_camel_case(s);
    if let Some(first_char) = camel_case.chars().next() {
        format!("{}{}", first_char.to_uppercase(), &camel_case[1..])
    } else {
        camel_case
    }
}

/// 验证标识符是否有效
pub fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    let first_char = chars.next().unwrap();

    // 第一个字符必须是字母或下划线
    if !first_char.is_alphabetic() && first_char != '_' {
        return false;
    }

    // 其余字符必须是字母、数字或下划线
    chars.all(|ch| ch.is_alphanumeric() || ch == '_')
}

/// 生成唯一的标识符
pub fn generate_unique_ident(base_name: &str, suffix: &str) -> Ident {
    let unique_name = format!("__{}__{}", base_name, suffix);
    Ident::new(&unique_name, Span::call_site())
}

/// 检查字段是否有特定属性
pub fn field_has_attribute(field: &Field, attr_name: &str) -> bool {
    field.attrs.iter().any(|attr| {
        attr.path()
            .get_ident()
            .map(|ident| ident == attr_name)
            .unwrap_or(false)
    })
}

/// 从字段属性中提取字符串值
pub fn extract_string_from_field_attr(field: &Field, attr_name: &str) -> Option<String> {
    for attr in &field.attrs {
        if attr
            .path()
            .get_ident()
            .map(|ident| ident == attr_name)
            .unwrap_or(false)
        {
            let mut result = None;
            let _ = attr.parse_nested_meta(|meta| {
                if let Ok(value) = meta.value() {
                    if let Ok(lit) = value.parse::<Lit>() {
                        if let Lit::Str(lit_str) = lit {
                            result = Some(lit_str.value());
                        }
                    }
                }
                Ok(())
            });
            if result.is_some() {
                return result;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("MyService"), "my_service");
        assert_eq!(to_snake_case("HTTPClient"), "http_client");
        assert_eq!(to_snake_case("XMLParser"), "xml_parser");
        assert_eq!(to_snake_case("SimpleTest"), "simple_test");
        assert_eq!(to_snake_case("already_snake"), "already_snake");
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("my_service"), "myService");
        assert_eq!(to_camel_case("http_client"), "httpClient");
        assert_eq!(to_camel_case("simple_test"), "simpleTest");
        assert_eq!(to_camel_case("alreadyCamel"), "alreadyCamel");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("my_service"), "MyService");
        assert_eq!(to_pascal_case("http_client"), "HttpClient");
        assert_eq!(to_pascal_case("simple_test"), "SimpleTest");
        assert_eq!(to_pascal_case("AlreadyPascal"), "AlreadyPascal");
    }

    #[test]
    fn test_is_valid_identifier() {
        assert!(is_valid_identifier("valid_name"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("name123"));
        assert!(is_valid_identifier("CamelCase"));

        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("123invalid"));
        assert!(!is_valid_identifier("invalid-name"));
        assert!(!is_valid_identifier("invalid.name"));
    }
}
