//! 配置绑定宏实现

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::Parse, parse::ParseStream, parse_macro_input, punctuated::Punctuated, DeriveInput, Expr,
    Ident, ItemStruct, Lit, Meta, Result, Token, Type,
};

use crate::utils::find_config_field;

/// 配置宏参数
#[derive(Debug, Clone)]
pub struct ConfigurableArgs {
    /// 配置路径
    pub path: String,
    /// 是否可选
    pub optional: bool,
    /// 是否使用默认配置
    pub use_default: bool,
    /// 配置字段名称
    pub config_field: Option<String>,
}

impl Default for ConfigurableArgs {
    fn default() -> Self {
        Self {
            path: String::new(),
            optional: false,
            use_default: false,
            config_field: None,
        }
    }
}

impl Parse for ConfigurableArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = ConfigurableArgs::default();

        let parsed = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

        for meta in parsed {
            match meta {
                Meta::Path(path) => {
                    if path.is_ident("optional") {
                        args.optional = true;
                    } else if path.is_ident("default") {
                        args.use_default = true;
                    }
                }
                Meta::NameValue(nv) => {
                    if nv.path.is_ident("path") {
                        if let Expr::Lit(expr_lit) = nv.value {
                            if let Lit::Str(lit_str) = expr_lit.lit {
                                args.path = lit_str.value();
                            }
                        }
                    } else if nv.path.is_ident("field") {
                        if let Expr::Lit(expr_lit) = nv.value {
                            if let Lit::Str(lit_str) = expr_lit.lit {
                                args.config_field = Some(lit_str.value());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if args.path.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                "配置路径不能为空，请使用 path = \"config.path\" 指定",
            ));
        }

        Ok(args)
    }
}

/// 实现 #[configurable] 宏
pub fn configurable_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let configurable_args = if args.is_empty() {
        return syn::Error::new(
            Span::call_site(),
            "configurable 宏需要指定配置路径，例如: #[configurable(path = \"config.path\")]",
        )
        .to_compile_error()
        .into();
    } else {
        match syn::parse::<ConfigurableArgs>(args) {
            Ok(args) => args,
            Err(e) => return e.to_compile_error().into(),
        }
    };

    let input_struct = parse_macro_input!(input as ItemStruct);

    let struct_name = &input_struct.ident;
    let config_path = &configurable_args.path;

    // 尝试推断配置类型
    let config_type = infer_config_type(&input_struct, &configurable_args);

    let configurable_impl = generate_configurable_impl(
        struct_name,
        &config_type,
        config_path,
        configurable_args.optional,
        configurable_args.use_default,
    );

    let expanded = quote! {
        #input_struct

        #configurable_impl
    };

    TokenStream::from(expanded)
}

/// 推断配置类型
fn infer_config_type(input_struct: &ItemStruct, args: &ConfigurableArgs) -> Type {
    // 如果指定了配置字段，尝试从该字段推断类型
    if let Some(field_name) = &args.config_field {
        if let Some(field) = find_config_field(&input_struct.fields, field_name) {
            return field.ty.clone();
        }
    }

    // 尝试查找名为 "config" 的字段
    if let Some(field) = find_config_field(&input_struct.fields, "config") {
        return field.ty.clone();
    }

    // 尝试查找以 "Config" 结尾的字段
    for field in &input_struct.fields {
        if let Some(ident) = &field.ident {
            if ident.to_string().ends_with("config") || ident.to_string().ends_with("Config") {
                return field.ty.clone();
            }
        }
    }

    // 默认使用 {StructName}Config 类型
    let struct_name = &input_struct.ident;
    let config_type_name = format!("{}Config", struct_name);
    let config_ident = Ident::new(&config_type_name, Span::call_site());
    syn::parse_quote! { #config_ident }
}

/// 生成 Configurable trait 实现
fn generate_configurable_impl(
    struct_name: &Ident,
    config_type: &Type,
    config_path: &str,
    optional: bool,
    use_default: bool,
) -> proc_macro2::TokenStream {
    // 从 Option<T> 中提取 T 类型
    let actual_config_type = if let Type::Path(type_path) = config_type {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                        inner_type
                    } else {
                        config_type
                    }
                } else {
                    config_type
                }
            } else {
                config_type
            }
        } else {
            config_type
        }
    } else {
        config_type
    };

    let configure_impl = if optional {
        quote! {
            fn configure(&mut self, config: Self::Config) -> Result<(), infrastructure_common::ConfigError> {
                // 可选配置的实现
                // 这里可以添加具体的配置应用逻辑
                Ok(())
            }
        }
    } else {
        quote! {
            fn configure(&mut self, config: Self::Config) -> Result<(), infrastructure_common::ConfigError> {
                // 必需配置的实现
                // 这里可以添加具体的配置应用逻辑
                Ok(())
            }
        }
    };

    let default_config_impl = if use_default {
        quote! {
            fn default_config() -> Self::Config
            where
                Self::Config: Default,
            {
                Self::Config::default()
            }
        }
    } else {
        quote! {}
    };

    quote! {
        impl infrastructure_common::Configurable for #struct_name {
            type Config = #actual_config_type;

            #configure_impl

            fn get_config_path() -> &'static str {
                #config_path
            }

            #default_config_impl
        }
    }
}

/// 实现 #[derive(Configurable)] 宏
pub fn derive_configurable_impl(input: DeriveInput) -> TokenStream {
    let struct_name = &input.ident;

    // 查找 #[configurable] 属性
    let mut config_path = None;
    let mut optional = false;
    let mut use_default = false;

    for attr in &input.attrs {
        if attr.path().is_ident("configurable") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("path") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(lit_str) = lit {
                        config_path = Some(lit_str.value());
                    }
                } else if meta.path.is_ident("optional") {
                    optional = true;
                } else if meta.path.is_ident("default") {
                    use_default = true;
                }
                Ok(())
            });
        }
    }

    let config_path = config_path.unwrap_or_else(|| {
        // 默认配置路径：将结构体名称转换为蛇形命名
        let snake_case = to_snake_case(&struct_name.to_string());
        format!("components.{}", snake_case)
    });

    // 推断配置类型
    let config_type_name = format!("{}Config", struct_name);
    let config_type = Ident::new(&config_type_name, Span::call_site());

    let configure_impl = if optional {
        quote! {
            fn configure(&mut self, config: Self::Config) -> Result<(), infrastructure_common::ConfigError> {
                // 可选配置的默认实现
                Ok(())
            }
        }
    } else {
        quote! {
            fn configure(&mut self, config: Self::Config) -> Result<(), infrastructure_common::ConfigError> {
                // 必需配置的默认实现
                Ok(())
            }
        }
    };

    let default_config_impl = if use_default {
        quote! {
            fn default_config() -> Self::Config
            where
                Self::Config: Default,
            {
                Self::Config::default()
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        impl infrastructure_common::Configurable for #struct_name {
            type Config = #config_type;

            #configure_impl

            fn get_config_path() -> &'static str {
                #config_path
            }

            #default_config_impl
        }
    };

    TokenStream::from(expanded)
}

/// 将驼峰命名转换为蛇形命名
fn to_snake_case(s: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("MyService"), "my_service");
        assert_eq!(to_snake_case("HTTPClient"), "http_client");
        assert_eq!(to_snake_case("XMLParser"), "xml_parser");
        assert_eq!(to_snake_case("SimpleTest"), "simple_test");
    }
}
