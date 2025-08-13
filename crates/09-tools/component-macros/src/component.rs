//! 组件注册宏实现

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::Parse, parse::ParseStream, parse_macro_input, punctuated::Punctuated, DeriveInput, Expr,
    Ident, ItemStruct, Lit, Meta, Result, Token,
};

/// 组件配置参数
#[derive(Debug, Clone)]
pub struct ComponentArgs {
    /// 生命周期类型
    pub lifetime: ComponentLifetime,
    /// 组件优先级
    pub priority: i32,
    /// 自定义组件名称
    pub name: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

/// 组件生命周期类型
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentLifetime {
    Singleton,
    Scoped,
    Transient,
}

impl Default for ComponentArgs {
    fn default() -> Self {
        Self {
            lifetime: ComponentLifetime::Singleton,
            priority: 0,
            name: None,
            enabled: true,
        }
    }
}

impl Parse for ComponentArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = ComponentArgs::default();

        let parsed = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

        for meta in parsed {
            match meta {
                Meta::Path(path) => {
                    if path.is_ident("singleton") {
                        args.lifetime = ComponentLifetime::Singleton;
                    } else if path.is_ident("scoped") {
                        args.lifetime = ComponentLifetime::Scoped;
                    } else if path.is_ident("transient") {
                        args.lifetime = ComponentLifetime::Transient;
                    } else if path.is_ident("enabled") {
                        args.enabled = true;
                    } else if path.is_ident("disabled") {
                        args.enabled = false;
                    }
                }
                Meta::NameValue(nv) => {
                    if nv.path.is_ident("priority") {
                        if let Expr::Lit(expr_lit) = nv.value {
                            if let Lit::Int(lit_int) = expr_lit.lit {
                                args.priority = lit_int.base10_parse()?;
                            }
                        }
                    } else if nv.path.is_ident("name") {
                        if let Expr::Lit(expr_lit) = nv.value {
                            if let Lit::Str(lit_str) = expr_lit.lit {
                                args.name = Some(lit_str.value());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(args)
    }
}

/// 实现 #[component] 宏
pub fn component_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let component_args = if args.is_empty() {
        ComponentArgs::default()
    } else {
        match syn::parse::<ComponentArgs>(args) {
            Ok(args) => args,
            Err(e) => return e.to_compile_error().into(),
        }
    };

    let input_struct = parse_macro_input!(input as ItemStruct);

    let struct_name = &input_struct.ident;
    let struct_name_string = struct_name.to_string();
    let component_name = component_args
        .name
        .as_deref()
        .unwrap_or(&struct_name_string);

    let lifetime_variant = match component_args.lifetime {
        ComponentLifetime::Singleton => quote! { infrastructure_common::Lifetime::Singleton },
        ComponentLifetime::Scoped => quote! { infrastructure_common::Lifetime::Scoped },
        ComponentLifetime::Transient => quote! { infrastructure_common::Lifetime::Transient },
    };

    let priority = component_args.priority;
    let enabled = component_args.enabled;

    // 生成 Component trait 实现
    let component_impl = quote! {
        impl infrastructure_common::Component for #struct_name {
            fn name(&self) -> &'static str {
                #component_name
            }

            fn priority(&self) -> i32 {
                #priority
            }

            fn is_enabled(&self) -> bool {
                #enabled
            }
        }
    };

    // 生成自动注册代码
    let registration_code = generate_registration_code(
        struct_name,
        component_name,
        &lifetime_variant,
        priority,
        enabled,
    );

    let expanded = quote! {
        #input_struct

        #component_impl

        #registration_code
    };

    TokenStream::from(expanded)
}

/// 生成组件自动注册代码
fn generate_registration_code(
    struct_name: &Ident,
    component_name: &str,
    lifetime: &proc_macro2::TokenStream,
    priority: i32,
    enabled: bool,
) -> proc_macro2::TokenStream {
    let registration_fn_name = Ident::new(
        &format!(
            "__register_component_{}",
            struct_name.to_string().to_lowercase()
        ),
        Span::call_site(),
    );

    quote! {
        // 使用 ctor 在程序启动时自动注册组件
        #[ctor::ctor]
        fn #registration_fn_name() {
            use infrastructure_common::{ComponentDescriptor, get_global_component_registry};
            use std::any::TypeId;
            use std::collections::HashMap;

            let descriptor = ComponentDescriptor {
                name: #component_name.to_string(),
                type_id: TypeId::of::<#struct_name>(),
                lifetime: #lifetime,
                priority: #priority,
                enabled: #enabled,
                metadata: HashMap::new(),
            };

            // 注册到全局组件注册表
            if let Some(registry) = get_global_component_registry() {
                if let Err(e) = registry.register_component_descriptor(descriptor) {
                    eprintln!("Failed to register component {}: {}", #component_name, e);
                }
            }
        }
    }
}

/// 实现 #[derive(Component)] 宏
pub fn derive_component_impl(input: DeriveInput) -> TokenStream {
    let struct_name = &input.ident;
    let component_name = struct_name.to_string();

    // 检查是否有 #[component] 属性
    let mut priority = 0;
    let mut enabled = true;
    let mut custom_name = None;

    for attr in &input.attrs {
        if attr.path().is_ident("component") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("priority") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Int(lit_int) = lit {
                        priority = lit_int.base10_parse().unwrap_or(0);
                    }
                } else if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(lit_str) = lit {
                        custom_name = Some(lit_str.value());
                    }
                } else if meta.path.is_ident("disabled") {
                    enabled = false;
                }
                Ok(())
            });
        }
    }

    let final_name = custom_name.as_deref().unwrap_or(&component_name);

    let expanded = quote! {
        impl infrastructure_common::Component for #struct_name {
            fn name(&self) -> &'static str {
                #final_name
            }

            fn priority(&self) -> i32 {
                #priority
            }

            fn is_enabled(&self) -> bool {
                #enabled
            }
        }
    };

    TokenStream::from(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_args_defaults() {
        let args = ComponentArgs::default();

        assert_eq!(args.lifetime, ComponentLifetime::Singleton);
        assert_eq!(args.priority, 0);
        assert_eq!(args.name, None);
        assert!(args.enabled);
    }
}
