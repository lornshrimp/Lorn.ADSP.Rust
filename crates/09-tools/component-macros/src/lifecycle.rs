//! 生命周期管理宏实现

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::Parse, parse::ParseStream, parse_macro_input, punctuated::Punctuated, Expr, Ident,
    ItemStruct, Lit, Meta, Result, Token,
};

/// 生命周期宏参数
#[derive(Debug, Clone)]
pub struct LifecycleArgs {
    /// 启动时调用的方法
    pub on_start: Option<String>,
    /// 停止时调用的方法
    pub on_stop: Option<String>,
    /// 依赖的组件列表
    pub depends_on: Vec<String>,
    /// 初始化方法
    pub initialize: Option<String>,
    /// 清理方法
    pub cleanup: Option<String>,
    /// 是否异步生命周期
    pub async_lifecycle: bool,
}

impl Default for LifecycleArgs {
    fn default() -> Self {
        Self {
            on_start: None,
            on_stop: None,
            depends_on: Vec::new(),
            initialize: None,
            cleanup: None,
            async_lifecycle: false,
        }
    }
}

impl Parse for LifecycleArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = LifecycleArgs::default();

        let parsed = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

        for meta in parsed {
            match meta {
                Meta::Path(path) => {
                    if path.is_ident("async") {
                        args.async_lifecycle = true;
                    }
                }
                Meta::NameValue(nv) => {
                    if nv.path.is_ident("on_start") {
                        if let Expr::Lit(expr_lit) = nv.value {
                            if let Lit::Str(lit_str) = expr_lit.lit {
                                args.on_start = Some(lit_str.value());
                            }
                        }
                    } else if nv.path.is_ident("on_stop") {
                        if let Expr::Lit(expr_lit) = nv.value {
                            if let Lit::Str(lit_str) = expr_lit.lit {
                                args.on_stop = Some(lit_str.value());
                            }
                        }
                    } else if nv.path.is_ident("initialize") {
                        if let Expr::Lit(expr_lit) = nv.value {
                            if let Lit::Str(lit_str) = expr_lit.lit {
                                args.initialize = Some(lit_str.value());
                            }
                        }
                    } else if nv.path.is_ident("cleanup") {
                        if let Expr::Lit(expr_lit) = nv.value {
                            if let Lit::Str(lit_str) = expr_lit.lit {
                                args.cleanup = Some(lit_str.value());
                            }
                        }
                    }
                }
                Meta::List(list) => {
                    if list.path.is_ident("depends_on") {
                        // 解析依赖列表
                        let content = list.tokens.to_string();
                        // 简单的字符串解析，实际实现可能需要更复杂的解析
                        let deps: Vec<String> = content
                            .split(',')
                            .map(|s| s.trim().trim_matches('"').to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        args.depends_on = deps;
                    }
                }
            }
        }

        Ok(args)
    }
}

/// 实现 #[lifecycle] 宏
pub fn lifecycle_impl(args: TokenStream, input: TokenStream) -> TokenStream {
    let lifecycle_args = if args.is_empty() {
        LifecycleArgs::default()
    } else {
        match syn::parse::<LifecycleArgs>(args) {
            Ok(args) => args,
            Err(e) => return e.to_compile_error().into(),
        }
    };

    let input_struct = parse_macro_input!(input as ItemStruct);

    let struct_name = &input_struct.ident;

    // 生成生命周期管理实现
    let lifecycle_impl = generate_lifecycle_impl(struct_name, &lifecycle_args);

    // 生成依赖管理代码
    let dependency_impl = generate_dependency_impl(struct_name, &lifecycle_args);

    let expanded = quote! {
        #input_struct

        #lifecycle_impl

        #dependency_impl
    };

    TokenStream::from(expanded)
}

/// 生成生命周期管理实现
fn generate_lifecycle_impl(struct_name: &Ident, args: &LifecycleArgs) -> proc_macro2::TokenStream {
    let start_method = if let Some(method_name) = &args.on_start {
        let method_ident = Ident::new(method_name, Span::call_site());
        if args.async_lifecycle {
            quote! {
                async fn on_start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    self.#method_ident().await
                }
            }
        } else {
            quote! {
                async fn on_start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    self.#method_ident()
                }
            }
        }
    } else if let Some(method_name) = &args.initialize {
        let method_ident = Ident::new(method_name, Span::call_site());
        if args.async_lifecycle {
            quote! {
                async fn on_start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    self.#method_ident().await
                }
            }
        } else {
            quote! {
                async fn on_start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    self.#method_ident()
                }
            }
        }
    } else {
        quote! {
            async fn on_start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                // 默认启动实现
                Ok(())
            }
        }
    };

    let stop_method = if let Some(method_name) = &args.on_stop {
        let method_ident = Ident::new(method_name, Span::call_site());
        if args.async_lifecycle {
            quote! {
                async fn on_stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    self.#method_ident().await
                }
            }
        } else {
            quote! {
                async fn on_stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    self.#method_ident()
                }
            }
        }
    } else if let Some(method_name) = &args.cleanup {
        let method_ident = Ident::new(method_name, Span::call_site());
        if args.async_lifecycle {
            quote! {
                async fn on_stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    self.#method_ident().await
                }
            }
        } else {
            quote! {
                async fn on_stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    self.#method_ident()
                }
            }
        }
    } else {
        quote! {
            async fn on_stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                // 默认停止实现
                Ok(())
            }
        }
    };

    quote! {
        #[async_trait::async_trait]
        impl infrastructure_common::Lifecycle for #struct_name {
            #start_method

            #stop_method

            fn get_lifecycle_state(&self) -> infrastructure_common::LifecycleState {
                // 默认实现，可以在具体组件中覆盖
                infrastructure_common::LifecycleState::Running
            }
        }
    }
}

/// 生成依赖管理实现
fn generate_dependency_impl(struct_name: &Ident, args: &LifecycleArgs) -> proc_macro2::TokenStream {
    if args.depends_on.is_empty() {
        return quote! {};
    }

    let dependencies: Vec<proc_macro2::TokenStream> = args
        .depends_on
        .iter()
        .map(|dep| {
            quote! { #dep.to_string() }
        })
        .collect();

    quote! {
        impl infrastructure_common::DependencyAware for #struct_name {
            fn get_dependencies(&self) -> Vec<String> {
                vec![#(#dependencies),*]
            }

            fn can_start_without_dependencies(&self) -> bool {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_args_defaults() {
        let args = LifecycleArgs::default();

        assert_eq!(args.on_start, None);
        assert_eq!(args.on_stop, None);
        assert!(args.depends_on.is_empty());
        assert!(!args.async_lifecycle);
    }
}
